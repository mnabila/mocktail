//! Auth interface slice: OAuth, sessions, the admin gate, and memberships.

mod access;
mod members;
mod oauth;

pub(crate) use access::{Actor, gate};
pub(crate) use members::{
    add_member_route, list_members_route, me_route, remove_member_route, update_member_route, update_user_route,
};
pub(crate) use oauth::{callback_route, connect_route, login_route, logout_route};
