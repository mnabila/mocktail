//! Signed session and OAuth-state tokens.

use super::oauth::OAuthState;
use crate::shared::ids::UserId;
use crate::shared::ports::{IdGenerator, Signer, UnixMillis};
use base64::Engine;
use serde::{Deserialize, Serialize};

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionClaims {
    pub uid: String,
    pub exp: UnixMillis,
    /// CSRF token tied to the session.
    pub csrf: String,
}

/// Errors decoding a signed token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("malformed token")]
    Malformed,
    #[error("bad signature")]
    BadSignature,
    #[error("token expired")]
    Expired,
}

/// Encode `payload` as `b64(payload).b64(mac)`, where the MAC covers a
/// domain-separation `label` plus the payload (prevents cross-use of a session
/// token as an OAuth-state token and vice versa).
fn encode_token<T: Serialize>(signer: &dyn Signer, label: &[u8], payload: &T) -> String {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    let mac = signer.sign(&signing_input(label, &body));
    format!("{}.{}", B64.encode(&body), B64.encode(&mac))
}

fn decode_token<T: for<'de> Deserialize<'de>>(
    signer: &dyn Signer,
    label: &[u8],
    token: &str,
) -> Result<T, TokenError> {
    let (body_b64, mac_b64) = token.split_once('.').ok_or(TokenError::Malformed)?;
    let body = B64.decode(body_b64).map_err(|_| TokenError::Malformed)?;
    let mac = B64.decode(mac_b64).map_err(|_| TokenError::Malformed)?;
    let expected = signer.sign(&signing_input(label, &body));
    if !constant_time_eq(&mac, &expected) {
        return Err(TokenError::BadSignature);
    }
    serde_json::from_slice(&body).map_err(|_| TokenError::Malformed)
}

fn signing_input(label: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(label.len() + 1 + body.len());
    v.extend_from_slice(label);
    v.push(b'|');
    v.extend_from_slice(body);
    v
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

const SESSION_LABEL: &[u8] = b"mocktail.session.v1";
const STATE_LABEL: &[u8] = b"mocktail.oauthstate.v1";

pub fn encode_session(signer: &dyn Signer, claims: &SessionClaims) -> String {
    encode_token(signer, SESSION_LABEL, claims)
}

pub fn decode_session(
    signer: &dyn Signer,
    token: &str,
    now: UnixMillis,
) -> Result<SessionClaims, TokenError> {
    let claims: SessionClaims = decode_token(signer, SESSION_LABEL, token)?;
    if claims.exp <= now {
        return Err(TokenError::Expired);
    }
    Ok(claims)
}

pub fn encode_state(signer: &dyn Signer, state: &OAuthState) -> String {
    encode_token(signer, STATE_LABEL, state)
}

pub fn decode_state(
    signer: &dyn Signer,
    token: &str,
    now: UnixMillis,
) -> Result<OAuthState, TokenError> {
    let state: OAuthState = decode_token(signer, STATE_LABEL, token)?;
    if state.exp <= now {
        return Err(TokenError::Expired);
    }
    Ok(state)
}

/// Build a fresh session token for `user_id`, valid for `ttl_ms`.
pub fn new_session(
    signer: &dyn Signer,
    ids: &dyn IdGenerator,
    user_id: &UserId,
    now: UnixMillis,
    ttl_ms: i64,
) -> (String, SessionClaims) {
    let claims = SessionClaims {
        uid: user_id.as_str().to_string(),
        exp: now + ttl_ms,
        csrf: ids.new_request_id(),
    };
    (encode_session(signer, &claims), claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct XorSigner;
    impl Signer for XorSigner {
        fn sign(&self, bytes: &[u8]) -> Vec<u8> {
            bytes.iter().map(|b| b ^ 0x5a).collect()
        }
    }

    #[test]
    fn session_round_trip_and_expiry() {
        let s = XorSigner;
        let claims = SessionClaims {
            uid: "u1".into(),
            exp: 1000,
            csrf: "c".into(),
        };
        let token = encode_session(&s, &claims);
        let decoded = decode_session(&s, &token, 500).unwrap();
        assert_eq!(decoded.uid, "u1");
        assert_eq!(decode_session(&s, &token, 2000), Err(TokenError::Expired));
    }

    #[test]
    fn tampered_token_rejected() {
        let s = XorSigner;
        let claims = SessionClaims {
            uid: "u1".into(),
            exp: 1000,
            csrf: "c".into(),
        };
        let token = encode_session(&s, &claims);
        let mut bad = token.clone();
        bad.insert(0, 'x');
        assert!(decode_session(&s, &bad, 500).is_err());
    }

    #[test]
    fn session_token_not_valid_as_state() {
        // Domain separation: a session token must not verify as an OAuth state.
        let s = XorSigner;
        let claims = SessionClaims {
            uid: "u1".into(),
            exp: 1000,
            csrf: "c".into(),
        };
        let token = encode_session(&s, &claims);
        assert_eq!(decode_state(&s, &token, 500), Err(TokenError::BadSignature));
    }
}
