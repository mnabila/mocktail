//! Swagger 2.x normalization into the OpenAPI 3.x response shape.

use super::examples::resolve_ref;
use super::spec::OPERATION_METHODS;
use serde_json::{Map, Value, json};

/// Rewrite a Swagger 2.x document into the OpenAPI 3.x response shape consumed
/// by `generate_drafts`. In 2.x, response bodies live in `schema`/`examples`
/// and media types in `produces`; 3.x nests both under `content`. The rest of
/// the document is kept so `$ref`s into `definitions`/`responses` still resolve.
pub fn normalize_swagger2(mut doc: Value) -> Value {
    let root_produces = media_types(doc.get("produces"));
    let mut paths = Map::new();

    if let Some(source_paths) = doc.get("paths").and_then(Value::as_object) {
        for (path, item) in source_paths {
            let mut item = item.clone();
            if let Some(item_obj) = item.as_object_mut() {
                for (method, _) in OPERATION_METHODS {
                    let Some(op) = item_obj.get_mut(*method).and_then(Value::as_object_mut) else {
                        continue;
                    };
                    let produces = media_types(op.get("produces"))
                        .or_else(|| root_produces.clone())
                        .unwrap_or_else(|| vec!["application/json".to_string()]);
                    if let Some(responses) = op.get_mut("responses").and_then(Value::as_object_mut)
                    {
                        for response in responses.values_mut() {
                            *response = swagger2_response(&doc, response, &produces);
                        }
                    }
                }
            }
            paths.insert(path.clone(), item);
        }
    }

    if let Value::Object(obj) = &mut doc {
        obj.insert("paths".to_string(), Value::Object(paths));
    }
    doc
}

/// Read a Swagger 2.x `produces` array (operation or root level).
fn media_types(value: Option<&Value>) -> Option<Vec<String>> {
    let types: Vec<String> = value?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    (!types.is_empty()).then_some(types)
}

/// Convert one Swagger 2.x response to the 3.x `content` shape. A response-level
/// `$ref` to a reusable root response (`#/responses/...`) is resolved first.
fn swagger2_response(doc: &Value, response: &Value, produces: &[String]) -> Value {
    let response = response
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| resolve_ref(doc, r))
        .unwrap_or(response);

    // Prefer JSON when the operation produces it, else the first declared type.
    let media_type = produces
        .iter()
        .find(|t| t.eq_ignore_ascii_case("application/json"))
        .or_else(|| produces.first())
        .cloned()
        .unwrap_or_else(|| "application/json".to_string());

    let mut out = Map::new();
    if let Some(description) = response.get("description") {
        out.insert("description".to_string(), description.clone());
    }

    // 2.x `examples` maps media type -> example and wins over `schema`, matching
    // the import priority used for 3.x responses.
    let explicit = response
        .get("examples")
        .and_then(Value::as_object)
        .and_then(|examples| {
            examples
                .get(&media_type)
                .or_else(|| examples.values().next())
        });
    let content = match explicit {
        Some(example) => Some(json!({ "example": example })),
        None => response
            .get("schema")
            .map(|schema| json!({ "schema": schema })),
    };
    if let Some(content) = content {
        let mut media = Map::new();
        media.insert(media_type, content);
        out.insert("content".to_string(), Value::Object(media));
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::openapi::domain::{BodySource, generate_drafts};
    use crate::shared::http::Method;

    #[test]
    fn swagger2_spec_generates_drafts() {
        let spec = json!({
            "swagger": "2.0",
            "info": {"title": "Pets", "version": "1"},
            "produces": ["application/json"],
            "paths": {
                "/pets/{id}": {
                    "get": {
                        "operationId": "getPet",
                        "responses": {
                            "200": {
                                "description": "ok",
                                "schema": {"$ref": "#/definitions/Pet"}
                            },
                            "404": {"description": "missing"}
                        }
                    }
                },
                "/pets": {
                    "post": {
                        "produces": ["application/vnd.pet+json"],
                        "responses": {
                            "201": {
                                "description": "created",
                                "schema": {"$ref": "#/definitions/Pet"},
                                "examples": {
                                    "application/vnd.pet+json": {"id": 7, "name": "Rex"}
                                }
                            }
                        }
                    }
                }
            },
            "definitions": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"}
                    }
                }
            }
        });
        let drafts = generate_drafts(&normalize_swagger2(spec));
        assert_eq!(drafts.len(), 3);

        let ok = drafts
            .iter()
            .find(|d| d.status == 200 && d.method == Method::Get)
            .unwrap();
        assert_eq!(ok.path_template, "/pets/{id}");
        assert_eq!(ok.content_type.as_deref(), Some("application/json"));
        assert_eq!(ok.source, BodySource::Schema);
        assert_eq!(
            serde_json::from_slice::<Value>(&ok.body).unwrap(),
            json!({"id": 0, "name": "string"})
        );

        let created = drafts.iter().find(|d| d.status == 201).unwrap();
        assert_eq!(
            created.content_type.as_deref(),
            Some("application/vnd.pet+json")
        );
        assert_eq!(created.source, BodySource::Example);
        assert_eq!(
            serde_json::from_slice::<Value>(&created.body).unwrap(),
            json!({"id": 7, "name": "Rex"})
        );

        let missing = drafts.iter().find(|d| d.status == 404).unwrap();
        assert_eq!(missing.source, BodySource::Empty);
        assert!(missing.body.is_empty());
    }

    #[test]
    fn swagger2_resolves_reusable_response_ref() {
        let spec = json!({
            "swagger": "2.0",
            "paths": {
                "/health": {
                    "get": {
                        "responses": {
                            "200": {"$ref": "#/responses/Healthy"}
                        }
                    }
                }
            },
            "responses": {
                "Healthy": {
                    "description": "ok",
                    "schema": {"type": "object", "properties": {"ok": {"type": "boolean"}}}
                }
            }
        });
        let drafts = generate_drafts(&normalize_swagger2(spec));
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].source, BodySource::Schema);
        assert_eq!(
            serde_json::from_slice::<Value>(&drafts[0].body).unwrap(),
            json!({"ok": true})
        );
    }
}
