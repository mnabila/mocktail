//! Auth domain: roles, identities, users, memberships, OAuth provider config,
//! and signed session / OAuth-state tokens.

mod oauth;
mod role;
mod token;
mod user;

pub use oauth::{OAuthMode, OAuthProvider, OAuthState, urlencode};
pub use role::Role;
pub use token::{
    SessionClaims, TokenError, constant_time_eq, decode_session, decode_state, encode_session,
    encode_state, new_session,
};
pub use user::{Identity, Membership, User};
