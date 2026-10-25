//! Collection interface slice: admin CRUD and CORS policy mapping.

use crate::http::auth::gate;
use crate::shared::extract::AxumRequest;
use crate::shared::json::{json_body, parse_slug_or_derive, str_field};
use crate::shared::response::finish_admin;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::collection::application as coll_app;
use mocktail_core::modules::collection::domain::{
    AllowOrigins, Collection, CorsPolicy, Source, normalize_base_path,
};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::{CollectionId, Slug, WorkspaceId};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) async fn list_collections_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(list_collections(&ctx, &ws_id).await)
}

pub(crate) async fn create_collection_route(
    State(ctx): State<Arc<Ctx>>,
    Path(ws_id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(create_collection(&req, &ctx, &ws_id).await)
}

pub(crate) async fn get_collection_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(get_collection(&ctx, &id).await)
}

pub(crate) async fn patch_collection_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(patch_collection(&req, &ctx, &id).await)
}

pub(crate) async fn delete_collection_route(
    State(ctx): State<Arc<Ctx>>,
    Path(id): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    let actor = match gate(&req, &ctx).await {
        Ok(actor) => actor,
        Err(e) => return finish_admin(Err(e)),
    };
    let _ = actor;
    finish_admin(delete_collection(&ctx, &id).await)
}

async fn list_collections(ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    let items: Vec<Value> = ctx
        .collections
        .list(&ws)
        .await?
        .iter()
        .map(collection_json)
        .collect();
    Ok(Response::api_json(200, &json!({ "items": items })))
}

async fn create_collection(req: &Request, ctx: &Ctx, ws_id: &str) -> AppResult<Response> {
    let ws = WorkspaceId::new(ws_id);
    // Validate the workspace exists so collections never dangle.
    if ctx.workspaces.find_by_id(&ws).await?.is_none() {
        return Err(AppError::NotFound(format!("workspace {ws_id}")));
    }
    let body = json_body(req)?;
    let name = str_field(&body, "name")?;
    let slug = parse_slug_or_derive(&body, &name)?;
    let source = match body.get("source").and_then(Value::as_str) {
        None | Some("manual") => Source::Manual,
        Some("openapi") => Source::OpenApi,
        Some(other) => return Err(AppError::BadRequest(format!("unknown source {other:?}"))),
    };
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let base_path = body
        .get("basePath")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let cors_policy = match body.get("cors") {
        Some(v) if !v.is_null() => Some(parse_cors(v)?),
        _ => None,
    };
    let coll = coll_app::create_collection(
        ctx.collections.as_ref(),
        CollectionId::new(ctx.ids.new_object_id()),
        ws,
        coll_app::NewCollection {
            name,
            slug,
            description,
            source,
            base_path,
            cors_policy,
        },
        ctx.clock.now_millis(),
    )
    .await?;
    Ok(Response::api_json(201, &collection_json(&coll)))
}

async fn get_collection(ctx: &Ctx, id: &str) -> AppResult<Response> {
    let coll = coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(id)).await?;
    Ok(Response::api_json(200, &collection_json(&coll)))
}

async fn patch_collection(req: &Request, ctx: &Ctx, id: &str) -> AppResult<Response> {
    let mut coll =
        coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(id)).await?;
    let body = json_body(req)?;
    if let Some(name) = body.get("name").and_then(Value::as_str) {
        coll.name = name.to_string();
    }
    if let Some(slug) = body.get("slug").and_then(Value::as_str) {
        coll.slug = Slug::parse(slug).map_err(|e| AppError::Domain(e.into()))?;
    }
    if let Some(desc) = body.get("description").and_then(Value::as_str) {
        coll.description = desc.to_string();
    }
    if let Some(bp) = body.get("basePath").and_then(Value::as_str) {
        coll.base_path = normalize_base_path(bp);
    }
    if let Some(cors) = body.get("cors").filter(|c| !c.is_null()) {
        coll.cors_policy = parse_cors(cors)?;
    }
    let updated = coll_app::update_collection(ctx.collections.as_ref(), coll).await?;
    Ok(Response::api_json(200, &collection_json(&updated)))
}

async fn delete_collection(ctx: &Ctx, id: &str) -> AppResult<Response> {
    coll_app::delete_collection(ctx.collections.as_ref(), &CollectionId::new(id)).await?;
    Ok(Response::new(204))
}

fn collection_json(c: &Collection) -> Value {
    json!({
        "id": c.id.as_str(),
        "workspaceId": c.workspace_id.as_str(),
        "name": c.name,
        "slug": c.slug.as_str(),
        "description": c.description,
        "source": c.source.as_str(),
        "basePath": c.base_path,
        "mountPath": c.mount_path(),
        "version": c.version,
        "cors": cors_json(&c.cors_policy),
        "createdAt": c.created_at,
    })
}

fn cors_json(p: &CorsPolicy) -> Value {
    let origins = match &p.allow_origins {
        AllowOrigins::Any => json!("*"),
        AllowOrigins::List(list) => json!(list),
    };
    json!({
        "allowOrigins": origins,
        "allowMethods": p.allow_methods,
        "allowHeaders": p.allow_headers,
        "exposeHeaders": p.expose_headers,
        "allowCredentials": p.allow_credentials,
        "maxAgeSecs": p.max_age_secs,
    })
}

fn parse_cors(v: &Value) -> AppResult<CorsPolicy> {
    let default = CorsPolicy::default();
    let allow_origins = match v.get("allowOrigins") {
        Some(Value::String(s)) if s == "*" => AllowOrigins::Any,
        Some(Value::Array(arr)) => AllowOrigins::List(
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
        ),
        None => default.allow_origins,
        _ => {
            return Err(AppError::BadRequest(
                "allowOrigins must be \"*\" or a list".into(),
            ));
        }
    };
    let str_list = |key: &str, fallback: Vec<String>| -> Vec<String> {
        match v.get(key) {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            _ => fallback,
        }
    };
    Ok(CorsPolicy {
        allow_origins,
        allow_methods: str_list("allowMethods", default.allow_methods),
        allow_headers: str_list("allowHeaders", default.allow_headers),
        expose_headers: str_list("exposeHeaders", default.expose_headers),
        allow_credentials: v
            .get("allowCredentials")
            .and_then(Value::as_bool)
            .unwrap_or(default.allow_credentials),
        max_age_secs: v
            .get("maxAgeSecs")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(default.max_age_secs),
    })
}
