//! OpenAPI import -> serve -> export -> re-import round-trip, all through
//! `handle` with in-memory fakes.

mod common;

use common::{admin_json, araw, areq, block_on, body_json, harness, mock_req};
use mocktail_core::shared::http::Method;
use mocktail_http::handle;

fn sample_spec() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "openapi": "3.0.0",
        "info": {"title": "Todo API", "version": "1.0.0"},
        "paths": {
            "/todos": {
                "get": {
                    "operationId": "listTodos",
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {"application/json": {
                                "example": [{"id": "1", "title": "first"}]
                            }}
                        }
                    }
                }
            },
            "/todos/{id}": {
                "get": {
                    "operationId": "getTodo",
                    "responses": {
                        "200": {
                            "description": "ok",
                            "content": {"application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "string"},
                                        "done": {"type": "boolean"}
                                    }
                                }
                            }}
                        },
                        "404": {"description": "not found"}
                    }
                }
            }
        }
    }))
    .unwrap()
}

#[test]
fn import_serve_export_reimport_roundtrip() {
    block_on(async {
        let h = harness();
        let ctx = &h.ctx;

        // Workspace + manual collection.
        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({"name": "Acme"}),
                ),
                ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();
        let coll = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/workspaces/{ws_id}/collections"),
                    serde_json::json!({"name": "Todo", "basePath": "/api/v1"}),
                ),
                ctx,
            )
            .await,
        );
        let coll_id = coll["id"].as_str().unwrap().to_string();

        // 1. Preview the import: 3 drafts, all added (empty collection).
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import/preview"),
                sample_spec(),
            ),
            ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            200,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        let preview = admin_json(&resp);
        assert_eq!(preview["summary"]["added"], 3);
        // The 404 is present but not selected by default (non-2xx).
        let selected = preview["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["selected"] == true)
            .count();
        assert_eq!(selected, 2);

        // 2. Commit (raw spec, default selection => the two 2xx responses).
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import"),
                sample_spec(),
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let result = admin_json(&resp);
        assert_eq!(result["created"], 2);
        assert_eq!(result["updated"], 0);

        // 3. Serve a generated mock. The schema-generated body fills defaults.
        let resp = handle(mock_req("acme", "/todo/api/v1/todos/9"), ctx).await;
        assert_eq!(resp.status, 200);
        let served = body_json(&resp);
        assert_eq!(served, serde_json::json!({"id": "string", "done": true}));

        // The list endpoint returns the explicit example.
        let resp = handle(mock_req("acme", "/todo/api/v1/todos"), ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(
            body_json(&resp),
            serde_json::json!([{"id": "1", "title": "first"}])
        );

        // 4. Export to OpenAPI JSON.
        let mut export_req = areq(
            &h,
            Method::Get,
            &format!("/__admin/collections/{coll_id}/export"),
            serde_json::Value::Null,
        );
        export_req.query = vec![("format".to_string(), "json".to_string())];
        let resp = handle(export_req, ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(resp.headers.get("content-type"), Some("application/json"));
        let exported = resp.body.clone();
        let exported_doc: serde_json::Value = serde_json::from_slice(&exported).unwrap();
        assert_eq!(exported_doc["openapi"], "3.0.3");
        assert!(exported_doc["paths"]["/todos/{id}"]["get"].is_object());

        // 5. Re-import the exported document: everything is Unchanged (stable
        //    round-trip), nothing added or changed.
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import/preview"),
                exported,
            ),
            ctx,
        )
        .await;
        let reimport = admin_json(&resp);
        assert_eq!(reimport["summary"]["added"], 0);
        assert_eq!(reimport["summary"]["changed"], 0);
        assert_eq!(reimport["summary"]["unchanged"], 2);
        assert_eq!(reimport["summary"]["removed"], 0);
    });
}

#[test]
fn imports_swagger2_document_from_yaml() {
    block_on(async {
        let h = harness();
        let ctx = &h.ctx;

        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({"name": "Acme"}),
                ),
                ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();
        let coll = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/workspaces/{ws_id}/collections"),
                    serde_json::json!({"name": "Pet Store"}),
                ),
                ctx,
            )
            .await,
        );
        let coll_id = coll["id"].as_str().unwrap().to_string();

        // Swagger 2.x puts the body schema on the response and media types in
        // `produces`; both root-level and response-level shapes must import.
        let spec = br#"
swagger: "2.0"
info:
  title: Pets API
  version: "1.0.0"
produces:
  - application/json
paths:
  /pets/{id}:
    get:
      operationId: getPet
      responses:
        "200":
          description: ok
          schema:
            type: object
            properties:
              id:
                type: integer
              name:
                type: string
"#
        .to_vec();

        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import/preview"),
                spec.clone(),
            ),
            ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            200,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        let preview = admin_json(&resp);
        assert_eq!(preview["summary"]["added"], 1);
        assert_eq!(preview["items"][0]["selected"], true);
        assert_eq!(preview["items"][0]["source"], "schema");

        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import"),
                spec,
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        assert_eq!(admin_json(&resp)["created"], 1);

        let resp = handle(mock_req("acme", "/pet-store/pets/9"), ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(
            body_json(&resp),
            serde_json::json!({"id": 0, "name": "string"})
        );
    });
}

#[test]
fn import_auto_creates_folders_from_tags_and_reuses_existing() {
    block_on(async {
        let h = harness();
        let ctx = &h.ctx;

        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({"name": "Acme"}),
                ),
                ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();
        let coll = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/workspaces/{ws_id}/collections"),
                    serde_json::json!({"name": "Shop API"}),
                ),
                ctx,
            )
            .await,
        );
        let coll_id = coll["id"].as_str().unwrap().to_string();

        let spec = br#"
swagger: "2.0"
info:
  title: Shop API
  version: "1.0.0"
basePath: /v1
produces:
  - application/json
paths:
  /pets:
    get:
      tags:
        - Pets
      summary: List all pets
      responses:
        "200":
          description: ok
    post:
      tags:
        - Pets
      summary: Create a pet
      responses:
        "201":
          description: created
  /admin/purge:
    delete:
      tags:
        - Admin/Dangerous
      summary: Purge data
      responses:
        "204":
          description: purged
  /health:
    get:
      summary: Health check (untagged)
      responses:
        "200":
          description: healthy
"#;

        // 1. Preview reports the 3 folders to be created (Pets, Admin, Admin/Dangerous)
        //    and associates each item with its tag.
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import/preview"),
                spec.to_vec(),
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let preview = admin_json(&resp);
        assert_eq!(preview["spec"]["title"], "Shop API");
        assert_eq!(preview["spec"]["basePath"], "/v1");
        assert_eq!(preview["summary"]["foldersAdded"], 3);
        let preview_folders = preview["folders"].as_array().unwrap();
        assert_eq!(preview_folders.len(), 3);
        assert!(
            preview_folders
                .iter()
                .any(|f| f["path"] == "Pets" && f["action"] == "added")
        );
        assert!(preview_folders.iter().any(|f| {
            f["path"] == "Admin/Dangerous"
                && f["parentPath"] == "Admin"
                && f["name"] == "Dangerous"
                && f["action"] == "added"
        }));

        // 2. Commit import: auto-creates the 3 folders and places mocks inside them.
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import"),
                spec.to_vec(),
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let commit_res = admin_json(&resp);
        assert_eq!(commit_res["created"], 4);
        assert_eq!(commit_res["foldersCreated"], 3);

        // Verify folders in collection.
        let folders_resp = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::Value::Null,
                ),
                ctx,
            )
            .await,
        );
        let folders = folders_resp["items"].as_array().unwrap();
        assert_eq!(folders.len(), 3);
        let pets_folder = folders.iter().find(|f| f["name"] == "Pets").unwrap();
        assert!(pets_folder["parentId"].is_null());
        let admin_folder = folders.iter().find(|f| f["name"] == "Admin").unwrap();
        assert!(admin_folder["parentId"].is_null());
        let dangerous_folder = folders.iter().find(|f| f["name"] == "Dangerous").unwrap();
        assert_eq!(dangerous_folder["parentId"], admin_folder["id"]);

        // Verify mocks are assigned to the right folderId.
        let mocks_resp = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/collections/{coll_id}/mocks"),
                    serde_json::Value::Null,
                ),
                ctx,
            )
            .await,
        );
        let mocks = mocks_resp["items"].as_array().unwrap();
        assert_eq!(mocks.len(), 4);
        for m in mocks.iter().filter(|m| m["pathTemplate"] == "/pets") {
            assert_eq!(m["folderId"], pets_folder["id"]);
        }
        let purge = mocks
            .iter()
            .find(|m| m["pathTemplate"] == "/admin/purge")
            .unwrap();
        assert_eq!(purge["folderId"], dangerous_folder["id"]);
        let health = mocks
            .iter()
            .find(|m| m["pathTemplate"] == "/health")
            .unwrap();
        assert!(health["folderId"].is_null());

        // 3. Re-importing the same spec reuses existing folders (foldersCreated == 0).
        let resp = handle(
            araw(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import"),
                spec.to_vec(),
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let re_commit = admin_json(&resp);
        assert_eq!(re_commit["foldersCreated"], 0);
        assert_eq!(re_commit["updated"], 4);

        // 4. Exporting preserves folder hierarchy as OpenAPI operation tags.
        let export_resp = handle(
            areq(
                &h,
                Method::Get,
                &format!("/__admin/collections/{coll_id}/export"),
                serde_json::Value::Null,
            ),
            ctx,
        )
        .await;
        assert_eq!(export_resp.status, 200);
        let exported: serde_json::Value = serde_json::from_slice(&export_resp.body).unwrap();
        assert_eq!(
            exported["paths"]["/pets"]["get"]["tags"],
            serde_json::json!(["Pets"])
        );
        assert_eq!(
            exported["paths"]["/admin/purge"]["delete"]["tags"],
            serde_json::json!(["Admin/Dangerous"])
        );
    });
}

#[test]
fn import_can_disable_auto_folder_creation() {
    block_on(async {
        let h = harness();
        let ctx = &h.ctx;

        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({"name": "Acme"}),
                ),
                ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();
        let coll = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/workspaces/{ws_id}/collections"),
                    serde_json::json!({"name": "Flat API"}),
                ),
                ctx,
            )
            .await,
        );
        let coll_id = coll["id"].as_str().unwrap().to_string();

        let spec = r#"{"openapi":"3.0.0","info":{"title":"Flat","version":"1"},"paths":{"/users":{"get":{"tags":["Users"],"responses":{"200":{"description":"ok"}}}}}}"#;
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/import"),
                serde_json::json!({
                    "spec": spec,
                    "createFolders": false
                }),
            ),
            ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let res = admin_json(&resp);
        assert_eq!(res["created"], 1);
        assert_eq!(res["foldersCreated"], 0);

        let folders_resp = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::Value::Null,
                ),
                ctx,
            )
            .await,
        );
        assert!(folders_resp["items"].as_array().unwrap().is_empty());
    });
}
