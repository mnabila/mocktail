//! Auth use cases: repository ports, the OAuth login/connect flow, and RBAC
//! checks.

mod cookie;
mod login;
mod oauth;
mod ports;
mod rbac;

pub use cookie::cookie;
pub use login::{complete_login, connect_provider};
pub use oauth::{CallbackOutcome, STATE_TTL_MS, begin_oauth, complete_oauth_callback};
pub use ports::{MembershipRepository, UserRepository};
pub use rbac::require_role;
