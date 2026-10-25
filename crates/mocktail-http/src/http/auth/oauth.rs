//! `/auth/*` routes on the apex surface: OAuth login, callback, provider
//! connect, and logout.

use crate::shared::extract::{AxumRequest, to_axum};
use crate::shared::response::finish_public;
use axum::extract::{Path, State};
use axum::response::Response as AxumResponse;
use mocktail_core::app::ctx::Ctx;
use mocktail_core::modules::auth::application as auth_app;
use mocktail_core::modules::auth::domain::{OAuthMode, SessionClaims, decode_session, new_session};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::{Request, Response};
use mocktail_core::shared::ids::UserId;
use std::sync::Arc;

const SESSION_COOKIE: &str = "mocktail_session";
const OAUTH_COOKIE: &str = "mocktail_oauth";

pub(crate) async fn logout_route(State(ctx): State<Arc<Ctx>>) -> AxumResponse {
    to_axum(logout(&ctx))
}

pub(crate) async fn login_route(
    State(ctx): State<Arc<Ctx>>,
    Path(provider): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    finish_public(begin(&ctx, &provider, false, &req))
}

pub(crate) async fn connect_route(
    State(ctx): State<Arc<Ctx>>,
    Path(provider): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    finish_public(begin(&ctx, &provider, true, &req))
}

pub(crate) async fn callback_route(
    State(ctx): State<Arc<Ctx>>,
    Path(provider): Path<String>,
    AxumRequest(req): AxumRequest,
) -> AxumResponse {
    finish_public(callback(&ctx, &provider, &req).await)
}

fn redirect_uri(ctx: &Ctx, provider: &str) -> String {
    format!(
        "{}/auth/{}/callback",
        ctx.auth.public_base_url.trim_end_matches('/'),
        provider
    )
}

fn begin(ctx: &Ctx, provider: &str, connect: bool, req: &Request) -> AppResult<Response> {
    let cfg = ctx
        .auth
        .providers
        .get(provider)
        .ok_or_else(|| AppError::NotFound(format!("provider {provider} not enabled")))?;

    let mode = if connect {
        // Connecting requires a current session; the user is bound into the
        // signed state so the callback links to the right account.
        let claims = current_session(ctx, req)
            .ok_or_else(|| AppError::Unauthorized("sign in before connecting a provider".into()))?;
        OAuthMode::Connect {
            user_id: claims.uid,
        }
    } else {
        OAuthMode::Login
    };

    let (url, state) = auth_app::begin_oauth(
        cfg,
        &redirect_uri(ctx, provider),
        mode,
        ctx.ids.as_ref(),
        ctx.signer.as_ref(),
        ctx.clock.now_millis(),
    );

    Ok(Response::new(302).with_header("location", url).with_header(
        "set-cookie",
        cookie(OAUTH_COOKIE, &state, 600, "/auth", ctx.auth.secure_cookies),
    ))
}

async fn callback(ctx: &Ctx, provider: &str, req: &Request) -> AppResult<Response> {
    let cfg = ctx
        .auth
        .providers
        .get(provider)
        .ok_or_else(|| AppError::NotFound(format!("provider {provider} not enabled")))?;

    if let Some(err) = req.query_first("error") {
        return Err(AppError::Unauthorized(format!("provider error: {err}")));
    }
    let code = req
        .query_first("code")
        .ok_or_else(|| AppError::BadRequest("missing code".into()))?;
    let returned_state = req
        .query_first("state")
        .ok_or_else(|| AppError::BadRequest("missing state".into()))?;
    let cookie_state = auth_app::cookie(req, OAUTH_COOKIE);

    let outcome = auth_app::complete_oauth_callback(
        cfg,
        ctx.http.as_ref(),
        ctx.signer.as_ref(),
        ctx.clock.now_millis(),
        cookie_state.as_deref(),
        returned_state,
        code,
        cfg.redirect_url.as_deref().unwrap_or(&redirect_uri(ctx, provider)),
    )
    .await?;

    match &outcome.mode {
        OAuthMode::Login => {
            let (user, _is_new) = auth_app::complete_login(
                ctx.users.as_ref(),
                ctx.ids.as_ref(),
                ctx.clock.as_ref(),
                &outcome,
            )
            .await?;
            let (token, _claims) = new_session(
                ctx.signer.as_ref(),
                ctx.ids.as_ref(),
                &user.id,
                ctx.clock.now_millis(),
                ctx.auth.session_ttl_ms,
            );
            Ok(Response::new(302)
                .with_header("location", "/")
                .with_header(
                    "set-cookie",
                    cookie(
                        SESSION_COOKIE,
                        &token,
                        ctx.auth.session_ttl_ms / 1000,
                        "/",
                        ctx.auth.secure_cookies,
                    ),
                )
                .with_header(
                    "set-cookie",
                    clear_cookie(OAUTH_COOKIE, "/auth", ctx.auth.secure_cookies),
                ))
        }
        OAuthMode::Connect { user_id } => {
            // The signed state names the account; require a live session for it.
            match current_session(ctx, req) {
                Some(c) if &c.uid == user_id => {}
                _ => return Err(AppError::Unauthorized("connect session mismatch".into())),
            }
            auth_app::connect_provider(ctx.users.as_ref(), &UserId::new(user_id.clone()), &outcome)
                .await?;
            Ok(Response::new(302)
                .with_header("location", "/account")
                .with_header(
                    "set-cookie",
                    clear_cookie(OAUTH_COOKIE, "/auth", ctx.auth.secure_cookies),
                ))
        }
    }
}

fn logout(ctx: &Ctx) -> Response {
    Response::new(302)
        .with_header("location", "/login")
        .with_header(
            "set-cookie",
            clear_cookie(SESSION_COOKIE, "/", ctx.auth.secure_cookies),
        )
}

/// Decode the current session from the request cookie, if valid.
pub(super) fn current_session(ctx: &Ctx, req: &Request) -> Option<SessionClaims> {
    let token = auth_app::cookie(req, SESSION_COOKIE)?;
    decode_session(ctx.signer.as_ref(), &token, ctx.clock.now_millis()).ok()
}

fn cookie(name: &str, value: &str, max_age_secs: i64, path: &str, secure: bool) -> String {
    let mut c =
        format!("{name}={value}; HttpOnly; SameSite=Lax; Path={path}; Max-Age={max_age_secs}");
    if secure {
        c.push_str("; Secure");
    }
    c
}

fn clear_cookie(name: &str, path: &str, secure: bool) -> String {
    cookie(name, "", 0, path, secure)
}
