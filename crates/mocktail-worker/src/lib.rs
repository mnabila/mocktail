mod runtime;
mod otel;

use mocktail_core::app::ctx::{AuthConfig, Ctx};
use mocktail_core::modules::auth::domain::OAuthProvider;
use mocktail_infra_sqlite::SqliteRepositories;
use runtime::{
    D1Exec, FetchHttpClient, HmacSigner, NoopTimer, UuidIds, WorkerClock, WorkerRateLimiter,
};
use std::collections::HashMap;
use std::sync::Arc;
use tower_service::Service;
use worker::*;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    let start_time = js_sys::Date::now();
    let ctx = match build_ctx(&env) {
        Ok(c) => c,
        Err(e) => {
            let resp = Response::error(format!("config error: {e}"), 500)?;
            return Ok(resp.into());
        }
    };

    let host = req
        .uri()
        .host()
        .map(str::to_ascii_lowercase)
        .or_else(|| {
            req.headers()
                .get("host")
                .and_then(|value| value.to_str().ok())
                .map(str::to_ascii_lowercase)
        })
        .unwrap_or_default();
    if host == ctx.apex_host && !is_control_path(req.uri().path()) {
        let assets = match env.assets("ASSETS") {
            Ok(a) => a,
            Err(e) => {
                let resp = Response::error(format!("assets binding 'ASSETS' missing: {e}"), 500)?;
                return Ok(resp.into());
            }
        };
        let forwarded: worker::Request = req.try_into()?;
        let asset_resp = assets.fetch_request(forwarded).await?;
        let (parts, body) = asset_resp.into_parts();
        return Ok(axum::http::Response::from_parts(
            parts,
            axum::body::Body::new(body),
        ));
    }

    let mut app = mocktail_http::router(Arc::new(ctx));
    let response = app
        .call(req)
        .await
        .unwrap_or_else(|_| unreachable!("axum router is infallible"));
    
    // OTel Push: Send trace to Alloy before returning response.
    let otel_endpoint = env.var("OTEL_ENDPOINT").unwrap_or_else(|| "http://localhost:4318/v1/traces".to_string());
    let otel_env = env.var("OTEL_ENVIRONMENT").unwrap_or_else(|| "development".to_string());
    let otel = otel::OtelWorker::new(otel_endpoint, otel_env);
    
    let duration_ms = (js_sys::Date::now() - start_time) as u64;
    let path = "http.request"; // Simplified for worker
    
    // We don't await this if we want to keep latency low, but in Workers, 
    // we must await or use ctx.waitUntil.
    let _ = otel.push_trace(path, duration_ms).await;

    Ok(response)
}

/// Admin API and OAuth routes are handled by the router; everything else on
/// the apex host is the SvelteKit portal.
fn is_control_path(path: &str) -> bool {
    path == "/__admin"
        || path.starts_with("/__admin/")
        || path == "/auth"
        || path.starts_with("/auth/")
}

fn build_ctx(env: &Env) -> std::result::Result<Ctx, String> {
    let get = |k: &str| -> Option<String> {
        env.secret(k)
            .map(|s| s.to_string())
            .or_else(|_| env.var(k).map(|v| v.to_string()))
            .ok()
    };
    let require = |k: &str| -> std::result::Result<String, String> {
        get(k).ok_or_else(|| format!("missing config {k}"))
    };

    let http: Arc<dyn mocktail_core::shared::ports::HttpClient> = Arc::new(FetchHttpClient);
    let apex_host = require("APEX_HOST")?.to_ascii_lowercase();
    let public_base_url = require("PUBLIC_BASE_URL")?;
    let session_secret = require("SESSION_SECRET")?;
    let auth = Arc::new(AuthConfig {
        providers: build_providers(&get),
        public_base_url,
        session_ttl_ms: 7 * 24 * 60 * 60 * 1000,
        secure_cookies: true,
    });

    let parts = Parts {
        apex_host,
        http: http.clone(),
        signer: Arc::new(HmacSigner::new(session_secret)),
        auth,
    };

    // D1 (fully-Cloudflare SQLite) is the only store.
    let db = env
        .d1("DB")
        .map_err(|e| format!("D1 binding 'DB' missing: {e}"))?;
    let exec: Arc<dyn mocktail_infra_sqlite::SqlExecutor> = Arc::new(D1Exec::new(db));
    Ok(assemble(Arc::new(SqliteRepositories::new(exec)), parts))
}

/// Runtime pieces shared by both store backends.
struct Parts {
    apex_host: String,
    http: Arc<dyn mocktail_core::shared::ports::HttpClient>,
    signer: Arc<dyn mocktail_core::shared::ports::Signer>,
    auth: Arc<AuthConfig>,
}

/// Wire the D1 repository bundle into a `Ctx`.
fn assemble<R>(repos: Arc<R>, parts: Parts) -> Ctx
where
    R: mocktail_core::modules::workspace::application::WorkspaceRepository
        + mocktail_core::modules::collection::application::CollectionRepository
        + mocktail_core::modules::folder::application::FolderRepository
        + mocktail_core::modules::mock::application::MockRepository
        + mocktail_core::modules::request_log::application::RequestLogRepository
        + mocktail_core::modules::auth::application::UserRepository
        + mocktail_core::modules::auth::application::MembershipRepository
        + 'static,
{
    Ctx {
        apex_host: parts.apex_host,
        workspaces: repos.clone(),
        collections: repos.clone(),
        folders: repos.clone(),
        mocks: repos.clone(),
        request_logs: repos.clone(),
        users: repos.clone(),
        memberships: repos.clone(),
        rate_limiter: Arc::new(WorkerRateLimiter),
        clock: Arc::new(WorkerClock),
        ids: Arc::new(UuidIds),
        http: parts.http,
        signer: parts.signer,
        timer: Arc::new(NoopTimer),
        auth: parts.auth,
    }
}

/// Build the OAuth provider registry from env credentials (parity with the
/// native `config/oauth.rs`).
fn build_providers(get: &dyn Fn(&str) -> Option<String>) -> HashMap<String, OAuthProvider> {
    let mut map = HashMap::new();

    let mut add = |name: &str,
                   id_key: &str,
                   secret_key: &str,
                   auth_url: &str,
                   token_url: &str,
                   userinfo_url: &str,
                   scopes: &[&str],
                   subject_field: &str,
                   email_field: &str| {
        if let (Some(client_id), Some(client_secret)) = (get(id_key), get(secret_key)) {
            map.insert(
                name.to_string(),
                OAuthProvider {
                    name: name.to_string(),
                    client_id,
                    client_secret,
                    auth_url: auth_url.to_string(),
                    token_url: token_url.to_string(),
                    userinfo_url: userinfo_url.to_string(),
                    scopes: scopes.iter().map(|s| s.to_string()).collect(),
                    subject_field: subject_field.to_string(),
                    email_field: email_field.to_string(),
                    redirect_url: None,
                },
            );
        }
    };

    add(
        "google",
        "GOOGLE_CLIENT_ID",
        "GOOGLE_CLIENT_SECRET",
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://oauth2.googleapis.com/token",
        "https://openidconnect.googleapis.com/v1/userinfo",
        &["openid", "email", "profile"],
        "sub",
        "email",
    );
    add(
        "github",
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "https://github.com/login/oauth/authorize",
        "https://github.com/login/oauth/access_token",
        "https://api.github.com/user",
        &["read:user", "user:email"],
        "id",
        "email",
    );
    add(
        "gitlab",
        "GITLAB_CLIENT_ID",
        "GITLAB_CLIENT_SECRET",
        "https://gitlab.com/oauth/authorize",
        "https://gitlab.com/oauth/token",
        "https://gitlab.com/oauth/userinfo",
        &["openid", "email"],
        "sub",
        "email",
    );

    map
}
