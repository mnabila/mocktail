//! Cross-feature HTTP plumbing: host classification, axum/core conversion,
//! response envelopes, JSON helpers, and CORS.

pub mod cors;
pub mod extract;
pub mod hosts;
pub(crate) mod json;
pub(crate) mod response;
