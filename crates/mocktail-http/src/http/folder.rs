//! Folder interface slice: collection folder CRUD.

use crate::http::auth::gate;
use crate::shared::extract::AxumRequest;
use crate::shared::json::{json_body, str_field};
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::collection::application as coll_app;
use mocktail_core::modules::folder::application as folder_app;
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::{CollectionId, FolderId};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn list_folders_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(list_folders(&ctx, &id).await)
}

pub(crate) async fn create_folder_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(create_folder(&req, &ctx, &id).await)
}

pub(crate) async fn delete_folder_route(
    State(ctx): State<Arc<Ctx>>,
    Path((id, fid)): Path<(String, String)>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(delete_folder(&ctx, &id, &fid).await)
}

async fn list_folders(ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let items: Vec<Value> =
        folder_app::list_folders(ctx.folders.as_ref(), &coll.workspace_id, &coll.id)
            .await?
            .iter()
            .map(folder_json)
            .collect();
    Ok(Response::api_json(200, &json!({ "items": items })))
}

async fn create_folder(req: &Request, ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let body = json_body(req)?;
    let name = str_field(&body, "name")?;
    let parent_id = body
        .get("parentId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(FolderId::new);
    let folder = folder_app::create_folder(
        ctx.folders.as_ref(),
        FolderId::new(ctx.ids.new_object_id()),
        coll.workspace_id.clone(),
        coll.id.clone(),
        folder_app::NewFolder { name, parent_id },
        ctx.clock.now_millis(),
    )
    .await?;
    Ok(Response::api_json(201, &folder_json(&folder)))
}

async fn delete_folder(ctx: &Ctx, coll_id: &str, folder_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let folder =
        folder_app::get_folder(ctx.folders.as_ref(), &coll.id, &FolderId::new(folder_id)).await?;
    folder_app::delete_folder(ctx.folders.as_ref(), ctx.mocks.as_ref(), &folder).await?;
    Ok(Response::new(204))
}

fn folder_json(f: &Folder) -> Value {
    json!({
        "id": f.id.as_str(),
        "workspaceId": f.workspace_id.as_str(),
        "collectionId": f.collection_id.as_str(),
        "name": f.name,
        "parentId": f.parent_id.as_ref().map(|id| id.as_str()),
        "createdAt": f.created_at,
    })
}

/// Reject folder ids from another collection (or nonexistent folders).
pub(crate) async fn ensure_folder(
    ctx: &Ctx,
    collection_id: &CollectionId,
    folder_id: Option<&FolderId>,
) -> AppResult<()> {
    if let Some(folder_id) = folder_id {
        folder_app::get_folder(ctx.folders.as_ref(), collection_id, folder_id).await?;
    }
    Ok(())
}
