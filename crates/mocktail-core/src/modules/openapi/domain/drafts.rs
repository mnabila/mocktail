//! Mock draft generation from OpenAPI operations.

use super::examples::{BodySource, response_body};
use super::spec::OPERATION_METHODS;
use crate::shared::http::Method;
use serde_json::Value;

/// A mock to be created from one operation+response, before ids/timestamps are
/// assigned by the application layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockDraft {
    pub method: Method,
    pub path_template: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub source: BodySource,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
}

impl MockDraft {
    /// Stable identity key for diffing and round-trip id reuse.
    pub fn key(&self) -> String {
        format!(
            "{} {} {}",
            self.method.as_str(),
            self.path_template,
            self.status
        )
    }

    /// Folder path segments derived from the operation's first non-empty tag.
    pub fn folder_segments(&self) -> Vec<String> {
        self.tags
            .iter()
            .map(|t| super::spec::tag_to_folder_segments(t))
            .find(|segs| !segs.is_empty())
            .unwrap_or_default()
    }

    /// Canonical slash-separated folder path (e.g. `"Pets"` or `"Admin/Dangerous"`).
    pub fn folder_path(&self) -> Option<String> {
        let segs = self.folder_segments();
        (!segs.is_empty()).then(|| segs.join("/"))
    }
}

fn extract_tags(value: Option<&Value>) -> Option<Vec<String>> {
    let tags: Vec<String> = value?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    (!tags.is_empty()).then_some(tags)
}

/// Generate one `MockDraft` per operation+response in a parsed OpenAPI document.
pub fn generate_drafts(doc: &Value) -> Vec<MockDraft> {
    let mut drafts = Vec::new();
    let Some(paths) = doc.get("paths").and_then(Value::as_object) else {
        return drafts;
    };

    for (path, item) in paths {
        let Some(item_obj) = item.as_object() else {
            continue;
        };
        let path_tags = extract_tags(item_obj.get("tags"));
        for (method_str, method) in OPERATION_METHODS {
            let Some(op) = item_obj.get(*method_str) else {
                continue;
            };
            let operation_id = op
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::to_string);
            let summary = op
                .get("summary")
                .and_then(Value::as_str)
                .map(str::to_string);
            let tags = extract_tags(op.get("tags"))
                .or_else(|| path_tags.clone())
                .unwrap_or_default();

            let Some(responses) = op.get("responses").and_then(Value::as_object) else {
                continue;
            };
            for (code, resp) in responses {
                let status = parse_status(code);
                let (content_type, body, source) = response_body(doc, resp);
                drafts.push(MockDraft {
                    method: *method,
                    path_template: path.clone(),
                    status,
                    content_type,
                    body,
                    source,
                    operation_id: operation_id.clone(),
                    summary: summary.clone(),
                    tags: tags.clone(),
                });
            }
        }
    }
    drafts
}

/// Map an OpenAPI response-code key to a numeric status. `default` and ranges
/// like `2XX` collapse to the range's base (`200`).
fn parse_status(code: &str) -> u16 {
    if let Ok(n) = code.parse::<u16>() {
        return n;
    }
    let up = code.to_ascii_uppercase();
    if up == "DEFAULT" {
        return 200;
    }
    if up.len() == 3
        && up.ends_with("XX")
        && let Some(d) = up.chars().next().and_then(|c| c.to_digit(10))
    {
        return (d as u16) * 100;
    }
    200
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_parsing() {
        assert_eq!(parse_status("200"), 200);
        assert_eq!(parse_status("404"), 404);
        assert_eq!(parse_status("default"), 200);
        assert_eq!(parse_status("2XX"), 200);
        assert_eq!(parse_status("5XX"), 500);
    }

    #[test]
    fn generate_drafts_from_minimal_spec() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "t", "version": "1"},
            "paths": {
                "/todos/{id}": {
                    "get": {
                        "operationId": "getTodo",
                        "responses": {
                            "200": {
                                "description": "ok",
                                "content": {"application/json": {
                                    "example": {"id": "1", "title": "x"}
                                }}
                            },
                            "404": {"description": "missing"}
                        }
                    }
                }
            }
        });
        let drafts = generate_drafts(&spec);
        assert_eq!(drafts.len(), 2);
        let ok = drafts.iter().find(|d| d.status == 200).unwrap();
        assert_eq!(ok.method, Method::Get);
        assert_eq!(ok.path_template, "/todos/{id}");
        assert_eq!(ok.source, BodySource::Example);
        assert_eq!(ok.operation_id.as_deref(), Some("getTodo"));
        let nf = drafts.iter().find(|d| d.status == 404).unwrap();
        assert_eq!(nf.source, BodySource::Empty);
        assert!(nf.body.is_empty());
    }
}
