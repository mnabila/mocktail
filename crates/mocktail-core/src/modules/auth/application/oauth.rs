//! OAuth login/connect callback flow.

use crate::modules::auth::domain::{
    OAuthMode, OAuthProvider, OAuthState, decode_state, encode_state,
};
use crate::shared::error::{AppError, AppResult};
use crate::shared::ports::{HttpClient, IdGenerator, Signer};
use serde_json::Value;

/// Default OAuth-state lifetime (10 minutes).
pub const STATE_TTL_MS: i64 = 10 * 60 * 1000;

/// Begin an OAuth flow: returns (authorize_url, signed_state). The caller sets
/// `signed_state` as a short-lived cookie and redirects to `authorize_url`.
pub fn begin_oauth(
    provider: &OAuthProvider,
    redirect_uri: &str,
    mode: OAuthMode,
    ids: &dyn IdGenerator,
    signer: &dyn Signer,
    now: i64,
) -> (String, String) {
    let state = OAuthState {
        nonce: ids.new_request_id(),
        provider: provider.name.clone(),
        mode,
        exp: now + STATE_TTL_MS,
    };
    let token = encode_state(signer, &state);
    let url = provider.authorize_url(redirect_uri, &token);
    (url, token)
}

/// The identity resolved from a completed OAuth callback.
pub struct CallbackOutcome {
    pub provider: String,
    pub subject: String,
    pub email: Option<String>,
    pub mode: OAuthMode,
}

/// Validate the callback (CSRF double-submit + signed state), exchange the code
/// for a token, and fetch the provider's userinfo.
#[allow(clippy::too_many_arguments)]
pub async fn complete_oauth_callback(
    provider: &OAuthProvider,
    http: &dyn HttpClient,
    signer: &dyn Signer,
    now: i64,
    cookie_state: Option<&str>,
    returned_state: &str,
    code: &str,
    redirect_uri: &str,
) -> AppResult<CallbackOutcome> {
    // CSRF: the state echoed back by the provider must equal the cookie we set.
    match cookie_state {
        Some(c) if c == returned_state => {}
        _ => return Err(AppError::Unauthorized("oauth state mismatch".into())),
    }
    let state = decode_state(signer, returned_state, now)
        .map_err(|e| AppError::Unauthorized(format!("invalid oauth state: {e}")))?;
    if state.provider != provider.name {
        return Err(AppError::Unauthorized("oauth provider mismatch".into()));
    }

    // Exchange the authorization code for an access token.
    let form = vec![
        ("grant_type".to_string(), "authorization_code".to_string()),
        ("code".to_string(), code.to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("client_id".to_string(), provider.client_id.clone()),
        ("client_secret".to_string(), provider.client_secret.clone()),
    ];
    let token_resp = http
        .post_form(
            &provider.token_url,
            &form,
            &[("Accept".to_string(), "application/json".to_string())],
        )
        .await
        .map_err(|e| AppError::Internal(format!("token exchange failed: {e}")))?;
    if token_resp.status >= 400 {
        return Err(AppError::Unauthorized("token exchange rejected".into()));
    }
    let token_json: Value = serde_json::from_slice(&token_resp.body)
        .map_err(|e| AppError::Internal(format!("token response not JSON: {e}")))?;
    let access_token = token_json
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Unauthorized("no access_token in response".into()))?;

    // Fetch userinfo with the bearer token.
    let userinfo_resp = http
        .get(
            &provider.userinfo_url,
            &[
                (
                    "Authorization".to_string(),
                    format!("Bearer {access_token}"),
                ),
                ("Accept".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), "mocktail".to_string()),
            ],
        )
        .await
        .map_err(|e| AppError::Internal(format!("userinfo failed: {e}")))?;
    if userinfo_resp.status >= 400 {
        return Err(AppError::Unauthorized("userinfo rejected".into()));
    }
    let userinfo: Value = serde_json::from_slice(&userinfo_resp.body)
        .map_err(|e| AppError::Internal(format!("userinfo not JSON: {e}")))?;

    let subject = extract_field(&userinfo, &provider.subject_field)
        .ok_or_else(|| AppError::Unauthorized("userinfo missing subject".into()))?;
    let email = extract_field(&userinfo, &provider.email_field);

    Ok(CallbackOutcome {
        provider: provider.name.clone(),
        subject,
        email,
        mode: state.mode,
    })
}

/// Read a value at a dotted/pointer path, coercing scalars to a string.
fn extract_field(v: &Value, path: &str) -> Option<String> {
    let pointer = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path.replace('.', "/"))
    };
    match v.pointer(&pointer)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
