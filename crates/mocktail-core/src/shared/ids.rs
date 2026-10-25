//! Id newtypes and the slug value object.

use serde::{Deserialize, Serialize};
use std::fmt;

/// An ObjectId-compatible identifier, stored as its 24-char hex string.
///
/// Core treats ids as opaque strings; infra maps them to/from a native
/// `ObjectId` at the persistence boundary. Generation goes through the
/// `IdGenerator` port so core stays deterministic.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub String);

impl Id {
    pub fn new(s: impl Into<String>) -> Self {
        Id(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

macro_rules! typed_id {
    ($name:ident) => {
        /// A strongly-typed id wrapper so distinct aggregates cannot be mixed.
        #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Id);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                $name(Id::new(s))
            }
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
            pub fn id(&self) -> &Id {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }
    };
}

typed_id!(WorkspaceId);
typed_id!(CollectionId);
typed_id!(FolderId);
typed_id!(MockId);
typed_id!(UserId);
typed_id!(MembershipId);

/// Reserved first-path / apex labels that can never be a slug.
pub const RESERVED_SLUGS: &[&str] = &["__admin", "portal", "auth", "api", "www"];

/// A validated DNS-label-safe slug: lowercase alphanumeric and hyphens, not
/// leading/trailing/doubled hyphen, 1..=63 chars, and never a reserved word.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slug(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SlugError {
    #[error("slug must not be empty")]
    Empty,
    #[error("slug too long (max 63 chars)")]
    TooLong,
    #[error("slug contains invalid character {0:?} (lowercase a-z, 0-9, hyphen only)")]
    InvalidChar(char),
    #[error("slug must not start or end with a hyphen")]
    EdgeHyphen,
    #[error("slug must not contain a double hyphen")]
    DoubleHyphen,
    #[error("{0:?} is a reserved slug")]
    Reserved(String),
}

impl Slug {
    pub fn parse(raw: &str) -> Result<Self, SlugError> {
        if raw.is_empty() {
            return Err(SlugError::Empty);
        }
        if raw.len() > 63 {
            return Err(SlugError::TooLong);
        }
        for c in raw.chars() {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(SlugError::InvalidChar(c));
            }
        }
        if raw.starts_with('-') || raw.ends_with('-') {
            return Err(SlugError::EdgeHyphen);
        }
        if raw.contains("--") {
            return Err(SlugError::DoubleHyphen);
        }
        if RESERVED_SLUGS.contains(&raw) {
            return Err(SlugError::Reserved(raw.to_string()));
        }
        Ok(Slug(raw.to_string()))
    }

    /// Derive a slug from a free-text name by lowercasing, replacing runs of
    /// non-alphanumeric with a single hyphen, and trimming edge hyphens.
    pub fn slugify(name: &str) -> Result<Self, SlugError> {
        let mut out = String::new();
        let mut prev_hyphen = false;
        for c in name.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_lowercase());
                prev_hyphen = false;
            } else if !prev_hyphen {
                out.push('-');
                prev_hyphen = true;
            }
        }
        let trimmed = out.trim_matches('-');
        Slug::parse(trimmed)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Slug({})", self.0)
    }
}

impl TryFrom<String> for Slug {
    type Error = SlugError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Slug::parse(&value)
    }
}

impl From<Slug> for String {
    fn from(s: Slug) -> String {
        s.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slugs() {
        for s in ["todo", "api-v1", "a", "team-42"] {
            assert!(Slug::parse(s).is_ok(), "{s} should be valid");
        }
    }

    #[test]
    fn invalid_slugs() {
        assert_eq!(Slug::parse(""), Err(SlugError::Empty));
        assert_eq!(Slug::parse("-x"), Err(SlugError::EdgeHyphen));
        assert_eq!(Slug::parse("x-"), Err(SlugError::EdgeHyphen));
        assert_eq!(Slug::parse("a--b"), Err(SlugError::DoubleHyphen));
        assert_eq!(Slug::parse("Up"), Err(SlugError::InvalidChar('U')));
        assert_eq!(Slug::parse("a_b"), Err(SlugError::InvalidChar('_')));
        assert_eq!(
            Slug::parse("portal"),
            Err(SlugError::Reserved("portal".into()))
        );
    }

    #[test]
    fn slugify_names() {
        assert_eq!(Slug::slugify("Todo API").unwrap().as_str(), "todo-api");
        assert_eq!(
            Slug::slugify("  Hello, World! ").unwrap().as_str(),
            "hello-world"
        );
        assert_eq!(Slug::slugify("a___b").unwrap().as_str(), "a-b");
    }
}
