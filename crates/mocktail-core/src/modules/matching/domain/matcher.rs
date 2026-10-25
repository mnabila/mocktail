//! Request matcher evaluation.

use super::path::match_path;
use crate::modules::mock::domain::{MatchKind, MatchTarget, Mock, RequestMatcher};
use crate::shared::http::Request;

/// Evaluate a single request matcher. A missing target value
/// or an invalid regex yields a non-match.
fn matcher_passes(m: &RequestMatcher, req: &Request) -> bool {
    let value: Option<String> = match &m.target {
        MatchTarget::Query(name) => req.query_first(name).map(|s| s.to_string()),
        MatchTarget::Header(name) => req.headers.get(name).map(|s| s.to_string()),
        MatchTarget::Body => Some(String::from_utf8_lossy(&req.body).to_string()),
    };

    match &m.kind {
        MatchKind::Exact(expected) => value.as_deref() == Some(expected.as_str()),
        MatchKind::Contains(needle) => value.map(|v| v.contains(needle)).unwrap_or(false),
        MatchKind::Regex(pat) => match regex::Regex::new(pat) {
            Ok(re) => value.map(|v| re.is_match(&v)).unwrap_or(false),
            Err(_) => false,
        },
        MatchKind::JsonPath { path, equals } => {
            let Some(raw) = value else { return false };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                return false;
            };
            // Accept a dot or slash separated path; normalize to a JSON pointer.
            let pointer = to_json_pointer(path);
            match json.pointer(&pointer) {
                Some(found) => match equals {
                    None => true,
                    Some(exp) => json_value_eq_str(found, exp),
                },
                None => false,
            }
        }
    }
}

/// Turn `user.name` or `/user/name` or `user/0/id` into a JSON pointer
/// (`/user/name`, `/user/0/id`).
fn to_json_pointer(path: &str) -> String {
    let normalized = path.replace('.', "/");
    if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    }
}

fn json_value_eq_str(v: &serde_json::Value, expected: &str) -> bool {
    match v {
        serde_json::Value::String(s) => s == expected,
        serde_json::Value::Bool(b) => b.to_string() == expected,
        serde_json::Value::Number(n) => n.to_string() == expected,
        serde_json::Value::Null => expected == "null",
        _ => false,
    }
}

/// Does `mock` match this request? Returns captured params if so.
pub(super) fn mock_matches(
    mock: &Mock,
    req: &Request,
    spec_path: &str,
) -> Option<Vec<(String, String)>> {
    if mock.method != req.method {
        return None;
    }
    let params = match_path(&mock.path_template, spec_path)?;
    if mock.matchers.iter().all(|m| matcher_passes(m, req)) {
        Some(params)
    } else {
        None
    }
}
