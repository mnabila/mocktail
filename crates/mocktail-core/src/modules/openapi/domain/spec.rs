//! OpenAPI document metadata and shared operation constants.

use crate::shared::http::Method;
use serde_json::Value;

/// HTTP methods an OpenAPI path item may declare, in a stable order.
pub(super) const OPERATION_METHODS: &[(&str, Method)] = &[
    ("get", Method::Get),
    ("put", Method::Put),
    ("post", Method::Post),
    ("delete", Method::Delete),
    ("patch", Method::Patch),
    ("head", Method::Head),
    ("options", Method::Options),
    ("trace", Method::Trace),
];

/// Top-level metadata extracted from an OpenAPI / Swagger document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecInfo {
    pub title: Option<String>,
    pub version: Option<String>,
    pub base_path: Option<String>,
}

/// Extract `info.title`, `info.version`, and the first server / Swagger `basePath`.
pub fn extract_spec_info(doc: &Value) -> SpecInfo {
    let info = doc.get("info").and_then(Value::as_object);
    let title = info
        .and_then(|i| i.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let version = info
        .and_then(|i| i.get("version"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let base_path = doc
        .get("basePath")
        .and_then(Value::as_str)
        .or_else(|| {
            doc.get("servers")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(|s| s.get("url"))
                .and_then(Value::as_str)
        })
        .and_then(extract_url_path);

    SpecInfo {
        title,
        version,
        base_path,
    }
}

fn extract_url_path(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() || s == "/" {
        return None;
    }
    let path_part = if let Some((_, rest)) = s.split_once("://") {
        let idx = rest.find('/')?;
        &rest[idx..]
    } else {
        s
    };
    let clean = path_part.trim_end_matches('/');
    if clean.is_empty() {
        None
    } else if clean.starts_with('/') {
        Some(clean.to_string())
    } else {
        Some(format!("/{clean}"))
    }
}

/// Split a tag string into normalized folder path segments (supporting nested
/// tags like `"Admin/Dangerous"` as well as flat tags like `"Pets"`).
pub fn tag_to_folder_segments(tag: &str) -> Vec<String> {
    tag.split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
