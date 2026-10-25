//! OpenAPI use cases: parse a document, preview an import against the existing
//! collection, commit it (reusing ids by key for round-trip), and export.

use crate::modules::openapi::domain::normalize_swagger2;
use crate::shared::error::{AppError, AppResult};
use serde_json::Value;

/// Parse an uploaded OpenAPI document: JSON first, YAML as a fallback.
pub fn parse_spec(bytes: &[u8]) -> AppResult<Value> {
    if let Ok(v) = serde_json::from_slice::<Value>(bytes) {
        return validate_openapi(v);
    }
    match serde_yaml::from_slice::<Value>(bytes) {
        Ok(v) => validate_openapi(v),
        Err(e) => Err(AppError::BadRequest(format!(
            "document is neither valid JSON nor YAML: {e}"
        ))),
    }
}

fn validate_openapi(v: Value) -> AppResult<Value> {
    if !v.is_object() {
        return Err(AppError::BadRequest(
            "OpenAPI document must be an object".into(),
        ));
    }
    // Accept OpenAPI 3.x (`openapi`) and Swagger 2.x (`swagger`). Swagger 2.x
    // documents are normalized to the 3.x response shape used by the importer.
    let swagger2 = match (
        v.get("openapi").and_then(Value::as_str),
        v.get("swagger").and_then(Value::as_str),
    ) {
        (Some(_), _) => false,
        (None, Some(version)) if version.starts_with("2.") => true,
        (None, Some(version)) => {
            return Err(AppError::BadRequest(format!(
                "unsupported Swagger version `{version}` (2.x expected)"
            )));
        }
        (None, None) => {
            return Err(AppError::BadRequest(
                "missing `openapi` (3.x) or `swagger` (2.x) version field".into(),
            ));
        }
    };
    if v.get("paths").and_then(Value::as_object).is_none() {
        return Err(AppError::BadRequest(
            "missing or invalid `paths` object".into(),
        ));
    }
    Ok(if swagger2 { normalize_swagger2(v) } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_swagger2_documents() {
        let bytes = br#"{
            "swagger": "2.0",
            "info": {"title": "Pets", "version": "1"},
            "paths": {"/pets": {"get": {"responses": {"200": {"description": "ok"}}}}}
        }"#;
        let doc = parse_spec(bytes).unwrap();
        assert_eq!(
            doc["paths"]["/pets"]["get"]["responses"]["200"]["description"],
            "ok"
        );
    }

    #[test]
    fn rejects_missing_or_unsupported_versions() {
        assert!(parse_spec(br#"{"paths": {}}"#).is_err());
        assert!(parse_spec(br#"{"swagger": "1.2", "paths": {}}"#).is_err());
    }
}
