//! Static admin/auth route table, host-first dispatch, and the one-shot
//! `handle` helper.

use super::{auth, collection, folder, mock, mock_runtime, openapi, request_log, workspace};
use crate::shared::extract::{AxumRequest, from_axum, to_axum, to_axum_request};
use crate::shared::hosts;
use axum::Router;
use axum::extract::{FromRequest, Request, State};
use axum::http::header::HOST;
use axum::middleware::{self, Next};
use axum::response::Response as AxumResponse;
use axum::routing::{delete, get, patch, post};
use mocktail_core::app::ctx::Ctx;
use mocktail_core::shared::error::AppError;
use mocktail_core::shared::http::{Request as CoreRequest, Response as CoreResponse};
use serde_json::json;
use std::sync::Arc;
use tracing::info_span;
use crate::http::util::resolve_rid_from_axum;

/// Build the shared application router.
pub(crate) fn router(ctx: Arc<Ctx>) -> Router {
    let app = Router::new()
        .route("/__admin/health", get(health))
        .route("/__admin/me", get(auth::me_route).patch(auth::update_user_route))

        .route(
            "/__admin/workspaces",
            get(workspace::list_workspaces_route).post(workspace::create_workspace_route),
        )
        .route(
            "/__admin/workspaces/{ws_id}",
            patch(workspace::patch_workspace_route).delete(workspace::delete_workspace_route),
        )
        .route(
            "/__admin/workspaces/{ws_id}/members",
            get(auth::list_members_route).post(auth::add_member_route),
        )
        .route(
            "/__admin/workspaces/{ws_id}/members/{mid}",
            patch(auth::update_member_route).delete(auth::remove_member_route),
        )
        .route(
            "/__admin/workspaces/{ws_id}/collections",
            get(collection::list_collections_route).post(collection::create_collection_route),
        )
        .route(
            "/__admin/workspaces/{ws_id}/logs",
            get(request_log::get_logs_route),
        )
        .route(
            "/__admin/collections/{id}",
            get(collection::get_collection_route)
                .patch(collection::patch_collection_route)
                .delete(collection::delete_collection_route),
        )
        .route(
            "/__admin/collections/{id}/folders",
            get(folder::list_folders_route).post(folder::create_folder_route),
        )
        .route(
            "/__admin/collections/{id}/folders/{fid}",
            delete(folder::delete_folder_route),
        )
        .route(
            "/__admin/collections/{id}/mocks",
            get(mock::list_mocks_route).post(mock::create_mock_route),
        )
        .route(
            "/__admin/collections/{id}/import/preview",
            post(openapi::import_preview_route),
        )
        .route(
            "/__admin/collections/{id}/import",
            post(openapi::import_commit_route),
        )
        .route(
            "/__admin/collections/{id}/export",
            get(openapi::export_route),
        )
        .route(
            "/__admin/mocks/{id}",
            get(mock::get_mock_route)
                .put(mock::put_mock_route)
                .patch(mock::patch_mock_route)
                .delete(mock::delete_mock_route),
        )
        .route("/auth/logout", post(auth::logout_route))
        .route("/auth/{provider}/login", get(auth::login_route))
        .route("/auth/{provider}/connect", get(auth::connect_route))
        .route("/auth/{provider}/callback", get(auth::callback_route))
        .fallback(apex_fallback)
        .with_state(ctx.clone());

    app.layer(middleware::from_fn_with_state(ctx.clone(), logging_middleware))
       .layer(middleware::from_fn_with_state(ctx, host_guard))
}

/// Test/back-compat entry point. Runs the real router as a one-shot service.
pub(crate) async fn handle(req: CoreRequest, ctx: &Ctx) -> CoreResponse {
    let app = router(Arc::new(ctx.clone()));
    let response = tower::ServiceExt::oneshot(app, to_axum_request(req))
        .await
        .unwrap_or_else(|_| AxumResponse::new(axum::body::Body::empty()));
    from_axum(response).await
}

async fn health() -> AxumResponse {
    to_axum(CoreResponse::api_json(200, &json!({"status": "ok"})))
}

async fn logging_middleware(State(ctx): State<Arc<Ctx>>, req: Request, next: Next) -> AxumResponse {
    let rid = resolve_rid_from_axum(&req, &ctx);
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let host = request_host(&req);

    let span = info_span!(
        "http.request",
        rid = %rid,
        method = %method,
        path = %path,
        host = %host
    );

    let _enter = span.enter();
    next.run(req).await
}

/// Host-first dispatch: protect the apex admin/auth routes from workspace
/// hosts, and send every non-apex request to the dynamic mock runtime.
async fn host_guard(State(ctx): State<Arc<Ctx>>, req: Request, next: Next) -> AxumResponse {
    let host = request_host(&req);
    let method = req.method().clone();
    let uri = req.uri().clone();

    match hosts::classify(&host, &ctx.apex_host) {
        hosts::Surface::Admin => {
            tracing::info!(method = %method, uri = %uri, host = %host, "Admin request");
            next.run(req).await
        }
        hosts::Surface::Workspace(ws_slug) => {
            let ws_slug = ws_slug.to_string();
            tracing::info!(method = %method, uri = %uri, host = %host, workspace = %ws_slug, "Workspace request");
            match AxumRequest::from_request(req, &ctx).await {
                Ok(AxumRequest(req)) => {
                    to_axum(mock_runtime::mock_runtime(&req, &ctx, &ws_slug).await)
                }
                Err(resp) => resp,
            }
        }
        hosts::Surface::Unknown => {
            tracing::warn!(method = %method, uri = %uri, host = %host, "Unknown host request");
            match AxumRequest::from_request(req, &ctx).await {
                Ok(AxumRequest(req)) => to_axum(mock_runtime::unknown_host(&req, &ctx)),
                Err(resp) => resp,
            }
        }
    }
}

/// Only the apex reaches the fallback (the host guard sends subdomains to the
/// mock runtime). Unknown `/auth/*` paths keep auth-shaped errors; everything
/// else is the admin API envelope.
async fn apex_fallback(
    State(_ctx): State<Arc<Ctx>>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let err = AppError::NotFound(format!("route {}", req.path));
    if req.path == "/auth" || req.path.starts_with("/auth/") {
        to_axum(err.to_response())
    } else {
        to_axum(err.to_admin_response())
    }
}

fn request_host(req: &Request) -> String {
    req.headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .map(hosts::host_without_port)
        .or_else(|| req.uri().host().map(hosts::host_without_port))
        .unwrap_or_default()
}
