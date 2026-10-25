//! mocktail-core: runtime-agnostic domain + application layers.
//!
//! This crate is pure. It never names tokio, axum, workers-rs, or the SQLite
//! driver, and it compiles to both native and `wasm32-unknown-unknown`.
//! All outward needs are expressed as ports (traits) in the
//! inner layers and implemented by outer crates.

pub mod app;
pub mod modules;
pub mod shared;

// Re-export the context for composition roots and the HTTP interface crate.
pub use app::ctx::Ctx;
