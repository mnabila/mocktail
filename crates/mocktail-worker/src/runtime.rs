//! WASM runtime ports for the Workers target: clock from
//! the Workers `Date`, id/randomness from `uuid` (getrandom `js`), outbound
//! HTTP via the Workers `fetch`, and an HMAC-SHA256 signer.

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use mocktail_core::shared::ports::{
    Clock, HttpClient, HttpResponse, IdGenerator, RateLimitDecision, RateLimiter, Signer, Timer,
    UnixMillis,
};
use mocktail_core::shared::rate_limit::FixedWindowState;
use mocktail_infra_sqlite::{SqlExecutor, SqlValue};
use sha2::Sha256;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
use worker::{D1Database, Fetch, Headers, Method, RequestInit};

pub struct WorkerClock;
impl Clock for WorkerClock {
    fn now_millis(&self) -> UnixMillis {
        worker::Date::now().as_millis() as i64
    }
}

pub struct UuidIds;
impl IdGenerator for UuidIds {
    fn new_object_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn new_request_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

thread_local! {
    /// `Ctx` is rebuilt per request, but a Worker isolate reuses threads, so
    /// limiter state lives here to survive across requests in the isolate.
    static RATE_LIMITER: RefCell<FixedWindowState> = RefCell::new(FixedWindowState::default());
}

pub struct WorkerRateLimiter;

impl RateLimiter for WorkerRateLimiter {
    fn check(&self, key: &str, limit: u32, window_secs: u32, now: UnixMillis) -> RateLimitDecision {
        RATE_LIMITER.with(|state| state.borrow_mut().check(key, limit, window_secs, now))
    }
}

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

/// `SqlExecutor` over the Cloudflare D1 binding (the fully-Cloudflare store).
pub struct D1Exec {
    db: D1Database,
}

impl D1Exec {
    pub fn new(db: D1Database) -> Self {
        D1Exec { db }
    }
}

fn to_js(params: &[SqlValue]) -> Vec<JsValue> {
    params
        .iter()
        .map(|p| match p {
            SqlValue::Text(s) => JsValue::from_str(s),
            SqlValue::I64(n) => JsValue::from_f64(*n as f64),
            SqlValue::Bool(b) => JsValue::from_bool(*b),
            SqlValue::Real(f) => JsValue::from_f64(*f),
            SqlValue::Null => JsValue::NULL,
        })
        .collect()
}

#[async_trait]
impl SqlExecutor for D1Exec {
    async fn query(
        &self,
        sql: &str,
        params: Vec<SqlValue>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // D1 futures hold JS values, which are !Send. Workers is single-threaded,
        // so the `worker` adapter's SendFuture is the intended bridge.
        let sql_owned = sql.to_string();
        worker::send::SendFuture::new(async move {
            let stmt = self
                .db
                .prepare(&sql_owned)
                .bind(&to_js(&params))
                .map_err(|e| e.to_string())?;
            let result = stmt.all().await.map_err(|e| e.to_string())?;
            result
                .results::<serde_json::Value>()
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e: String| {
            worker::console_error!("D1 query failed: {e} | sql: {sql}");
            e
        })
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<(), String> {
        let sql_owned = sql.to_string();
        worker::send::SendFuture::new(async move {
            let stmt = self
                .db
                .prepare(&sql_owned)
                .bind(&to_js(&params))
                .map_err(|e| e.to_string())?;
            stmt.run().await.map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e: String| {
            worker::console_error!("D1 execute failed: {e} | sql: {sql}");
            e
        })
    }
}

/// Workers cannot block the request thread, so mock delays are a no-op here
/// (documented limitation; delays apply on the native target).
pub struct NoopTimer;

#[async_trait]
impl Timer for NoopTimer {
    async fn sleep_ms(&self, _ms: u32) {}
}

/// Outbound HTTP through the Workers runtime `fetch` (no raw TCP).
pub struct FetchHttpClient;

impl FetchHttpClient {
    async fn run(
        &self,
        method: Method,
        url: &str,
        body: Option<JsValue>,
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        let h = Headers::new();
        for (k, v) in headers {
            h.set(k, v).map_err(|e| e.to_string())?;
        }
        let mut init = RequestInit::new();
        init.with_method(method).with_headers(h);
        if let Some(b) = body {
            init.with_body(Some(b));
        }
        let req = worker::Request::new_with_init(url, &init).map_err(|e| e.to_string())?;
        let mut resp = Fetch::Request(req)
            .send()
            .await
            .map_err(|e| {
                worker::console_error!("outbound fetch failed: {e} | url: {url}");
                e.to_string()
            })?;
        let status = resp.status_code();
        let body = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(HttpResponse { status, body })
    }
}

#[async_trait]
impl HttpClient for FetchHttpClient {
    async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        // application/x-www-form-urlencoded body.
        let encoded = form
            .iter()
            .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let mut headers = headers.to_vec();
        headers.push((
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        ));
        worker::send::SendFuture::new(self.run(
            Method::Post,
            url,
            Some(JsValue::from_str(&encoded)),
            &headers,
        ))
        .await
    }

    async fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, String> {
        worker::send::SendFuture::new(self.run(Method::Get, url, None, headers)).await
    }

    async fn post_json(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        let text = String::from_utf8_lossy(body);
        worker::send::SendFuture::new(self.run(
            Method::Post,
            url,
            Some(JsValue::from_str(&text)),
            headers,
        ))
        .await
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
