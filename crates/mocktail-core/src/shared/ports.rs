//! Cross-cutting ports: supplied by the adapter, injected through `Ctx`.
//! Core never calls a clock or RNG directly.

/// Milliseconds since the Unix epoch. A plain integer keeps the domain free of
/// any date/time crate (which also keeps it wasm-clean).
pub type UnixMillis = i64;

/// Supplies the current time. Native reads the system clock; tests inject a
/// fixed value; WASM reads the request context.
pub trait Clock: Send + Sync {
    fn now_millis(&self) -> UnixMillis;
}

/// Supplies fresh ids. Native uses ObjectId/UUID; tests inject a counter.
pub trait IdGenerator: Send + Sync {
    /// A 24-char-hex ObjectId-compatible id for a new aggregate.
    fn new_object_id(&self) -> String;
    /// A request id (UUID v4) when the client sent none.
    fn new_request_id(&self) -> String;
}

/// Response from an outbound HTTP call made through the `HttpClient` port.
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Outbound HTTP, used by the OAuth flow to reach the identity provider.
/// Native uses reqwest; WASM wraps its JS-backed futures to satisfy axum's
/// `Send` handler requirement.
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String>;

    async fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, String>;

    /// POST a raw JSON body (e.g. OAuth token or userinfo calls).
    async fn post_json(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String>;
}

/// Keyed MAC for signing session + OAuth-state tokens. The
/// secret lives in config; core never holds raw key material, only this port.
pub trait Signer: Send + Sync {
    /// Produce a MAC (e.g. HMAC-SHA256) over `bytes`.
    fn sign(&self, bytes: &[u8]) -> Vec<u8>;
}

/// Suspends for a duration, used to enact mock response delays. Core
/// has no timer; native sleeps on the Tokio runtime, WASM is a no-op (Workers
/// cannot block the request), tests inject a no-op.
#[async_trait::async_trait]
pub trait Timer: Send + Sync {
    async fn sleep_ms(&self, ms: u32);
}

/// Outcome of a fixed-window rate-limit check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allow,
    Deny { retry_after_secs: u64 },
}

/// Best-effort per-process/isolate rate limiter for public mock endpoints.
/// The runtime supplies the wall clock; the implementation owns counter state.
pub trait RateLimiter: Send + Sync {
    fn check(&self, key: &str, limit: u32, window_secs: u32, now: UnixMillis) -> RateLimitDecision;
}
