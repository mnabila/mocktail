//! Session, CSRF, and role gate shared by protected admin routes.

use crate::http::auth::oauth::current_session;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::auth::application as auth_app;
use mocktail_core::modules::auth::domain::{Role, SessionClaims, User};
use mocktail_core::modules::collection::application as coll_app;
use mocktail_core::modules::mock::application as mock_app;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Method, Request};
use mocktail_core::shared::ids::{CollectionId, MockId, UserId, WorkspaceId};

pub(crate) struct Actor {
    pub(crate) claims: SessionClaims,
    pub(crate) user: User,
}

/// Run the session, CSRF, and RBAC gates shared by every protected route.
pub(crate) async fn gate(req: &Request, ctx: &Ctx) -> AppResult<Actor> {
    let actor = authed(req, ctx).await?;
    enforce_csrf(req, &actor)?;
    let segs: Vec<&str> = req
        .path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    authorize(ctx, &actor, &segs, req.method).await?;
    Ok(actor)
}

/// Resolve the authenticated user from the session cookie, or 401.
async fn authed(req: &Request, ctx: &Ctx) -> AppResult<Actor> {
    let claims = current_session(ctx, req)
        .ok_or_else(|| AppError::Unauthorized("sign in required".into()))?;
    let user = ctx
        .users
        .find_by_id(&UserId::new(claims.uid.clone()))
        .await?
        .ok_or_else(|| AppError::Unauthorized("session user no longer exists".into()))?;
    Ok(Actor { claims, user })
}

/// State-changing admin requests must present a matching CSRF token.
fn enforce_csrf(req: &Request, actor: &Actor) -> AppResult<()> {
    let mutating = matches!(
        req.method,
        Method::Post | Method::Put | Method::Patch | Method::Delete
    );
    if !mutating {
        return Ok(());
    }
    match req.headers.get("x-csrf-token") {
        Some(t) if t == actor.claims.csrf => Ok(()),
        _ => Err(AppError::Forbidden("missing or invalid CSRF token".into())),
    }
}

/// Require that the actor has at least `role` in `workspace`.
async fn require_ws(
    ctx: &Ctx,
    actor: &Actor,
    workspace: &WorkspaceId,
    role: Role,
) -> AppResult<()> {
    auth_app::require_role(ctx.memberships.as_ref(), &actor.user.id, workspace, role)
        .await
        .map(|_| ())
}

/// Per-route RBAC, resolving the workspace from the entity when needed.
async fn authorize(ctx: &Ctx, actor: &Actor, segs: &[&str], method: Method) -> AppResult<()> {
    use Method::*;
    match segs {
        // Workspace list/create and /me need only a valid session.
        ["__admin", "me"] | ["__admin", "workspaces"] => Ok(()),

        // Renaming/deleting a workspace is owner-only.
        ["__admin", "workspaces", ws] => {
            require_ws(ctx, actor, &WorkspaceId::new(*ws), Role::Owner).await
        }

        ["__admin", "workspaces", ws, "collections"] => {
            let min = if method == Get {
                Role::Viewer
            } else {
                Role::Editor
            };
            require_ws(ctx, actor, &WorkspaceId::new(*ws), min).await
        }
        ["__admin", "workspaces", ws, "logs"] => {
            require_ws(ctx, actor, &WorkspaceId::new(*ws), Role::Viewer).await
        }
        ["__admin", "workspaces", ws, "members"] | ["__admin", "workspaces", ws, "members", _] => {
            let min = if method == Get {
                Role::Viewer
            } else {
                Role::Owner
            };
            require_ws(ctx, actor, &WorkspaceId::new(*ws), min).await
        }

        ["__admin", "collections", id]
        | ["__admin", "collections", id, "mocks"]
        | ["__admin", "collections", id, "folders"]
        | ["__admin", "collections", id, "folders", _]
        | ["__admin", "collections", id, "export"]
        | ["__admin", "collections", id, "import"]
        | ["__admin", "collections", id, "import", "preview"] => {
            let coll =
                coll_app::get_collection(ctx.collections.as_ref(), &CollectionId::new(*id)).await?;
            // Reads (GET, incl. export) need viewer; mutations + imports editor.
            let min = if method == Get {
                Role::Viewer
            } else {
                Role::Editor
            };
            require_ws(ctx, actor, &coll.workspace_id, min).await
        }

        ["__admin", "mocks", id] => {
            let mock = mock_app::get_mock(ctx.mocks.as_ref(), &MockId::new(*id)).await?;
            let min = if method == Get {
                Role::Viewer
            } else {
                Role::Editor
            };
            require_ws(ctx, actor, &mock.workspace_id, min).await
        }

        // Unknown routes fall through to the handler (which 404s).
        _ => Ok(()),
    }
}
