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
use axum::routing::get;
use mocktail_core::app::ctx::Ctx;
use mocktail_http::extract::ClientIp;
use mocktail_http::router::host_without_port;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn serve(bind: &str, ctx: Arc<Ctx>, otel_handle: crate::otel::OtelHandle) -> Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, apex = %ctx.apex_host, "mocktail listening");

    let metrics_ctx = ctx.clone();
    let http_requests = otel_handle.http_requests;
    let app = mocktail_http::router(ctx.clone())
        .route(
            "/metrics",
            get(move |headers: axum::http::HeaderMap| metrics_handler(metrics_ctx.clone(), headers)),
        )
        .layer(axum::middleware::from_fn_with_state(ctx, native_layer))
        .layer(axum::middleware::from_fn(move |req: Request, next: Next| {
            let counter = http_requests.clone();
            async move {
                let method = req.method().to_string();
                let response = next.run(req).await;
                counter.add(
                    1,
                    &[
                        opentelemetry::KeyValue::new("http.method", method),
                        opentelemetry::KeyValue::new(
                            "http.status",
                            response.status().as_u16() as i64,
                        ),
                    ],
                );
                response
            }
        }));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("serving HTTP")
}

async fn metrics_handler(ctx: Arc<Ctx>, headers: axum::http::HeaderMap) -> Response {
    // /metrics sits outside the router's host guard, so classify here and only
    // serve it on the admin surface - never on a public workspace host.
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(host_without_port)
        .unwrap_or_default();
    if !matches!(
        mocktail_http::router::classify(&host, &ctx.apex_host),
        mocktail_http::router::Surface::Admin
    ) {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .expect("static response");
    }
    let body = crate::otel::get_metrics();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(Body::from(body))
        .unwrap()
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
