//! Native runtime ports: system clock, id generation, outbound HTTP, and the
//! session signer. The only place the native binary
//! touches wall-clock, RNG, the network, or key material.

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use mocktail_core::shared::ports::{
    Clock, HttpClient, HttpResponse, IdGenerator, RateLimitDecision, RateLimiter, Signer, Timer,
    UnixMillis,
};
use mocktail_core::shared::rate_limit::FixedWindowState;
use sha2::Sha256;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> UnixMillis {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

pub struct TokioTimer;

#[async_trait]
impl Timer for TokioTimer {
    async fn sleep_ms(&self, ms: u32) {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
}

/// Native rate-limiter adapter. `Ctx` is shared across axum tasks, so the
/// fixed-window state needs a lock.
#[derive(Default)]
pub struct LocalRateLimiter {
    state: Mutex<FixedWindowState>,
}

impl RateLimiter for LocalRateLimiter {
    fn check(&self, key: &str, limit: u32, window_secs: u32, now: UnixMillis) -> RateLimitDecision {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .check(key, limit, window_secs, now)
    }
}

pub struct UuidIdGenerator;

impl IdGenerator for UuidIdGenerator {
    fn new_object_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn new_request_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

// Removed mongo_object_id_hex helper as we now use full UUIDs.

/// HMAC-SHA256 signer for session + OAuth-state tokens.
pub struct HmacSigner {
    secret: Vec<u8>,
}

impl HmacSigner {
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        HmacSigner {
            secret: secret.into(),
        }
    }
}

impl Signer for HmacSigner {
    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(bytes);
        mac.finalize().into_bytes().to_vec()
    }
}

/// Outbound HTTP via reqwest (native OAuth flow).
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        ReqwestHttpClient {
            // A timeout bounds the forward/proxy mode: a slow upstream must not
            // pin a Tokio worker indefinitely.
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Send a prepared request and collect status + headers + body.
    async fn finish(req: reqwest::RequestBuilder) -> Result<HttpResponse, String> {
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn with_headers(
    mut req: reqwest::RequestBuilder,
    headers: &[(String, String)],
) -> reqwest::RequestBuilder {
    for (k, v) in headers {
        req = req.header(k, v);
    }
    req
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Self::finish(with_headers(self.client.post(url).form(form), headers)).await
    }

    async fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, String> {
        Self::finish(with_headers(self.client.get(url), headers)).await
    }

    async fn post_json(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Self::finish(with_headers(
            self.client.post(url).body(body.to_vec()),
            headers,
        ))
        .await
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
        Self::finish(with_headers(
            self.client.request(m, url).body(body.to_vec()),
            headers,
        ))
        .await
    }
}
