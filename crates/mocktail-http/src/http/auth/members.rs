//! Current-user and workspace-membership interface handlers.

use crate::http::auth::{Actor, gate};
use crate::shared::extract::AxumRequest;
use crate::shared::json::{json_body, str_field};
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::auth::domain::{Membership, Role, User};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::{MembershipId, WorkspaceId};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn me_route(
    State(ctx): State<Arc<Ctx>>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    finish_admin(me(&ctx, &actor).await)
}

pub(crate) async fn list_members_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(list_members(&ctx, &ws_id).await)
}

pub(crate) async fn add_member_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(add_member(&req, &ctx, &ws_id).await)
}

pub(crate) async fn update_member_route(
    State(ctx): State<Arc<Ctx>>,
    Path((ws_id, mid)): Path<(String, String)>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(update_member(&req, &ctx, &ws_id, &mid).await)
}

pub(crate) async fn remove_member_route(
    State(ctx): State<Arc<Ctx>>,
    Path((ws_id, mid)): Path<(String, String)>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(remove_member(&ctx, &ws_id, &mid).await)
}

pub(crate) async fn update_user_route(
    State(ctx): State<Arc<Ctx>>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    finish_admin(update_user(&req, &ctx, &actor).await)
}

async fn update_user(req: &Request, ctx: &Ctx, actor: &Actor) -> AppResult<Response> {
    let body = json_body(req)?;
    let username = str_field(&body, "username")?.trim().to_string();
    let display_name = str_field(&body, "displayName")?.trim().to_string();

    if username.is_empty() || display_name.is_empty() {
        return Err(AppError::BadRequest("username and display name are required".into()));
    }

    // Check if username is taken by someone else
    if let Some(existing) = ctx.users.find_by_username(&username).await?
        && existing.id != actor.user.id
    {
        return Err(AppError::Conflict("username is already taken".into()));
    }

    let mut user = actor.user.clone();
    user.username = username;
    user.display_name = display_name;

    ctx.users.update(&user).await?;

    Ok(Response::api_json(200, &json!({ "user": user_json(&user) })))
}

async fn me(ctx: &Ctx, actor: &Actor) -> AppResult<Response> {
    let memberships = ctx.memberships.list_for_user(&actor.user.id).await?;
    let identities = ctx.users.find_identities(&actor.user.id).await?;
    let ms: Vec<Value> = memberships
        .iter()
        .map(|m| json!({"workspaceId": m.workspace_id.as_str(), "role": m.role.as_str()}))
        .collect();
    let ids: Vec<Value> = identities
        .iter()
        .map(|i| json!({"provider": i.provider}))
        .collect();
    Ok(Response::api_json(
        200,
        &json!({ "user": user_json(&actor.user), "memberships": ms, "providers": ids, "csrf": actor.claims.csrf }),
    ))
}

async fn list_members(ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    let memberships = ctx.memberships.list_for_workspace(&ws).await?;
    let mut items = Vec::new();
    for m in &memberships {
        let user = ctx.users.find_by_id(&m.user_id).await?;
        items.push(membership_json(m, user.as_ref()));
    }
    Ok(Response::api_json(200, &json!({ "items": items })))
}

async fn add_member(req: &Request, ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    let body = json_body(req)?;
    let username = str_field(&body, "username")?.trim().to_string();
    if username.is_empty() {
        return Err(AppError::BadRequest("username is required".into()));
    }
    let role = Role::parse(&str_field(&body, "role")?)
        .ok_or_else(|| AppError::BadRequest("invalid role".into()))?;

    // An invite works only for an already-registered account.
    let user = match ctx.users.find_by_username(&username).await? {
        Some(u) => u,
        None => {
            return Err(AppError::NotFound(
                "no registered user with that username".into(),
            ));
        }
    };

    if ctx.memberships.find(&user.id, &ws).await?.is_some() {
        return Err(AppError::Conflict("user is already a member".into()));
    }
    let membership = Membership {
        id: MembershipId::new(ctx.ids.new_object_id()),
        user_id: user.id.clone(),
        workspace_id: ws,
        role,
    };
    ctx.memberships.insert(&membership).await?;
    Ok(Response::api_json(
        201,
        &membership_json(&membership, Some(&user)),
    ))
}

async fn update_member(req: &Request, ctx: &Ctx, ws_id: &str, mid: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    let body = json_body(req)?;
    let new_role = Role::parse(&str_field(&body, "role")?)
        .ok_or_else(|| AppError::BadRequest("invalid role".into()))?;
    let mut members = ctx.memberships.list_for_workspace(&ws).await?;
    let idx = members
        .iter()
        .position(|m| m.id.as_str() == mid)
        .ok_or_else(|| AppError::NotFound("membership".into()))?;
    // Guard: never demote the last remaining owner.
    if members[idx].role == Role::Owner
        && new_role != Role::Owner
        && members.iter().filter(|m| m.role == Role::Owner).count() == 1
    {
        return Err(AppError::Conflict(
            "workspace must keep at least one owner".into(),
        ));
    }
    members[idx].role = new_role;
    ctx.memberships.update(&members[idx]).await?;
    let user = ctx.users.find_by_id(&members[idx].user_id).await?;
    Ok(Response::api_json(
        200,
        &membership_json(&members[idx], user.as_ref()),
    ))
}

async fn remove_member(ctx: &Ctx, ws_id: &str, mid: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    let members = ctx.memberships.list_for_workspace(&ws).await?;
    let target = members
        .iter()
        .find(|m| m.id.as_str() == mid)
        .ok_or_else(|| AppError::NotFound("membership".into()))?;
    if target.role == Role::Owner && members.iter().filter(|m| m.role == Role::Owner).count() == 1 {
        return Err(AppError::Conflict(
            "workspace must keep at least one owner".into(),
        ));
    }
    ctx.memberships.delete(&MembershipId::new(mid)).await?;
    Ok(Response::new(204))
}

pub fn user_json(u: &User) -> Value {
    json!({
        "id": u.id.as_str(),
        "username": u.username,
        "displayName": u.display_name,
        "createdAt": u.created_at,
    })
}

fn membership_json(m: &Membership, user: Option<&User>) -> Value {
    json!({
        "id": m.id.as_str(),
        "userId": m.user_id.as_str(),
        "workspaceId": m.workspace_id.as_str(),
        "role": m.role.as_str(),
        "displayName": user.map(|u| u.display_name.clone()),
        "username": user.map(|u| u.username.clone()),
    })
}
