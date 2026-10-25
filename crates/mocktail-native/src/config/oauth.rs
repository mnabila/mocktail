//! Build the OAuth provider registry from operator config. The per-provider
//! endpoints, scopes, and userinfo field names are baked in; the operator only
//! supplies client id/secret.

use super::{AuthConfig, ProviderCreds};
use mocktail_core::modules::auth::domain::OAuthProvider;
use std::collections::HashMap;

/// Static provider template: everything except the operator credentials.
struct Template {
    name: &'static str,
    auth_url: &'static str,
    token_url: &'static str,
    userinfo_url: &'static str,
    scopes: &'static [&'static str],
    subject_field: &'static str,
    email_field: &'static str,
}

const GOOGLE: Template = Template {
    name: "google",
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo",
    scopes: &["openid", "email", "profile"],
    subject_field: "sub",
    email_field: "email",
};

const GITHUB: Template = Template {
    name: "github",
    auth_url: "https://github.com/login/oauth/authorize",
    token_url: "https://github.com/login/oauth/access_token",
    userinfo_url: "https://api.github.com/user",
    scopes: &["read:user", "user:email"],
    // GitHub's numeric `id` is the stable subject; `email` may be null if private.
    subject_field: "id",
    email_field: "email",
};

const GITLAB: Template = Template {
    name: "gitlab",
    auth_url: "https://gitlab.com/oauth/authorize",
    token_url: "https://gitlab.com/oauth/token",
    userinfo_url: "https://gitlab.com/oauth/userinfo",
    scopes: &["openid", "email"],
    subject_field: "sub",
    email_field: "email",
};

fn make(template: &Template, creds: &ProviderCreds) -> OAuthProvider {
    OAuthProvider {
        name: template.name.to_string(),
        client_id: creds.clientId.clone(),
        client_secret: creds.clientSecret.clone(),
        redirect_url: creds.redirectUrl.clone(),
        auth_url: template.auth_url.to_string(),
        token_url: template.token_url.to_string(),
        userinfo_url: template.userinfo_url.to_string(),
        scopes: template.scopes.iter().map(|s| s.to_string()).collect(),
        subject_field: template.subject_field.to_string(),
        email_field: template.email_field.to_string(),
    }
}

/// Assemble the enabled providers. A provider is enabled only when its
/// credentials are present.
pub fn build_providers(auth: &AuthConfig) -> HashMap<String, OAuthProvider> {
    let mut map = HashMap::new();
    for (template, creds) in [
        (&GOOGLE, &auth.google),
        (&GITHUB, &auth.github),
        (&GITLAB, &auth.gitlab),
    ] {
        if let Some(creds) = creds {
            map.insert(template.name.to_string(), make(template, creds));
        }
    }
    map
}
