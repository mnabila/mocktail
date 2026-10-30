//! Lightweight OTLP/HTTP exporter for Cloudflare Workers.
//!
//! Since Workers cannot use gRPC and are ephemeral, we use simple HTTP POSTs.

use serde::{Serialize};
use worker::*;

#[derive(Serialize)]
struct OTLPRequest {
    resourceSpans: Vec<ResourceSpan>,
}

#[derive(Serialize)]
struct ResourceSpan {
    resource: Resource,
    scopeSpans: Vec<ScopeSpan>,
}

#[derive(Serialize)]
struct Resource {
    attributes: Vec<KeyValue>,
}

#[derive(Serialize)]
struct KeyValue {
    key: String,
    value: Value,
}

#[derive(Serialize)]
struct Value {
    stringValue: String,
}

#[derive(Serialize)]
struct ScopeSpan {
    spans: Vec<Span>,
}

#[derive(Serialize)]
struct Span {
    trace_id: String,
    span_id: String,
    name: String,
    kind: i32,
    start_time_unix_nano: u64,
    end_time_unix_nano: u64,
    attributes: Vec<KeyValue>,
}

pub struct OtelWorker {
    endpoint: String,
    environment: String,
}

impl OtelWorker {
    pub fn new(endpoint: String, environment: String) -> Self {
        Self { endpoint, environment }
    }

    /// Pushes a simplified trace to the Alloy collector.
    pub async fn push_trace(&self, name: &str, duration_ms: u64) {
        let now = js_sys::Date::now() as u64 * 1_000_000;
        
        let request = OTLPRequest {
            resourceSpans: vec![ResourceSpan {
                resource: Resource {
                    attributes: vec![
                        KeyValue { key: "service.name".into(), value: Value { stringValue: "mocktail-worker".into() } },
                        KeyValue { key: "deployment.environment".into(), value: Value { stringValue: self.environment.clone() } },
                    ],
                },
                scopeSpans: vec![ScopeSpan {
                    spans: vec![Span {
                        trace_id: uuid::Uuid::new_v4().to_string().replace("-", ""),
                        span_id: uuid::Uuid::new_v4().to_string().replace("-", ""),
                        name: name.into(),
                        kind: 1, // Internal
                        start_time_unix_nano: now - (duration_ms * 1_000_000),
                        end_time_unix_nano: now,
                        attributes: vec![],
                    }],
                }],
            }],
        };

        let body = serde_json::to_string(&request).unwrap_or_default();
        
        // Use worker::Fetch to push the trace to Alloy.
        let headers = Headers::new();
        let _ = headers.set("Content-Type", "application/json");
        let init = RequestInit {
            method: Method::Post,
            body: Some(body.into()),
            headers,
            ..Default::default()
        };
        if let Ok(req) = Request::new_with_init(&self.endpoint, &init) {
            let _ = Fetch::Request(req).send().await;
        }
    }
}
