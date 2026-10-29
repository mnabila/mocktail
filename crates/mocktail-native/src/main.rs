//! Native entry point and composition root. Wires the SQLite store into a
//! shared `Ctx` and serves the axum router with Tokio.

mod config;
mod infrastructure;
mod interfaces;
mod otel;

use anyhow::{Context, Result};
use clap::Parser;
use infrastructure::runtime::{
    HmacSigner, LocalRateLimiter, ReqwestHttpClient, SystemClock, TokioTimer, UuidIdGenerator,
};
use infrastructure::sqlite::SqliteExecutor;
use mocktail_core::app::ctx::{AuthConfig, Ctx};
use mocktail_core::modules::auth::application::{MembershipRepository, UserRepository};
use mocktail_core::modules::collection::application::CollectionRepository;
use mocktail_core::modules::folder::application::FolderRepository;
use mocktail_core::modules::mock::application::MockRepository;
use mocktail_core::modules::request_log::application::RequestLogRepository;
use mocktail_core::modules::workspace::application::WorkspaceRepository;
use mocktail_infra_sqlite::SqliteRepositories;
use std::sync::Arc;

/// mocktail: self-hostable API mocking service (native/Docker target).
#[derive(Parser, Debug)]
#[command(name = "mocktail", version)]
struct Args {
    /// Path to the YAML config file. Defaults to ./config.yaml if present.
    #[arg(short = 'c', long = "config")]
    config: Option<String>,

    /// Override a dotted config key, repeatable: `-o server.adminHost=app.example.com`.
    #[arg(short = 'o', long = "option", value_name = "key=value")]
    option: Vec<String>,
}

use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> Result<()> {
    let args = Args::parse();
    
    // When no --config is passed, auto-pick ./config.yaml if it exists.
    let config_path = args.config.or_else(|| {
        std::path::Path::new(config::DEFAULT_CONFIG_FILE)
            .exists()
            .then(|| config::DEFAULT_CONFIG_FILE.to_string())
    });
    if let Some(path) = &config_path {
        tracing::info!(%path, "loading config file");
    } else {
        tracing::info!("no config file; using built-in defaults + CLI overrides");
    }
    let cfg = config::load(config_path.as_deref(), &args.option)?;

    let (otel_handle, tracing_layer) = otel::init(
        "mocktail",
        &cfg.otel,
    );

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    
    if let Some(layer) = tracing_layer {
        tracing_subscriber::registry()
            .with(layer)
            .with(filter)
            .with(fmt::layer())
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .init();
    }

    tracing::info!("Starting Mocktail native server...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    rt.block_on(run(cfg, otel_handle))
}

async fn run(cfg: config::Config, otel_handle: otel::OtelHandle) -> Result<()> {
    let providers = config::oauth::build_providers(&cfg.auth);
    if providers.is_empty() {
        tracing::warn!("no OAuth providers configured; the admin API will reject all sign-ins");
    } else {
        let names: Vec<&str> = providers.keys().map(String::as_str).collect();
        tracing::info!(?names, "OAuth providers enabled");
    }
    let auth = Arc::new(AuthConfig {
        providers,
        public_base_url: cfg.server.publicBaseUrl.clone(),
        session_ttl_ms: cfg.session_ttl_ms(),
        secure_cookies: cfg.auth.secureCookies.unwrap_or(true),
    });

    let parts = Parts {
        apex_host: cfg.server.adminHost.to_ascii_lowercase(),
        session_secret: cfg.auth.sessionSecret.clone(),
        auth,
    };

    // SQLite store (file-backed D1 repository layer); the only native backend.
    tracing::info!(path = %cfg.sqlite.path, "opening SQLite store");
    let exec = SqliteExecutor::connect(&cfg.sqlite.path)
        .await
        .context("opening SQLite store")?;
    let repos = Arc::new(SqliteRepositories::new(Arc::new(exec)));
    repos
        .ensure_schema()
        .await
        .context("creating SQLite schema")?;
    let ctx = Arc::new(assemble(repos, parts));

    interfaces::http::serve(&cfg.server.bind, ctx, otel_handle).await
}

/// Runtime pieces shared by both store backends.
struct Parts {
    apex_host: String,
    session_secret: String,
    auth: Arc<AuthConfig>,
}

/// Wire the SQLite/D1 repository bundle into a `Ctx`.
fn assemble<R>(repos: Arc<R>, parts: Parts) -> Ctx
where
    R: WorkspaceRepository
        + CollectionRepository
        + FolderRepository
        + MockRepository
        + RequestLogRepository
        + UserRepository
        + MembershipRepository
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
        rate_limiter: Arc::new(LocalRateLimiter::default()),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidIdGenerator),
        http: Arc::new(ReqwestHttpClient::new()),
        signer: Arc::new(HmacSigner::new(parts.session_secret)),
        timer: Arc::new(TokioTimer),
        auth: parts.auth,
    }
}
