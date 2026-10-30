//! Mock interface slice: mock CRUD and JSON definition parsing/mapping.

use crate::http::auth::gate;
use crate::http::folder::ensure_folder;
use crate::shared::extract::AxumRequest;
use crate::shared::json::{headers_json, json_body, str_field};
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::collection::application as coll_app;
use mocktail_core::modules::mock::application as mock_app;
use mocktail_core::modules::mock::domain::{
    Fault, MatchKind, MatchTarget, Mock, Priority, RateLimit, RequestMatcher, ResponseSpec,
};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Method, Request, Response};
use mocktail_core::shared::ids::{CollectionId, FolderId, MockId, WorkspaceId};
use mocktail_core::shared::ports::UnixMillis;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn list_mocks_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(list_mocks(&ctx, &id).await)
}

pub(crate) async fn create_mock_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(create_mock(&req, &ctx, &id).await)
}

pub(crate) async fn get_mock_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(get_mock(&ctx, &id).await)
}

pub(crate) async fn put_mock_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(put_mock(&req, &ctx, &id).await)
}

pub(crate) async fn patch_mock_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(patch_mock(&req, &ctx, &id).await)
}

pub(crate) async fn delete_mock_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(delete_mock(&ctx, &id).await)
}

async fn list_mocks(ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let items: Vec<Value> = ctx
        .mocks
        .list_active(&coll.workspace_id, &coll.id)
        .await?
        .iter()
        .map(mock_json)
        .collect();
    Ok(Response::api_json(200, &json!({ "items": items })))
}

async fn create_mock(req: &Request, ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let body = json_body(req)?;
    let now = ctx.clock.now_millis();
    let mock = parse_mock(
        &body,
        MockId::new(ctx.ids.new_object_id()),
        coll.workspace_id.clone(),
        coll.id.clone(),
        now,
        now,
    )?;
    ensure_folder(ctx, &coll.id, mock.folder_id.as_ref()).await?;
    mock_app::save_mock(ctx.mocks.as_ref(), &mock).await?;
    Ok(Response::api_json(201, &mock_json(&mock)))
}

async fn get_mock(ctx: &Ctx, id: &str) -> AppResult<Response> {
    let mock = mock_app::get_mock(ctx.mocks.as_ref(), &MockId::new(id)).await?;
    Ok(Response::api_json(200, &mock_json(&mock)))
}

async fn put_mock(req: &Request, ctx: &Ctx, id: &str) -> AppResult<Response> {
    let existing = mock_app::get_mock(ctx.mocks.as_ref(), &MockId::new(id)).await?;
    let body = json_body(req)?;
    let mock = parse_mock(
        &body,
        existing.id.clone(),
        existing.workspace_id.clone(),
        existing.collection_id.clone(),
        existing.created_at,
        ctx.clock.now_millis(),
    )?;
    ensure_folder(ctx, &existing.collection_id, mock.folder_id.as_ref()).await?;
    mock_app::save_mock(ctx.mocks.as_ref(), &mock).await?;
    Ok(Response::api_json(200, &mock_json(&mock)))
}

/// Move a mock between folders without touching the rest of its definition.
async fn patch_mock(req: &Request, ctx: &Ctx, id: &str) -> AppResult<Response> {
    let mut mock = mock_app::get_mock(ctx.mocks.as_ref(), &MockId::new(id)).await?;
    let body = json_body(req)?;
    let folder_id = match body.get("folderId") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let value = value.trim();
            (!value.is_empty()).then(|| FolderId::new(value))
        }
        Some(_) => {
            return Err(AppError::BadRequest(
                "folderId must be a string or null".into(),
            ));
        }
        None => {
            return Err(AppError::BadRequest("folderId is required".into()));
        }
    };
    ensure_folder(ctx, &mock.collection_id, folder_id.as_ref()).await?;
    mock.folder_id = folder_id;
    mock.updated_at = ctx.clock.now_millis();
    mock_app::save_mock(ctx.mocks.as_ref(), &mock).await?;
    Ok(Response::api_json(200, &mock_json(&mock)))
}

async fn delete_mock(ctx: &Ctx, id: &str) -> AppResult<Response> {
    mock_app::delete_mock(ctx.mocks.as_ref(), &MockId::new(id)).await?;
    Ok(Response::new(204))
}

fn mock_json(m: &Mock) -> Value {
    let content_type = m
        .response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| "application/json".to_string());
    let headers: Vec<(String, String)> = m
        .response
        .headers
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("content-type"))
        .cloned()
        .collect();
    json!({
        "id": m.id.as_str(),
        "workspaceId": m.workspace_id.as_str(),
        "collectionId": m.collection_id.as_str(),
        "folderId": m.folder_id.as_ref().map(|id| id.as_str()),
        "method": m.method.as_str(),
        "pathTemplate": m.path_template,
        "matchers": m.matchers.iter().map(matcher_json).collect::<Vec<_>>(),
        "response": {
            "status": m.response.status,
            "headers": headers_json(&headers),
            "body": String::from_utf8_lossy(&m.response.body),
            "contentType": content_type,
        },
        "priority": m.priority.0,
        "delayMs": m.delay_ms,
        "fault": m.fault.map(|f| match f {
            Fault::ConnectionReset => "connectionReset",
            Fault::MalformedResponse => "malformedResponse",
        }),
        "rateLimit": m.rate_limit.map(|rl| json!({
            "limit": rl.limit,
            "windowSecs": rl.window_secs,
            "status": rl.status,
        })),
        "createdAt": m.created_at,
        "updatedAt": m.updated_at,
    })
}

fn matcher_json(m: &RequestMatcher) -> Value {
    let target = match &m.target {
        MatchTarget::Query(n) => json!({"type": "query", "name": n}),
        MatchTarget::Header(n) => json!({"type": "header", "name": n}),
        MatchTarget::Body => json!({"type": "body"}),
    };
    let kind = match &m.kind {
        MatchKind::Exact(v) => json!({"type": "exact", "value": v}),
        MatchKind::Contains(v) => json!({"type": "contains", "value": v}),
        MatchKind::Regex(v) => json!({"type": "regex", "value": v}),
        MatchKind::JsonPath { path, equals } => {
            json!({"type": "jsonpath", "path": path, "equals": equals})
        }
    };
    json!({ "target": target, "kind": kind })
}

fn parse_mock(
    body: &Value,
    id: MockId,
    ws: WorkspaceId,
    coll: CollectionId,
    created_at: UnixMillis,
    updated_at: UnixMillis,
) -> AppResult<Mock> {
    let method_str = str_field(body, "method")?;
    let method = Method::parse(&method_str)
        .ok_or_else(|| AppError::BadRequest(format!("unknown method {method_str:?}")))?;
    let path_template = str_field(body, "pathTemplate")?;

    let matchers = match body.get("matchers") {
        Some(Value::Array(arr)) => arr
            .iter()
            .map(parse_matcher)
            .collect::<AppResult<Vec<_>>>()?,
        Some(Value::Null) | None => Vec::new(),
        _ => return Err(AppError::BadRequest("matchers must be an array".into())),
    };

    let response = parse_response(
        body.get("response")
            .ok_or_else(|| AppError::BadRequest("response is required".into()))?,
    )?;

    let priority = Priority(body.get("priority").and_then(Value::as_i64).unwrap_or(0) as i32);
    // Clamp to 5 minutes: a configured delay pins a connection on the public
    // mock surface, so an unbounded value is a stored DoS.
    let delay_ms = body.get("delayMs").and_then(Value::as_u64).unwrap_or(0).min(300_000) as u32;
    let fault = match body.get("fault").and_then(Value::as_str) {
        Some("connectionReset") => Some(Fault::ConnectionReset),
        Some("malformedResponse") => Some(Fault::MalformedResponse),
        None => None,
        Some(other) => return Err(AppError::BadRequest(format!("unknown fault {other:?}"))),
    };
    let folder_id = body
        .get("folderId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(FolderId::new);
    let rate_limit = parse_rate_limit(body.get("rateLimit"))?;

    Ok(Mock {
        id,
        workspace_id: ws,
        collection_id: coll,
        folder_id,
        method,
        path_template,
        matchers,
        response,
        priority,
        delay_ms,
        fault,
        rate_limit,
        created_at,
        updated_at,
    })
}

fn parse_matcher(v: &Value) -> AppResult<RequestMatcher> {
    let target_v = v
        .get("target")
        .ok_or_else(|| AppError::BadRequest("matcher.target is required".into()))?;
    let target = match target_v.get("type").and_then(Value::as_str) {
        Some("query") => MatchTarget::Query(str_field(target_v, "name")?),
        Some("header") => MatchTarget::Header(str_field(target_v, "name")?),
        Some("body") => MatchTarget::Body,
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown matcher target type {other:?}"
            )));
        }
    };
    let kind_v = v
        .get("kind")
        .ok_or_else(|| AppError::BadRequest("matcher.kind is required".into()))?;
    let kind = match kind_v.get("type").and_then(Value::as_str) {
        Some("exact") => MatchKind::Exact(str_field(kind_v, "value")?),
        Some("contains") => MatchKind::Contains(str_field(kind_v, "value")?),
        Some("regex") => MatchKind::Regex(str_field(kind_v, "value")?),
        Some("jsonpath") => MatchKind::JsonPath {
            path: str_field(kind_v, "path")?,
            equals: kind_v
                .get("equals")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown matcher kind type {other:?}"
            )));
        }
    };
    Ok(RequestMatcher { target, kind })
}

fn parse_response(v: &Value) -> AppResult<ResponseSpec> {
    let status = v
        .get("status")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::BadRequest("response.status is required".into()))?;
    if !(100..=599).contains(&status) {
        return Err(AppError::BadRequest(
            "response.status must be 100-599".into(),
        ));
    }
    let status = status as u16;
    let mut headers = parse_headers(v.get("headers"))?;
    if let Some(content_type) = v
        .get("contentType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-type"));
        headers.push(("content-type".to_string(), content_type.to_string()));
    }
    let body = v
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .as_bytes()
        .to_vec();
    Ok(ResponseSpec {
        status,
        headers,
        body,
    })
}

/// Accept the three header shapes the portal/imports use: a name->value
/// object, an array of `{name, value}` objects, or legacy `[name, value]` pairs.
fn parse_headers(v: Option<&Value>) -> AppResult<Vec<(String, String)>> {
    let mut headers = Vec::new();
    match v {
        None | Some(Value::Null) => {}
        Some(Value::Object(map)) => {
            for (name, value) in map {
                if name.trim().is_empty() {
                    continue;
                }
                let value = match value {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                };
                headers.push((name.clone(), value));
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                let (name, value) = match item {
                    Value::Array(pair) => {
                        let name = pair.first().and_then(Value::as_str).ok_or_else(|| {
                            AppError::BadRequest("header pair needs a string name".into())
                        })?;
                        let value = pair.get(1).and_then(Value::as_str).ok_or_else(|| {
                            AppError::BadRequest("header pair needs a string value".into())
                        })?;
                        (name, value)
                    }
                    Value::Object(obj) => {
                        let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| {
                            AppError::BadRequest("header needs a string name".into())
                        })?;
                        let value = match obj.get("value") {
                            Some(Value::String(s)) => s.as_str(),
                            Some(Value::Null) | None => "",
                            _ => {
                                return Err(AppError::BadRequest(
                                    "header value must be a string".into(),
                                ));
                            }
                        };
                        (name, value)
                    }
                    _ => {
                        return Err(AppError::BadRequest(
                            "headers must be objects or [name, value] pairs".into(),
                        ));
                    }
                };
                if !name.trim().is_empty() {
                    headers.push((name.to_string(), value.to_string()));
                }
            }
        }
        Some(_) => {
            return Err(AppError::BadRequest(
                "headers must be an object or an array".into(),
            ));
        }
    }
    Ok(headers)
}

fn parse_rate_limit(v: Option<&Value>) -> AppResult<Option<RateLimit>> {
    let Some(v) = v else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let Some(obj) = v.as_object() else {
        return Err(AppError::BadRequest("rateLimit must be an object".into()));
    };
    // Legacy documents carried scope/key; they are ignored.
    let limit = obj.get("limit").and_then(Value::as_u64).unwrap_or(0);
    if limit == 0 {
        return Ok(None);
    }
    let limit = u32::try_from(limit)
        .map_err(|_| AppError::BadRequest("rateLimit.limit is too large".into()))?;
    let window_secs = u32::try_from(obj.get("windowSecs").and_then(Value::as_u64).unwrap_or(60))
        .map_err(|_| AppError::BadRequest("rateLimit.windowSecs is too large".into()))?;
    let status = u32::try_from(obj.get("status").and_then(Value::as_u64).unwrap_or(429))
        .ok()
        .and_then(|s| u16::try_from(s).ok())
        .ok_or_else(|| AppError::BadRequest("rateLimit.status is invalid".into()))?;
    Ok(Some(RateLimit {
        limit,
        window_secs,
        status,
    }))
}
