//! Workspace interface slice: admin CRUD for workspaces.

use crate::http::auth::{Actor, gate};
use crate::shared::extract::AxumRequest;
use crate::shared::json::{json_body, parse_slug_or_derive, str_field};
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::auth::domain::{Membership, Role};
use mocktail_core::modules::workspace::application as ws_app;
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::{MembershipId, Slug, WorkspaceId};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn list_workspaces_route(
    State(ctx): State<Arc<Ctx>>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    finish_admin(list_workspaces(&ctx, &actor).await)
}

pub(crate) async fn create_workspace_route(
    State(ctx): State<Arc<Ctx>>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    finish_admin(create_workspace(&req, &ctx, &actor).await)
}

pub(crate) async fn patch_workspace_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(patch_workspace(&req, &ctx, &ws_id).await)
}

pub(crate) async fn delete_workspace_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(delete_workspace(&ctx, &ws_id).await)
}

/// List workspaces the actor belongs to, with the actor's role in each.
async fn list_workspaces(ctx: &Ctx, actor: &Actor) -> AppResult<Response> {
    let memberships = ctx.memberships.list_for_user(&actor.user.id).await?;
    let mut items = Vec::new();
    for m in &memberships {
        if let Some(ws) = ctx.workspaces.find_by_id(&m.workspace_id).await? {
            let mut j = workspace_json(&ws);
            j.as_object_mut()
                .unwrap()
                .insert("role".to_string(), json!(m.role.as_str()));
            items.push(j);
        }
    }
    Ok(Response::api_json(200, &json!({ "items": items })))
}

/// Create a workspace; the creator becomes its `owner`.
async fn create_workspace(req: &Request, ctx: &Ctx, actor: &Actor) -> AppResult<Response> {
    let body = json_body(req)?;
    let name = str_field(&body, "name")?;
    let slug = parse_slug_or_derive(&body, &name)?;
    let ws = ws_app::create_workspace(
        ctx.workspaces.as_ref(),
        WorkspaceId::new(ctx.ids.new_object_id()),
        name,
        slug,
        ctx.clock.now_millis(),
    )
    .await?;
    let membership = Membership {
        id: MembershipId::new(ctx.ids.new_object_id()),
        user_id: actor.user.id.clone(),
        workspace_id: ws.id.clone(),
        role: Role::Owner,
    };
    ctx.memberships.insert(&membership).await?;
    Ok(Response::api_json(201, &workspace_json(&ws)))
}

async fn patch_workspace(req: &Request, ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let id = WorkspaceId::new(ws_id);
    let mut ws = ctx
        .workspaces
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace {ws_id}")))?;
    let body = json_body(req)?;
    if let Some(name) = body.get("name").and_then(Value::as_str) {
        ws.name = name.to_string();
    }
    if let Some(slug) = body.get("slug").and_then(Value::as_str) {
        ws.slug = Slug::parse(slug).map_err(|e| AppError::Domain(e.into()))?;
    }
    let updated = ws_app::update_workspace(ctx.workspaces.as_ref(), ws).await?;
    Ok(Response::api_json(200, &workspace_json(&updated)))
}

async fn delete_workspace(ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let id = WorkspaceId::new(ws_id);
    if ctx.workspaces.find_by_id(&id).await?.is_none() {
        return Err(AppError::NotFound(format!("workspace {ws_id}")));
    }
    ws_app::delete_workspace(ctx.workspaces.as_ref(), &id).await?;
    Ok(Response::new(204))
}

fn workspace_json(w: &Workspace) -> Value {
    json!({
        "id": w.id.as_str(),
        "name": w.name,
        "slug": w.slug.as_str(),
        "createdAt": w.created_at,
    })
}
