//! Export mocks and folders to an OpenAPI 3.0 document.

use crate::modules::folder::domain::Folder;
use crate::modules::mock::domain::Mock;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Export a collection's mocks to an OpenAPI 3.0 document (inverse of import),
/// enabling round-trip. `base_path` becomes a `servers` entry.
pub fn export_document(title: &str, base_path: &str, mocks: &[Mock]) -> Value {
    export_document_with_folders(title, base_path, mocks, &[])
}

/// Export a collection's mocks and folder hierarchy to an OpenAPI 3.0 document.
/// Mocks placed inside a folder emit that folder path as an operation `tags` entry.
pub fn export_document_with_folders(
    title: &str,
    base_path: &str,
    mocks: &[Mock],
    folders: &[Folder],
) -> Value {
    let folder_paths = build_folder_paths(folders);
    // paths -> method(lower) -> operation.
    let mut paths: Map<String, Value> = Map::new();

    for mock in mocks {
        let path_entry = paths
            .entry(mock.path_template.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let path_obj = path_entry.as_object_mut().expect("path entry is object");

        let method_key = mock.method.as_str().to_ascii_lowercase();
        let op_entry = path_obj
            .entry(method_key)
            .or_insert_with(|| json!({ "responses": {} }));
        if let Some(folder_id) = &mock.folder_id
            && let Some(tag_path) = folder_paths.get(folder_id.as_str())
            && let Some(op_obj) = op_entry.as_object_mut()
            && !op_obj.contains_key("tags")
        {
            op_obj.insert("tags".to_string(), json!([tag_path]));
        }
        let responses = op_entry
            .get_mut("responses")
            .and_then(Value::as_object_mut)
            .expect("responses object");

        let content_type = mock
            .response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "application/json".to_string());

        let response_obj = build_response(&mock.response.body, &content_type);
        responses.insert(mock.response.status.to_string(), response_obj);
    }

    let mut doc = json!({
        "openapi": "3.0.3",
        "info": { "title": title, "version": "1.0.0" },
        "paths": Value::Object(paths),
    });
    if !base_path.is_empty() {
        doc.as_object_mut()
            .unwrap()
            .insert("servers".to_string(), json!([{ "url": base_path }]));
    }
    doc
}

/// Resolve each folder ID to its slash-separated ancestor path (e.g. `"Admin/Dangerous"`).
pub fn build_folder_paths(folders: &[Folder]) -> HashMap<String, String> {
    let by_id: HashMap<&str, &Folder> = folders.iter().map(|f| (f.id.as_str(), f)).collect();
    let mut out = HashMap::new();
    for f in folders {
        let mut parts = vec![f.name.trim().to_string()];
        let mut seen = vec![f.id.as_str()];
        let mut cur = f.parent_id.as_ref().map(|p| p.as_str());
        while let Some(pid) = cur {
            if seen.contains(&pid) {
                break;
            }
            seen.push(pid);
            let Some(parent) = by_id.get(pid) else {
                break;
            };
            parts.push(parent.name.trim().to_string());
            cur = parent.parent_id.as_ref().map(|p| p.as_str());
        }
        parts.reverse();
        out.insert(f.id.as_str().to_string(), parts.join("/"));
    }
    out
}

fn build_response(body: &[u8], content_type: &str) -> Value {
    if body.is_empty() {
        return json!({ "description": "" });
    }
    // Represent the body as an `example`: parse JSON if possible, else a string.
    let example: Value = if content_type.contains("json") {
        serde_json::from_slice(body)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(body).into_owned()))
    } else {
        Value::String(String::from_utf8_lossy(body).into_owned())
    };
    json!({
        "description": "",
        "content": { content_type: { "example": example } }
    })
}
