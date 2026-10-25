//! Matching use cases: resolve a mock and render its templated response.

pub mod render;
pub mod resolve;

pub use render::render_body;
pub use resolve::{Resolved, resolve_mock};
