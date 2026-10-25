//! Native axum adapter: serves the shared router and the embedded portal.
//!
//! The only native-specific concerns here are TCP binding, the SPA assets
//! fallback, and injecting the socket peer address for rate limiting.

use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use mocktail_core::app::ctx::Ctx;
use mocktail_http::extract::ClientIp;
use mocktail_http::router::host_without_port;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn serve(bind: &str, ctx: Arc<Ctx>) -> Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, apex = %ctx.apex_host, "mocktail listening");

    let app = mocktail_http::router(ctx.clone())
        .layer(axum::middleware::from_fn_with_state(ctx, native_layer));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("serving HTTP")
}

/// Add the peer IP, then serve the portal SPA on apex GETs before the router
/// sees the request.
async fn native_layer(State(ctx): State<Arc<Ctx>>, mut req: Request, next: Next) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string());
    if let Some(peer) = peer {
        req.extensions_mut().insert(ClientIp(peer));
    }

    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(host_without_port)
        .or_else(|| req.uri().host().map(host_without_port))
        .unwrap_or_default();
    let path = req.uri().path();
    let is_api = path.starts_with("/__admin") || path == "/auth" || path.starts_with("/auth/");
    let is_get = matches!(*req.method(), Method::GET | Method::HEAD);

    // Only serve the portal SPA if the host is classified as the Admin surface.
    // This explicitly prevents workspace subdomains from ever seeing the dashboard.
    let surface = mocktail_http::router::classify(&host, &ctx.apex_host);
    if matches!(surface, mocktail_http::router::Surface::Admin)
        && is_get
        && !is_api
        && let Some((bytes, content_type)) = crate::interfaces::assets::serve(path)
    {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(bytes))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .expect("static response")
            });
    }

    next.run(req).await
}
