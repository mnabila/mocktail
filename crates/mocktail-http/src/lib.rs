//! Shared axum interface for both adapters.
//!
//! Native serves this router with `axum::serve`; Cloudflare Workers call it as
//! a service through `worker`'s axum integration. Admin/auth routes are static
//! axum routes; the host-first mock runtime is the fallback.
//!
//! This crate is the inbound HTTP adapter. Handlers live under `http/`,
//! grouped by bounded context; cross-feature HTTP plumbing lives under
//! `http/util/`. The route table and host dispatch live in `http/routes.rs`.
//! This module is the public facade.

mod http;
mod shared;

pub use http::mock_runtime as handle;
pub use shared::{cors, extract, hosts as router};

use axum::Router;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::shared::http::{Request as CoreRequest, Response as CoreResponse};
use std::sync::Arc;

/// Build the shared application router.
pub fn router(ctx: Arc<Ctx>) -> Router {
    http::routes::router(ctx)
}

/// Test/back-compat entry point. Runs the real router as a one-shot service.
pub async fn handle(req: CoreRequest, ctx: &Ctx) -> CoreResponse {
    http::routes::handle(req, ctx).await
}
