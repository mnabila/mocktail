//! JSON request helpers and header serialization shared by feature handlers.

use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::Request;
use mocktail_core::shared::ids::Slug;
use serde_json::Value;

/// Render ordered header pairs as a JSON object for the portal. Repeated names
/// are combined per HTTP semantics; the ordered pairs remain available on the
/// log payload for callers that need exact wire order.
pub(crate) fn headers_json(headers: &[(String, String)]) -> Value {
    let mut map = serde_json::Map::new();
    for (name, value) in headers {
        match map.get_mut(name) {
            Some(Value::String(existing)) => {
                existing.push_str(", ");
                existing.push_str(value);
            }
            _ => {
                map.insert(name.clone(), Value::String(value.clone()));
            }
        }
    }
    Value::Object(map)
}

pub(crate) fn json_body(req: &Request) -> AppResult<Value> {
    if req.body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&req.body)
        .map_err(|e| AppError::BadRequest(format!("invalid JSON body: {e}")))
}

pub(crate) fn str_field(v: &Value, key: &str) -> AppResult<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::BadRequest(format!("field {key:?} is required")))
}

pub(crate) fn parse_slug_or_derive(body: &Value, name: &str) -> AppResult<Slug> {
    match body.get("slug").and_then(Value::as_str) {
        Some(s) => Slug::parse(s).map_err(|e| AppError::Domain(e.into())),
        None => Slug::slugify(name).map_err(|e| AppError::Domain(e.into())),
    }
}
