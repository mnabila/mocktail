//! Dynamic mock runtime: workspace/collection resolution and mock response
//! construction. Static admin/auth routes live in the other feature modules;
//! this is the fallback used for the host-first mock surface.

use crate::shared::{cors, hosts};
use crate::http::util::{resolve_request_id, REQUEST_ID_HEADER};
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::collection::domain::Collection;
use mocktail_core::modules::matching::application::{Resolved, render_body, resolve_mock};
use mocktail_core::modules::mock::domain::RateLimit;
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog, ResponseLog};
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::http::{Method, Request, Response};
use mocktail_core::shared::ids::{Id, Slug, WorkspaceId};
use mocktail_core::shared::ports::RateLimitDecision;

/// Diagnostic response for a host that is neither the apex nor a workspace.
pub(crate) fn unknown_host(req: &Request, ctx: &Ctx) -> Response {
    let rid = resolve_request_id(req, ctx);
    diagnostic_404(
        &rid,
        &serde_json::json!({
            "error": "unknown host",
            "host": hosts::host_without_port(&req.host),
            "hint": "expected {workspace}.{apex} or the apex admin host",
        }),
        None,
        req,
    )
}

pub(crate) async fn mock_runtime(req: &Request, ctx: &Ctx, ws_slug: &str) -> Response {
    let rid = resolve_request_id(req, ctx);

    // Resolve workspace by subdomain label.
    let Ok(slug) = Slug::parse(ws_slug) else {
        return diagnostic_404(
            &rid,
            &serde_json::json!({"error": "invalid workspace slug", "slug": ws_slug}),
            None,
            req,
        );
    };
    let ws: Workspace = match ctx.workspaces.find_by_slug(&slug).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            tracing::warn!(rid = %rid, slug = %slug, "Workspace not found");
            return diagnostic_404(
                &rid,
                &serde_json::json!({"error": "unknown workspace", "slug": ws_slug}),
                None,
                req,
            );
        }
        Err(e) => {
            tracing::error!(rid = %rid, error = %e, "Error finding workspace");
            return internal(&rid, e)
        }
    };

    // First path segment selects the collection within the workspace.
    let coll_slug_raw = hosts::first_segment(&req.path);
    let coll: Collection = match Slug::parse(coll_slug_raw) {
        Ok(cslug) => match ctx.collections.find_by_slug(&ws.id, &cslug).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!(rid = %rid, workspace = %ws.id, collection = %cslug, "Collection not found");
                return unknown_collection(&rid, ctx, &ws, coll_slug_raw, req).await
            }
            Err(e) => {
                tracing::error!(rid = %rid, error = %e, "Error finding collection");
                return internal(&rid, e)
            }
        },
        Err(_) => {
            tracing::debug!(rid = %rid, slug_raw = %coll_slug_raw, "Invalid collection slug");
            return unknown_collection(&rid, ctx, &ws, coll_slug_raw, req).await
        }
    };

    // Strip the mount path; the remainder is spec-relative.
    let Some(spec_path) = coll.strip_mount(&req.path) else {
        return unknown_collection(&rid, ctx, &ws, coll_slug_raw, req).await;
    };
    let spec_path = spec_path.to_string();

    // Automatic CORS preflight (no explicit OPTIONS mock needed).
    if req.method == Method::Options {
        let mut resp = cors::preflight_response(&coll.cors_policy, req);
        resp.headers
            .insert(REQUEST_ID_HEADER, rid.as_str().to_string());
        log_request(ctx, &rid, &ws, Some(&coll), req, &resp, None, None).await;
        return resp;
    }

    // Resolve the winning mock and build its response.
    match resolve_mock(ctx.mocks.as_ref(), &ws.id, &coll.id, req, &spec_path).await {
        Ok(Some(resolved)) => {
            tracing::info!(
                rid = %rid,
                mock_id = %resolved.mock.id,
                reason = %resolved.reason,
                "Request matched mock"
            );
            // Enforce the endpoint's IP-keyed fixed-window limit before any
            // configured delay or response work.
            if let Some(limit) = resolved.mock.rate_limit {
                let client = req
                    .client_ip
                    .as_deref()
                    .map(str::trim)
                    .filter(|ip| !ip.is_empty())
                    .unwrap_or("unknown");
                let key = format!("{}:{}", resolved.mock.id, client);
                if let RateLimitDecision::Deny { retry_after_secs } = ctx.rate_limiter.check(
                    &key,
                    limit.limit,
                    limit.window_secs,
                    ctx.clock.now_millis(),
                ) {
                    tracing::info!(rid = %rid, client = %client, "Request rate-limited");
                    let reason = format!("rate-limited: {}", resolved.reason);
                    let resp =
                        rate_limited_response(&coll, &limit, retry_after_secs, &rid, req, &reason);
                    log_request(
                        ctx,
                        &rid,
                        &ws,
                        Some(&coll),
                        req,
                        &resp,
                        Some(resolved.mock.id.as_str().to_string()),
                        Some(reason),
                    )
                    .await;
                    return resp;
                }
            }

            // Enact a configured response delay before responding.
            if resolved.mock.delay_ms > 0 {
                ctx.timer.sleep_ms(resolved.mock.delay_ms).await;
            }
            let resp = build_mock_response(&coll, &resolved, req, &rid);
            log_request(
                ctx,
                &rid,
                &ws,
                Some(&coll),
                req,
                &resp,
                Some(resolved.mock.id.as_str().to_string()),
                Some(resolved.reason.clone()),
            )
            .await;
            resp
        }
        Ok(None) => {
            tracing::info!(rid = %rid, "No mock matched request");
            // If calling the base URL and no mock matches, return the collection description as raw text.
            if spec_path == "/" && !coll.description.is_empty() {
                let mut resp = Response::new(200)
                    .with_body(coll.description.as_bytes().to_vec())
                    .with_header("content-type", "text/plain; charset=utf-8");
                cors::apply_cors_headers(&mut resp, &coll.cors_policy, req.headers.get("origin"));
                resp.headers
                    .insert(REQUEST_ID_HEADER, rid.as_str().to_string());
                log_request(ctx, &rid, &ws, Some(&coll), req, &resp, None, None).await;
                return resp;
            }

            let near = near_misses(ctx, &ws.id, &coll).await;
            let mut resp = diagnostic_404(
                &rid,
                &serde_json::json!({
                    "error": "no matching mock",
                    "method": req.method.as_str(),
                    "path": spec_path,
                    "collection": coll.slug.as_str(),
                    "nearMisses": near,
                }),
                Some(&coll),
                req,
            );
            cors::apply_cors_headers(&mut resp, &coll.cors_policy, req.headers.get("origin"));
            log_request(ctx, &rid, &ws, Some(&coll), req, &resp, None, None).await;
            resp
        }
        Err(e) => internal(&rid, e),
    }
}

/// `ConnectionReset` is represented as a 502 carrying this marker. Neither
/// adapter can drop the socket from a shared axum response; Workers could not
/// do so before either.
pub const FAULT_HEADER: &str = "X-Mocktail-Fault";

fn build_mock_response(
    coll: &Collection,
    resolved: &Resolved,
    req: &Request,
    rid: &RequestId,
) -> Response {
    use mocktail_core::modules::mock::domain::Fault;

    let spec = &resolved.mock.response;
    let body = render_body(&spec.body, &resolved.params, req);
    let mut resp = Response::new(spec.status).with_body(body);
    for (k, v) in &spec.headers {
        resp.headers.insert(k.clone(), v.clone());
    }

    // Enact a configured fault.
    match resolved.mock.fault {
        Some(Fault::MalformedResponse) => {
            // Replace the body with truncated garbage to simulate corruption.
            resp = resp.with_body(b"\x00\x00<mocktail: malformed response fault>".to_vec());
            resp.headers.insert(FAULT_HEADER, "malformed-response");
        }
        Some(Fault::ConnectionReset) => {
            resp = Response::new(502).with_body(Vec::new());
            resp.headers.insert(FAULT_HEADER, "connection-reset");
        }
        None => {}
    }

    cors::apply_cors_headers(&mut resp, &coll.cors_policy, req.headers.get("origin"));
    resp.headers
        .insert(REQUEST_ID_HEADER, rid.as_str().to_string());
    resp.headers
        .insert("X-Mocktail-Match-Reason", resolved.reason.clone());
    resp
}

fn rate_limited_response(
    coll: &Collection,
    limit: &RateLimit,
    retry_after_secs: u64,
    rid: &RequestId,
    req: &Request,
    reason: &str,
) -> Response {
    let status = if (400..=599).contains(&limit.status) {
        limit.status
    } else {
        429
    };
    let mut resp = Response::json(
        status,
        &serde_json::json!({
            "error": {
                "message": "rate limit exceeded",
                "status": status,
                "limit": limit.limit,
                "windowSecs": limit.window_secs,
            },
            "retryAfter": retry_after_secs,
        }),
    );
    resp.headers
        .insert("Retry-After", retry_after_secs.to_string());
    cors::apply_cors_headers(&mut resp, &coll.cors_policy, req.headers.get("origin"));
    resp.headers
        .insert(REQUEST_ID_HEADER, rid.as_str().to_string());
    resp.headers
        .insert("X-Mocktail-Match-Reason", reason.to_string());
    resp
}

/// Collect the path templates in a collection to aid debugging a 404.
async fn near_misses(ctx: &Ctx, ws: &WorkspaceId, coll: &Collection) -> Vec<String> {
    match ctx.mocks.list_active(ws, &coll.id).await {
        Ok(mocks) => mocks
            .iter()
            .map(|m| format!("{} {}", m.method.as_str(), m.path_template))
            .collect(),
        Err(_) => Vec::new(),
    }
}

async fn unknown_collection(
    rid: &RequestId,
    ctx: &Ctx,
    ws: &Workspace,
    slug: &str,
    req: &Request,
) -> Response {
    let available = match ctx.collections.list(&ws.id).await {
        Ok(cs) => cs.iter().map(|c| c.slug.as_str().to_string()).collect(),
        Err(_) => Vec::new(),
    };
    diagnostic_404(
        rid,
        &serde_json::json!({
            "error": "unknown collection",
            "slug": slug,
            "workspace": ws.slug.as_str(),
            "availableCollections": available,
        }),
        None,
        req,
    )
}

fn diagnostic_404(
    rid: &RequestId,
    body: &serde_json::Value,
    _coll: Option<&Collection>,
    _req: &Request,
) -> Response {
    Response::json(404, body).with_header(REQUEST_ID_HEADER, rid.as_str().to_string())
}

fn internal(rid: &RequestId, e: mocktail_core::shared::error::AppError) -> Response {
    e.to_response()
        .with_header(REQUEST_ID_HEADER, rid.as_str().to_string())
}

/// Best-effort request log; a failure never affects the client response.
#[allow(clippy::too_many_arguments)]
async fn log_request(
    ctx: &Ctx,
    rid: &RequestId,
    ws: &Workspace,
    coll: Option<&Collection>,
    req: &Request,
    resp: &Response,
    matched_mock_id: Option<String>,
    reason: Option<String>,
) {
    let ip = req.client_ip.as_deref().map(|s| s.to_string());

    // We avoid blocking the request path with GeoIP resolution.
    // In a real system, we'd send this to a background queue.
    // For now, we resolve it simply if available.
    let (country, lat, lng) = if let Some(_ip_addr) = &ip {
        // Simple mock resolution for now; in production, this would hit a GeoIP DB.
        // We'll implement the actual resolver in the API layer or via a helper.
        (None, None, None)
    } else {
        (None, None, None)
    };

    let log = RequestLog {
        id: Id::new(ctx.ids.new_object_id()),
        request_id: rid.clone(),
        workspace_id: ws.id.clone(),
        collection_id: coll.map(|c| c.id.clone()),
        received_at: ctx.clock.now_millis(),
        method: req.method.as_str().to_string(),
        path: req.path.clone(),
        request_headers: req
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        request_body: String::from_utf8_lossy(&req.body).into_owned(),
        matched_mock_id: matched_mock_id.map(mocktail_core::shared::ids::MockId::new),
        match_reason: reason,
        status: resp.status,
        response: ResponseLog {
            status: resp.status,
            headers: resp
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: String::from_utf8_lossy(&resp.body).into_owned(),
        },
        ip,
        country,
        lat,
        lng,
    };
    if let Err(e) = mocktail_core::modules::request_log::application::record_request(
        ctx.request_logs.as_ref(),
        &log,
    )
    .await
    {
        tracing::warn!(
            request_id = rid.as_str(),
            error = %e,
            "request log write failed"
        );
    }
}
