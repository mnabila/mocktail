//! HTTP utilities shared across handlers and middleware.

use axum::extract::Request as AxumRequest;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::request_log::domain::RequestId;
use mocktail_core::shared::http::Request;

pub const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Honor a well-formed inbound `X-Request-Id`, else generate one (from core request).
pub fn resolve_request_id(req: &Request, ctx: &Ctx) -> RequestId {
    req.headers
        .get(REQUEST_ID_HEADER)
        .and_then(RequestId::accept_inbound)
        .unwrap_or_else(|| RequestId(ctx.ids.new_request_id()))
}

/// Honor a well-formed inbound `X-Request-Id`, else generate one (from axum request).
pub fn resolve_rid_from_axum(req: &AxumRequest, ctx: &Ctx) -> RequestId {
    req.headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(RequestId::accept_inbound)
        .unwrap_or_else(|| RequestId(ctx.ids.new_request_id()))
}
