//! OpenAPI interface slice: preview, commit, and export.

use crate::http::auth::gate;
use crate::shared::extract::AxumRequest;
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::collection::application as coll_app;
use mocktail_core::modules::openapi::application as openapi_app;
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::CollectionId;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn import_preview_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(import_preview(&req, &ctx, &id).await)
}

pub(crate) async fn import_commit_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(import_commit(&req, &ctx, &id).await)
}

pub(crate) async fn export_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(export(&req, &ctx, &id).await)
}

async fn import_preview(req: &Request, ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let doc = openapi_app::parse_spec(&req.body)?;
    let preview =
        openapi_app::preview_import(ctx.mocks.as_ref(), ctx.folders.as_ref(), &coll, &doc).await?;
    Ok(Response::api_json(200, &preview_json(&preview)))
}

async fn import_commit(req: &Request, ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;

    // Two body shapes: a `{spec, accept?, deleteRemoved?, createFolders?}` wrapper,
    // or the raw spec document (JSON/YAML) with default selection.
    let (spec_bytes, selection) = parse_import_body(&req.body)?;
    let doc = openapi_app::parse_spec(&spec_bytes)?;

    let result = openapi_app::commit_import(
        ctx.mocks.as_ref(),
        ctx.folders.as_ref(),
        ctx.ids.as_ref(),
        ctx.clock.as_ref(),
        &coll,
        &doc,
        selection,
    )
    .await?;

    // Extract description from the spec to update the collection.
    let spec_description = doc
        .get("info")
        .and_then(Value::as_object)
        .and_then(|info| info.get("description").and_then(Value::as_str))
        .unwrap_or("");

    let mut updated_coll = coll.clone();
    updated_coll.description = spec_description.to_string();

    // Bump the collection version so cached mock sets are invalidated.
    let _ = coll_app::update_collection(ctx.collections.as_ref(), updated_coll).await?;

    Ok(Response::api_json(
        200,
        &json!({
            "created": result.created,
            "updated": result.updated,
            "deleted": result.deleted,
            "foldersCreated": result.folders_created,
        }),
    ))
}

/// Split the commit request body into (spec bytes, selection).
fn parse_import_body(body: &[u8]) -> AppResult<(Vec<u8>, openapi_app::ImportSelection)> {
    // Wrapper only when the body is a JSON object carrying a string `spec`.
    if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(body)
        && let Some(Value::String(spec)) = map.get("spec")
    {
        let accept = map.get("accept").and_then(Value::as_array).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<std::collections::HashSet<_>>()
        });
        let delete_removed = map
            .get("deleteRemoved")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let create_folders = map
            .get("createFolders")
            .and_then(Value::as_bool)
            .or_else(|| {
                map.get("folderOrganization")
                    .and_then(Value::as_str)
                    .map(|s| !s.eq_ignore_ascii_case("none"))
            })
            .unwrap_or(true);
        return Ok((
            spec.clone().into_bytes(),
            openapi_app::ImportSelection {
                accept,
                delete_removed,
                create_folders,
            },
        ));
    }
    // Raw spec document, default selection.
    Ok((body.to_vec(), openapi_app::ImportSelection::default()))
}

async fn export(req: &Request, ctx: &Ctx, coll_id: &str) -> AppResult<Response> {
    let coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(coll_id)).await?;
    let format = openapi_app::ExportFormat::parse(req.query_first("format").unwrap_or("json"));
    let (bytes, content_type) =
        openapi_app::export_collection(ctx.mocks.as_ref(), ctx.folders.as_ref(), &coll, format)
            .await?;
    let filename = format!("{}.openapi", coll.slug.as_str());
    Ok(Response::new(200)
        .with_header("content-type", content_type)
        .with_header(
            "content-disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .with_body(bytes))
}

fn preview_json(preview: &openapi_app::ImportPreview) -> Value {
    let (added, changed, unchanged, removed) = preview.counts();
    let folders_added = preview.folders_added_count();
    let folders: Vec<Value> = preview
        .folders
        .iter()
        .map(|f| {
            json!({
                "path": f.path,
                "name": f.name,
                "parentPath": f.parent_path,
                "action": f.action.as_str(),
                "existingFolderId": f.existing_folder_id.as_ref().map(|id| id.as_str()),
            })
        })
        .collect();
    let items: Vec<Value> = preview
        .items
        .iter()
        .map(|item| {
            let (method, path, status, source, body_preview, tag, tags, op_id, summary) =
                match &item.draft {
                    Some(d) => (
                        Some(d.method.as_str()),
                        Some(d.path_template.clone()),
                        Some(d.status),
                        Some(d.source.as_str()),
                        Some(truncate_preview(&d.body)),
                        d.folder_path(),
                        d.tags.clone(),
                        d.operation_id.clone(),
                        d.summary.clone(),
                    ),
                    None => (None, None, None, None, None, None, Vec::new(), None, None),
                };
            json!({
                "key": item.key,
                "action": item.action.as_str(),
                "selected": item.selected,
                "existingMockId": item.existing_mock_id.as_ref().map(|m| m.as_str()),
                "method": method,
                "pathTemplate": path,
                "status": status,
                "source": source,
                "bodyPreview": body_preview,
                "tag": tag,
                "tags": tags,
                "operationId": op_id,
                "summary": summary,
            })
        })
        .collect();
    json!({
        "spec": {
            "title": preview.spec_info.title,
            "version": preview.spec_info.version,
            "basePath": preview.spec_info.base_path,
        },
        "summary": {
            "added": added,
            "changed": changed,
            "unchanged": unchanged,
            "removed": removed,
            "foldersAdded": folders_added,
        },
        "folders": folders,
        "items": items,
    })
}

fn truncate_preview(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    if text.chars().count() > 200 {
        let head: String = text.chars().take(200).collect();
        format!("{head}...")
    } else {
        text.into_owned()
    }
}
