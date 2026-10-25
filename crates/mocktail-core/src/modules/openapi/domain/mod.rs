//! Pure OpenAPI <-> entity transforms: Swagger 2.x normalization, schema example
//! generation, mock-draft generation from a parsed spec, and export to an
//! OpenAPI 3.0 document. No I/O, no ports — the most heavily
//! unit-tested part of the feature.

pub mod drafts;
pub mod examples;
pub mod export;
pub mod spec;
pub mod swagger2;

pub use drafts::{MockDraft, generate_drafts};
pub use examples::{BodySource, gen_example};
pub use export::{export_document, export_document_with_folders};
pub use spec::{SpecInfo, extract_spec_info, tag_to_folder_segments};
pub use swagger2::normalize_swagger2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::http::Method;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn status_parsing() {
        // Test internal parse_status via generate_drafts or expose it.
        // Since parse_status is private to drafts, we test the output of generate_drafts.
        let spec = json!({
            "openapi": "3.0.0",
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            "200": { "description": "ok" },
                            "default": { "description": "default" },
                            "404": { "description": "not found" },
                            "5XX": { "description": "server error" }
                        }
                    }
                }
            }
        });
        let drafts = generate_drafts(&spec);
        assert_eq!(drafts.len(), 4);
        let statuses: Vec<u16> = drafts.iter().map(|d| d.status).collect();
        assert!(statuses.contains(&200));
        assert!(statuses.contains(&200)); // default -> 200
        assert!(statuses.contains(&404));
        assert!(statuses.contains(&500)); // 5XX -> 500
    }

    #[test]
    fn schema_example_object() {
        let doc = json!({});
        let schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"},
                "name": {"type": "string"},
                "active": {"type": "boolean"}
            }
        });
        let mut v = Vec::new();
        let out = gen_example(&doc, &schema, &mut v, 0);
        assert_eq!(out, json!({"id": 0, "name": "string", "active": true}));
    }

    #[test]
    fn resolves_ref_and_breaks_cycle() {
        let doc = json!({
            "components": {"schemas": {
                "Node": {"type": "object", "properties": {
                    "next": {"$ref": "#/components/schemas/Node"},
                    "label": {"type": "string"}
                }}
            }}
        });
        let schema = json!({"$ref": "#/components/schemas/Node"});
        let mut v = Vec::new();
        let out = gen_example(&doc, &schema, &mut v, 0);
        assert_eq!(out, json!({"next": null, "label": "string"}));
    }

    #[test]
    fn example_wins_over_schema() {
        let doc = json!({});
        let schema = json!({"type": "string", "example": "hello"});
        let mut v = Vec::new();
        assert_eq!(gen_example(&doc, &schema, &mut v, 0), json!("hello"));
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
        let drafts = generate_drafts(&super::swagger2::normalize_swagger2(spec));
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
        let drafts = generate_drafts(&super::swagger2::normalize_swagger2(spec));
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].source, BodySource::Schema);
        assert_eq!(
            serde_json::from_slice::<Value>(&drafts[0].body).unwrap(),
            json!({"ok": true})
        );
    }
}
