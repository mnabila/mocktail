//! Request-log interface slice: paginated admin reads.

use crate::http::auth::gate;
use crate::shared::extract::AxumRequest;
use crate::shared::json::headers_json;
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::request_log::application::{LogCursor, LogQuery};
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog};
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::{CollectionId, WorkspaceId};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn get_logs_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(get_logs(&req, &ctx, &ws_id).await)
}

async fn get_logs(req: &Request, ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    if let Some(rid) = req.query_first("requestId") {
        let found = ctx
            .request_logs
            .find_by_request_id(&ws, &RequestId(rid.to_string()))
            .await?;
        return Ok(Response::api_json(
            200,
            &json!({ "items": found.iter().map(log_json).collect::<Vec<_>>() }),
        ));
    }
    let coll = req.query_first("collectionId").map(CollectionId::new);
    let limit = req
        .query_first("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20)
        .clamp(1, 200);
    let path = req
        .query_first("path")
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let status = req
        .query_first("status")
        .and_then(|s| s.parse::<u16>().ok());
    // statusClass is a bucket like "2xx"/"4xx"; translate to its lower bound.
    let status_class = req.query_first("statusClass").and_then(|s| match s {
        "2xx" => Some(200),
        "3xx" => Some(300),
        "4xx" => Some(400),
        "5xx" => Some(500),
        _ => None,
    });
    // Cursor format: "<receivedAt>_<id>" (receivedAt is digits, id follows the
    // first underscore).
    let cursor = req.query_first("cursor").and_then(parse_cursor);

    // Fetch one extra row to tell whether another page exists.
    let q = LogQuery {
        coll: coll.as_ref(),
        path_contains: path,
        status,
        status_class,
        cursor,
        limit: limit + 1,
    };
    let mut rows = ctx.request_logs.page(&ws, &q).await?;
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let next_cursor = if has_more {
        rows.last()
            .map(|l| json!(format!("{}_{}", l.received_at, l.id.as_str())))
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let items: Vec<Value> = rows.iter().map(log_json).collect();
    Ok(Response::api_json(
        200,
        &json!({ "items": items, "nextCursor": next_cursor }),
    ))
}

/// Parse a keyset cursor of the form "<receivedAt>_<id>".
fn parse_cursor(raw: &str) -> Option<LogCursor> {
    let (ts, id) = raw.split_once('_')?;
    let received_at = ts.parse::<i64>().ok()?;
    if id.is_empty() {
        return None;
    }
    Some(LogCursor {
        received_at,
        id: id.to_string(),
    })
}

fn log_json(l: &RequestLog) -> Value {
    json!({
        "id": l.id.as_str(),
        "requestId": l.request_id.as_str(),
        "workspaceId": l.workspace_id.as_str(),
        "collectionId": l.collection_id.as_ref().map(|c| c.as_str()),
        "receivedAt": l.received_at,
        "method": l.method,
        "path": l.path,
        "requestHeaders": headers_json(&l.request_headers),
        "requestHeaderPairs": l.request_headers.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        "requestBody": l.request_body,
        "matchedMockId": l.matched_mock_id.as_ref().map(|m| m.as_str()),
        "matchReason": l.match_reason,
        "status": l.status,
        "response": {
            "status": l.response.status,
            "headers": headers_json(&l.response.headers),
            "headerPairs": l.response.headers.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
            "body": l.response.body,
        },
        "ip": l.ip,
        "country": l.country,
        "lat": l.lat,
        "lng": l.lng,
    })
}
