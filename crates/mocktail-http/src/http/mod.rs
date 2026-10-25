//! HTTP interface slices, one module per bounded context. Each owns its axum
//! handlers, JSON mapping, and request parsing; the route table lives in
//! `routes.rs`.

pub(crate) mod auth;
pub(crate) mod collection;
pub(crate) mod folder;
pub(crate) mod mock;
pub mod mock_runtime;
pub(crate) mod openapi;
pub(crate) mod request_log;
pub(crate) mod routes;
pub(crate) mod util;
pub(crate) mod workspace;
