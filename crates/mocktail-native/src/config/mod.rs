//! Operator config: built-in defaults < YAML file < CLI `-o` overrides.
//! No environment-variable layer.
#![allow(non_snake_case)] // config keys are camelCase to match the YAML schema

pub mod oauth;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Config file auto-loaded from the working directory when no `--config` is given.
pub const DEFAULT_CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub sqlite: SqliteConfig,
    pub auth: AuthConfig,
    pub otel: OtelConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OtelConfig {
    /// Master switch; false = file logging only, no exporters.
    pub enabled: bool,
    /// OTLP/gRPC collector address for traces.
    pub endpoint: String,
    /// True for in-cluster gRPC without TLS.
    pub insecure: bool,
    /// Parent-based trace sampling ratio (0..1).
    pub sample_ratio: f64,
    /// deployment.environment resource attribute.
    pub environment: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        OtelConfig {
            enabled: false,
            endpoint: "localhost:4317".to_string(),
            insecure: true,
            sample_ratio: 1.0,
            environment: "development".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address the HTTP server binds, e.g. `0.0.0.0:8787`.
    pub bind: String,
    /// Apex/base domain reserved for the admin surface.
    pub adminHost: String,
    /// Public base URL used to derive OAuth redirect URIs.
    pub publicBaseUrl: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    /// HMAC secret for session + OAuth-state tokens. Required once any OAuth
    /// provider is configured.
    pub sessionSecret: String,
    /// Session lifetime in hours.
    pub sessionTtlHours: u32,
    /// Emit the `Secure` cookie attribute. Disable only for plain-HTTP local dev.
    pub secureCookies: Option<bool>,
    pub google: Option<ProviderCreds>,
    pub github: Option<ProviderCreds>,
    pub gitlab: Option<ProviderCreds>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderCreds {
    pub clientId: String,
    pub clientSecret: String,
    pub redirectUrl: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SqliteConfig {
    /// Path to the SQLite database file (created if absent).
    pub path: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        SqliteConfig {
            path: "mocktail.db".to_string(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind: "0.0.0.0:8787".to_string(),
            adminHost: "localhost".to_string(),
            publicBaseUrl: "http://localhost:8787".to_string(),
        }
    }
}

/// Load config: start from the YAML file (or `{}` if none), apply dotted-key
/// CLI overrides, then deserialize into the typed `Config` and validate.
pub fn load(config_path: Option<&str>, overrides: &[String]) -> Result<Config> {
    let mut root: serde_yaml::Value = match config_path {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading config file {path}"))?;
            serde_yaml::from_str(&text).with_context(|| format!("parsing YAML {path}"))?
        }
        None => serde_yaml::Value::Mapping(Default::default()),
    };

    for ov in overrides {
        let (key, value) = ov
            .split_once('=')
            .with_context(|| format!("override {ov:?} must be key=value"))?;
        apply_override(&mut root, key, value)?;
    }

    let config: Config =
        serde_yaml::from_value(root).context("config does not match the expected schema")?;
    validate(&config)?;
    Ok(config)
}

/// Set a dotted key (`server.adminHost`) in the YAML value tree.
fn apply_override(root: &mut serde_yaml::Value, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    set_path(root, &parts, value)
}

fn set_path(node: &mut serde_yaml::Value, parts: &[&str], value: &str) -> Result<()> {
    use serde_yaml::Value;
    if !node.is_mapping() {
        *node = Value::Mapping(Default::default());
    }
    let map = node
        .as_mapping_mut()
        .context("override path crosses a non-mapping node")?;
    let head = parts[0];
    let entry = map
        .entry(Value::String(head.to_string()))
        .or_insert(Value::Mapping(Default::default()));
    if parts.len() == 1 {
        // Parse scalar so numbers/bools override with the right type.
        *entry = serde_yaml::from_str(value).unwrap_or(Value::String(value.to_string()));
        Ok(())
    } else {
        set_path(entry, &parts[1..], value)
    }
}

fn validate(c: &Config) -> Result<()> {
    if c.server.bind.trim().is_empty() {
        bail!("server.bind must not be empty");
    }
    if c.server.adminHost.trim().is_empty() {
        bail!("server.adminHost must not be empty");
    }
    if c.sqlite.path.trim().is_empty() {
        bail!("sqlite.path must not be empty");
    }
    // A configured provider is useless without a session secret to sign with.
    let any_provider =
        c.auth.google.is_some() || c.auth.github.is_some() || c.auth.gitlab.is_some();
    if any_provider && c.auth.sessionSecret.trim().is_empty() {
        bail!("auth.sessionSecret is required when any OAuth provider is configured");
    }
    Ok(())
}

impl Config {
    /// Session lifetime in milliseconds (defaults to 7 days).
    pub fn session_ttl_ms(&self) -> i64 {
        let hours = if self.auth.sessionTtlHours == 0 {
            24 * 7
        } else {
            self.auth.sessionTtlHours
        };
        hours as i64 * 60 * 60 * 1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        let c = load(None, &[]).unwrap();
        assert_eq!(c.server.bind, "0.0.0.0:8787");
    }

    #[test]
    fn cli_override_nested() {
        let c = load(
            None,
            &[
                "server.adminHost=app.example.com".to_string(),
                "sqlite.path=/data/prod.db".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(c.server.adminHost, "app.example.com");
        assert_eq!(c.sqlite.path, "/data/prod.db");
    }
}
