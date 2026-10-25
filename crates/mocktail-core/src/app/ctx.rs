//! `Ctx` holds the resolved ports injected by the composition root.
//! Shared across requests; ports are `Send + Sync` so axum handlers on native
//! and Workers can move the context across await points.

use crate::modules::auth::application::{MembershipRepository, UserRepository};
use crate::modules::auth::domain::OAuthProvider;
use crate::modules::collection::application::CollectionRepository;
use crate::modules::folder::application::FolderRepository;
use crate::modules::mock::application::MockRepository;
use crate::modules::request_log::application::RequestLogRepository;
use crate::modules::workspace::application::WorkspaceRepository;
use crate::shared::ports::{Clock, HttpClient, IdGenerator, RateLimiter, Signer, Timer};
use std::collections::HashMap;
use std::sync::Arc;

/// Auth configuration resolved from operator config.
pub struct AuthConfig {
    /// Enabled OAuth providers, keyed by name (`google`/`github`/`gitlab`).
    pub providers: HashMap<String, OAuthProvider>,
    /// Public base URL used to derive OAuth redirect URIs, e.g.
    /// `https://mocktail.com`.
    pub public_base_url: String,
    /// Session lifetime in milliseconds.
    pub session_ttl_ms: i64,
    /// Emit the `Secure` cookie attribute (disable for plain-HTTP local dev).
    pub secure_cookies: bool,
}

#[derive(Clone)]
pub struct Ctx {
    /// The apex/base domain reserved for the admin surface.
    pub apex_host: String,
    pub workspaces: Arc<dyn WorkspaceRepository>,
    pub collections: Arc<dyn CollectionRepository>,
    pub folders: Arc<dyn FolderRepository>,
    pub mocks: Arc<dyn MockRepository>,
    pub request_logs: Arc<dyn RequestLogRepository>,
    pub users: Arc<dyn UserRepository>,
    pub memberships: Arc<dyn MembershipRepository>,
    pub rate_limiter: Arc<dyn RateLimiter>,
    pub clock: Arc<dyn Clock>,
    pub ids: Arc<dyn IdGenerator>,
    pub http: Arc<dyn HttpClient>,
    pub signer: Arc<dyn Signer>,
    pub timer: Arc<dyn Timer>,
    pub auth: Arc<AuthConfig>,
}
