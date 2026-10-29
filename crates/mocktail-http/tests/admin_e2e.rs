//! Admin flows added for the SvelteKit portal: folders, IP rate limiting,
//! full request logs, workspace lifecycle, and email invites.

mod common;

use common::{admin_json, anon, areq, block_on, body_json, harness, mock_req};
use mocktail_core::modules::auth::application::UserRepository;
use mocktail_core::modules::auth::domain::{Identity, User};
use mocktail_core::shared::http::Method;
use mocktail_core::shared::ids::UserId;
use mocktail_http::handle;

async fn create_workspace(h: &common::Harness) -> String {
    let ws = admin_json(
        &handle(
            areq(
                h,
                Method::Post,
                "/__admin/workspaces",
                serde_json::json!({"name": "Acme"}),
            ),
            &h.ctx,
        )
        .await,
    );
    ws["id"].as_str().unwrap().to_string()
}

async fn create_collection(h: &common::Harness, ws_id: &str) -> String {
    let coll = admin_json(
        &handle(
            areq(
                h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({"name": "Todo", "basePath": ""}),
            ),
            &h.ctx,
        )
        .await,
    );
    coll["id"].as_str().unwrap().to_string()
}

#[test]
fn folders_rate_limit_and_full_logs() {
    block_on(async {
        let h = harness();
        let ws_id = create_workspace(&h).await;
        let coll_id = create_collection(&h, &ws_id).await;

        // Create a folder and a sub-folder.
        let folder = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::json!({"name": "API"}),
                ),
                &h.ctx,
            )
            .await,
        );
        let folder_id = folder["id"].as_str().unwrap().to_string();
        let sub = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::json!({"name": "Users", "parentId": folder_id}),
                ),
                &h.ctx,
            )
            .await,
        );
        let sub_id = sub["id"].as_str().unwrap().to_string();
        assert_eq!(sub["parentId"], folder_id);

        let list = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::Value::Null,
                ),
                &h.ctx,
            )
            .await,
        );
        assert_eq!(list["items"].as_array().unwrap().len(), 2);

        // A mock in the sub-folder with an object header map, a content type,
        // and a legacy scope/key payload that must be ignored.
        let created = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/ping",
                    "folderId": sub_id,
                    "response": {
                        "status": 200,
                        "contentType": "text/plain",
                        "headers": {"x-test": "yes"},
                        "body": "pong"
                    },
                    "rateLimit": {
                        "limit": 1,
                        "windowSecs": 60,
                        "status": 429,
                        "scope": "ip",
                        "key": "ignored"
                    }
                }),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(
            created.status,
            201,
            "{:?}",
            String::from_utf8_lossy(&created.body)
        );
        let mock = admin_json(&created);
        let mock_id = mock["id"].as_str().unwrap().to_string();
        assert_eq!(mock["folderId"], sub_id);
        assert_eq!(mock["response"]["contentType"], "text/plain");
        assert_eq!(mock["response"]["headers"]["x-test"], "yes");
        assert!(mock["response"]["headers"].get("content-type").is_none());
        assert_eq!(mock["rateLimit"]["limit"], 1);
        assert!(mock["rateLimit"].get("scope").is_none());

        // First request succeeds, second from the same IP is throttled.
        let mut req = mock_req("acme", "/todo/ping");
        req.headers.insert("x-custom", "kept");
        req.body = b"hello".to_vec();
        let first = handle(req, &h.ctx).await;
        assert_eq!(first.status, 200);
        assert_eq!(first.headers.get("content-type"), Some("text/plain"));
        assert_eq!(first.body, b"pong");

        let second = handle(mock_req("acme", "/todo/ping"), &h.ctx).await;
        assert_eq!(second.status, 429);
        assert!(second.headers.get("retry-after").is_some());

        // Logs retain full request + response headers and bodies.
        let logs = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/workspaces/{ws_id}/logs"),
                    serde_json::Value::Null,
                ),
                &h.ctx,
            )
            .await,
        );
        let items = logs["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let ok = items.iter().find(|l| l["status"] == 200).unwrap();
        assert_eq!(ok["requestHeaders"]["x-custom"], "kept");
        assert_eq!(ok["requestBody"], "hello");
        assert_eq!(ok["response"]["headers"]["content-type"], "text/plain");
        assert_eq!(ok["response"]["body"], "pong");
        let limited = items.iter().find(|l| l["status"] == 429).unwrap();
        assert!(
            limited["matchReason"]
                .as_str()
                .unwrap()
                .contains("rate-limited")
        );
        assert_eq!(limited["response"]["status"], 429);

        // Deleting the sub-folder keeps the mock but moves it to the root;
        // deleting the parent removes the rest of the subtree.
        let resp = handle(
            areq(
                &h,
                Method::Delete,
                &format!("/__admin/collections/{coll_id}/folders/{sub_id}"),
                serde_json::Value::Null,
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 204);
        let mock = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/mocks/{mock_id}"),
                    serde_json::Value::Null,
                ),
                &h.ctx,
            )
            .await,
        );
        assert!(mock["folderId"].is_null());

        let resp = handle(
            areq(
                &h,
                Method::Delete,
                &format!("/__admin/collections/{coll_id}/folders/{folder_id}"),
                serde_json::Value::Null,
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 204);
        let list = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Get,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::Value::Null,
                ),
                &h.ctx,
            )
            .await,
        );
        assert!(list["items"].as_array().unwrap().is_empty());

        // Workspace deletion cascades every child row.
        let resp = handle(
            areq(
                &h,
                Method::Delete,
                &format!("/__admin/workspaces/{ws_id}"),
                serde_json::Value::Null,
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 204);
        assert!(h.repos.workspaces.borrow().is_empty());
        assert!(h.repos.collections.borrow().is_empty());
        assert!(h.repos.folders.borrow().is_empty());
        assert!(h.repos.mocks.borrow().is_empty());
        assert!(h.repos.logs.borrow().is_empty());
        assert!(h.repos.memberships.borrow().is_empty());
    });
}

#[test]
fn workspace_update_and_email_invite() {
    block_on(async {
        let h = harness();
        let ws_id = create_workspace(&h).await;

        // Invite an already-registered account by email.
        h.repos.users.borrow_mut().push(User {
            id: UserId::new("invitee0000000000000001"),
            username: "invitee".into(),
            display_name: "Invitee".into(),
            created_at: 0,
        });
        // Seed the identity for the invitee
        let invitee_id = UserId::new("invitee0000000000000001");
        h.repos.add_identity(&invitee_id, "github", "2", Some("invitee@example.com".into())).await.unwrap();
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/members"),
                serde_json::json!({"email": "INVITEE@example.com", "role": "editor"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            201,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        let member = admin_json(&resp);
        assert_eq!(member["email"], "invitee@example.com");
        assert_eq!(member["role"], "editor");

        // Ambiguous email => 409 rather than guessing an account.
        h.repos.users.borrow_mut().push(User {
            id: UserId::new("invitee0000000000000002"),
            username: "otherinvitee".into(),
            display_name: "Other Invitee".into(),
            created_at: 0,
        });
        // Seed the identity for the other invitee
        let other_invitee_id = UserId::new("invitee0000000000000002");
        h.repos.add_identity(&other_invitee_id, "google", "3", Some("invitee@example.com".into())).await.unwrap();
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/members"),
                serde_json::json!({"email": "invitee@example.com", "role": "viewer"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 409);

        // Rename + re-slug the workspace.
        let resp = handle(
            areq(
                &h,
                Method::Patch,
                &format!("/__admin/workspaces/{ws_id}"),
                serde_json::json!({"name": "Acme Renamed", "slug": "acme-renamed"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let ws = admin_json(&resp);
        assert_eq!(ws["name"], "Acme Renamed");
        assert_eq!(ws["slug"], "acme-renamed");

        // A viewer cannot rename the workspace.
        for m in h.repos.memberships.borrow_mut().iter_mut() {
            if m.workspace_id.as_str() == ws_id && m.user_id.as_str() == h.user_id {
                m.role = mocktail_core::modules::auth::domain::Role::Viewer;
            }
        }
        let resp = handle(
            areq(
                &h,
                Method::Patch,
                &format!("/__admin/workspaces/{ws_id}"),
                serde_json::json!({"name": "Nope"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 403);
    });
}

#[test]
fn patch_mock_moves_between_folders_without_rewriting_it() {
    block_on(async {
        let h = harness();
        let ws_id = create_workspace(&h).await;
        let coll_id = create_collection(&h, &ws_id).await;

        let folder = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/collections/{coll_id}/folders"),
                    serde_json::json!({"name": "Moved"}),
                ),
                &h.ctx,
            )
            .await,
        );
        let folder_id = folder["id"].as_str().unwrap().to_string();

        let created = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    &format!("/__admin/collections/{coll_id}/mocks"),
                    serde_json::json!({
                        "method": "GET",
                        "pathTemplate": "/keep-me",
                        "response": {"status": 200, "body": "ok"}
                    }),
                ),
                &h.ctx,
            )
            .await,
        );
        let mock_id = created["id"].as_str().unwrap().to_string();
        assert!(created["folderId"].is_null());

        let moved = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Patch,
                    &format!("/__admin/mocks/{mock_id}"),
                    serde_json::json!({"folderId": folder_id}),
                ),
                &h.ctx,
            )
            .await,
        );
        assert_eq!(moved["folderId"], folder_id);
        assert_eq!(moved["pathTemplate"], "/keep-me");
        assert_eq!(moved["response"]["body"], "ok");

        let moved = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Patch,
                    &format!("/__admin/mocks/{mock_id}"),
                    serde_json::json!({"folderId": null}),
                ),
                &h.ctx,
            )
            .await,
        );
        assert!(moved["folderId"].is_null());

        let resp = handle(
            areq(
                &h,
                Method::Patch,
                &format!("/__admin/mocks/{mock_id}"),
                serde_json::json!({"folderId": "missing"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 404);

        let resp = handle(
            areq(
                &h,
                Method::Patch,
                &format!("/__admin/mocks/{mock_id}"),
                serde_json::json!({"folderId": 42}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 400);
    });
}

#[test]
fn admin_responses_use_the_standard_envelope() {
    block_on(async {
        let h = harness();

        // Success: 200 + service `000` + sequence `0`.
        let resp = handle(
            anon(Method::Get, "/__admin/health", serde_json::Value::Null),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
        let body = body_json(&resp);
        assert_eq!(body["status"]["code"], "2000000");
        assert_eq!(body["status"]["message"], "OK");
        assert_eq!(body["data"]["status"], "ok");
        assert_eq!(body["metadata"], serde_json::json!({}));

        // Created responses keep the HTTP status in the envelope code.
        let resp = handle(
            areq(
                &h,
                Method::Post,
                "/__admin/workspaces",
                serde_json::json!({"name": "Acme"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 201);
        let body = body_json(&resp);
        assert_eq!(body["status"]["code"], "2010000");
        assert_eq!(body["status"]["message"], "Created");
        assert_eq!(body["data"]["slug"], "acme");

        // Errors use the same shape and carry no payload.
        let resp = handle(
            areq(
                &h,
                Method::Get,
                "/__admin/collections/missing",
                serde_json::Value::Null,
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 404);
        let body = body_json(&resp);
        assert_eq!(body["status"]["code"], "4040000");
        assert!(
            body["status"]["message"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
        assert_eq!(body["data"], serde_json::json!({}));
        assert_eq!(body["metadata"], serde_json::json!({}));
    });
}
