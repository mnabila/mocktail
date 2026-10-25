//! User accounts, linked identities, and workspace memberships.

use super::role::Role;
use crate::shared::ids::{MembershipId, UserId, WorkspaceId};
use crate::shared::ports::UnixMillis;

/// A linked SSO identity (one per provider login) within a user's account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub provider: String,
    pub subject: String,
    pub email: Option<String>,
}

/// A user account. Multiple identities may be linked, but only explicitly
/// (never auto-merged by email).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub created_at: UnixMillis,
}

impl User {
    pub fn has_identity(&self, _provider: &str, _subject: &str) -> bool {
        // This now requires a repository check because identities are not stored in the User struct
        false
    }
}

/// A user's role within a workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub id: MembershipId,
    pub user_id: UserId,
    pub workspace_id: WorkspaceId,
    pub role: Role,
}
