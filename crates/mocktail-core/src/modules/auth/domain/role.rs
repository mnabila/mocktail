//! Workspace roles and privilege ordering.

/// Workspace role. Ordered: viewer < editor < owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(Role::Owner),
            "editor" => Some(Role::Editor),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
    /// Higher rank = more privilege. Used by `allows`.
    pub fn rank(&self) -> u8 {
        match self {
            Role::Viewer => 1,
            Role::Editor => 2,
            Role::Owner => 3,
        }
    }
    /// Does holding `self` satisfy a minimum required role?
    pub fn allows(&self, required: Role) -> bool {
        self.rank() >= required.rank()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_ordering() {
        assert!(Role::Owner.allows(Role::Editor));
        assert!(Role::Editor.allows(Role::Viewer));
        assert!(!Role::Viewer.allows(Role::Editor));
        assert!(Role::Viewer.allows(Role::Viewer));
    }
}
