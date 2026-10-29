//! OAuth provider configuration and in-progress flow state.

use crate::shared::ports::UnixMillis;
use serde::{Deserialize, Serialize};

/// Operator-supplied OAuth/OIDC provider config. A provider
/// is only offered when its credentials are present.
#[derive(Clone, Debug)]
pub struct OAuthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: Vec<String>,
    /// JSON pointer / dotted path into the userinfo response for the subject.
    pub subject_field: String,
    pub email_field: String,
}

impl OAuthProvider {
    /// Build the provider authorize URL with a signed `state`.
    pub fn authorize_url(&self, redirect_uri: &str, state: &str) -> String {
        let actual_redirect = self.redirect_url.as_deref().unwrap_or(redirect_uri);
        let scope = self.scopes.join(" ");
        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.auth_url,
            urlencode(&self.client_id),
            urlencode(actual_redirect),
            urlencode(&scope),
            urlencode(state),
        )
    }
}

/// Minimal percent-encoding for query parameter values.
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// What an in-progress OAuth flow intends to do on callback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum OAuthMode {
    /// Sign in or create an account.
    Login,
    /// Link this identity to an already-signed-in user.
    Connect { user_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthState {
    pub nonce: String,
    pub provider: String,
    #[serde(flatten)]
    pub mode: OAuthMode,
    pub exp: UnixMillis,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_encodes() {
        let p = OAuthProvider {
            name: "google".into(),
            client_id: "cid".into(),
            client_secret: "sec".into(),
            redirect_url: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
            scopes: vec!["openid".into(), "email".into()],
            subject_field: "sub".into(),
            email_field: "email".into(),
        };
        let url = p.authorize_url("https://mocktail.com/auth/google/callback", "st8");
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("scope=openid%20email"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fmocktail.com%2Fauth%2Fgoogle%2Fcallback"));
        assert!(url.contains("state=st8"));
    }
}
