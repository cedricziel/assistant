//! Identity types for multi-user, multi-org operation.
//!
//! These newtypes provide type-safe identifiers for organizations, spaces,
//! and users. They are used throughout the system wherever identity context
//! is required.

use std::fmt;

use serde::{Deserialize, Serialize};

// -- Newtypes --

/// Unique identifier for an organization (top-level namespace).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrgId(pub String);

/// Unique identifier for a space within an organization.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub String);

/// Unique identifier for a user.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

// -- Display impls --

impl fmt::Display for OrgId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for SpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// -- From<String> / AsRef<str> convenience --

impl From<String> for OrgId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for OrgId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl AsRef<str> for OrgId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for SpaceId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SpaceId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl AsRef<str> for SpaceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// -- Role --

/// Predefined roles that control access within the system.
///
/// Roles form a hierarchy: `OrgAdmin` > `SpaceAdmin` > `Member` > `Viewer`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full control over the organization: manages spaces, users, config.
    OrgAdmin,
    /// Manages resources within a specific space: personas, interfaces, grants.
    SpaceAdmin,
    /// Can create personas (within quota) and use granted personas.
    Member,
    /// Read-only access to granted personas.
    Viewer,
}

impl Role {
    /// Returns the privilege level (higher = more access).
    pub fn privilege_level(&self) -> u8 {
        match self {
            Self::OrgAdmin => 3,
            Self::SpaceAdmin => 2,
            Self::Member => 1,
            Self::Viewer => 0,
        }
    }

    /// Returns `true` if this role has at least the same privilege as `other`.
    pub fn has_at_least(&self, other: &Role) -> bool {
        self.privilege_level() >= other.privilege_level()
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OrgAdmin => f.write_str("org_admin"),
            Self::SpaceAdmin => f.write_str("space_admin"),
            Self::Member => f.write_str("member"),
            Self::Viewer => f.write_str("viewer"),
        }
    }
}

// -- Scope & Permission types --

/// A resource kind that can be acted upon.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Personas,
    Conversations,
    Messages,
    Skills,
    Interfaces,
    Bindings,
    Users,
    Org,
    ApiKeys,
    Spaces,
}

impl std::str::FromStr for ResourceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "personas" => Ok(Self::Personas),
            "conversations" => Ok(Self::Conversations),
            "messages" => Ok(Self::Messages),
            "skills" => Ok(Self::Skills),
            "interfaces" => Ok(Self::Interfaces),
            "bindings" => Ok(Self::Bindings),
            "users" => Ok(Self::Users),
            "org" => Ok(Self::Org),
            "api_keys" => Ok(Self::ApiKeys),
            "spaces" => Ok(Self::Spaces),
            _ => Err(format!("unknown resource kind: {s}")),
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Personas => "personas",
            Self::Conversations => "conversations",
            Self::Messages => "messages",
            Self::Skills => "skills",
            Self::Interfaces => "interfaces",
            Self::Bindings => "bindings",
            Self::Users => "users",
            Self::Org => "org",
            Self::ApiKeys => "api_keys",
            Self::Spaces => "spaces",
        };
        f.write_str(s)
    }
}

/// An action that can be performed on a resource.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
    Manage,
}

impl std::str::FromStr for Action {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "delete" => Ok(Self::Delete),
            "execute" => Ok(Self::Execute),
            "manage" => Ok(Self::Manage),
            _ => Err(format!("unknown action: {s}")),
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Delete => "delete",
            Self::Execute => "execute",
            Self::Manage => "manage",
        };
        f.write_str(s)
    }
}

/// A scope grants permission to perform an action on a resource kind,
/// optionally restricted to specific resource IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub resource: ResourceKind,
    pub action: Action,
    /// When `Some`, only the listed resource IDs are accessible.
    /// `None` means all resources of this kind.
    pub resource_ids: Option<Vec<String>>,
}

impl Scope {
    /// Create a scope for all resources of the given kind and action.
    pub fn new(resource: ResourceKind, action: Action) -> Self {
        Self {
            resource,
            action,
            resource_ids: None,
        }
    }

    /// Create a scope restricted to specific resource IDs.
    pub fn restricted(resource: ResourceKind, action: Action, ids: Vec<String>) -> Self {
        Self {
            resource,
            action,
            resource_ids: Some(ids),
        }
    }

    /// If this scope restricts access to specific resource IDs, returns them.
    ///
    /// Callers should use this to filter listing results when `covers()`
    /// returns `false` due to a missing `target_id`.
    pub fn visible_ids(&self) -> Option<&[String]> {
        self.resource_ids.as_deref()
    }

    /// Returns `true` if this scope covers the given resource, action, and
    /// optional target ID.
    pub fn covers(
        &self,
        resource: &ResourceKind,
        action: &Action,
        target_id: Option<&str>,
    ) -> bool {
        if self.resource != *resource || self.action != *action {
            return false;
        }
        match (&self.resource_ids, target_id) {
            // Unrestricted scope covers everything.
            (None, _) => true,
            // Restricted scope but no target → deny (caller must use
            // `visible_ids()` to filter listings).
            (Some(_), None) => false,
            // Restricted scope with target → must be in the list.
            (Some(ids), Some(id)) => ids.iter().any(|s| s == id),
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.resource, self.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_privilege_ordering() {
        assert!(Role::OrgAdmin.has_at_least(&Role::SpaceAdmin));
        assert!(Role::SpaceAdmin.has_at_least(&Role::Member));
        assert!(Role::Member.has_at_least(&Role::Viewer));
        assert!(!Role::Viewer.has_at_least(&Role::Member));
        assert!(Role::OrgAdmin.has_at_least(&Role::OrgAdmin));
    }

    #[test]
    fn scope_covers_unrestricted() {
        let scope = Scope::new(ResourceKind::Personas, Action::Read);
        assert!(scope.covers(&ResourceKind::Personas, &Action::Read, None));
        assert!(scope.covers(&ResourceKind::Personas, &Action::Read, Some("p1")));
        assert!(!scope.covers(&ResourceKind::Personas, &Action::Write, None));
        assert!(!scope.covers(&ResourceKind::Conversations, &Action::Read, None));
    }

    #[test]
    fn scope_covers_restricted() {
        let scope = Scope::restricted(
            ResourceKind::Personas,
            Action::Read,
            vec!["p1".into(), "p2".into()],
        );
        assert!(scope.covers(&ResourceKind::Personas, &Action::Read, Some("p1")));
        assert!(scope.covers(&ResourceKind::Personas, &Action::Read, Some("p2")));
        assert!(!scope.covers(&ResourceKind::Personas, &Action::Read, Some("p3")));
        // Listing (no target) denied — callers must use visible_ids() to filter.
        assert!(!scope.covers(&ResourceKind::Personas, &Action::Read, None));
        // visible_ids() returns the restriction set.
        assert_eq!(
            scope.visible_ids(),
            Some(vec!["p1".to_string(), "p2".to_string()].as_slice()),
            "restricted scope should expose allowed IDs"
        );
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::OrgAdmin.to_string(), "org_admin");
        assert_eq!(Role::Viewer.to_string(), "viewer");
    }

    #[test]
    fn scope_display() {
        let scope = Scope::new(ResourceKind::Messages, Action::Execute);
        assert_eq!(scope.to_string(), "messages:execute");
    }

    #[test]
    fn newtype_from_and_display() {
        let org = OrgId::from("acme");
        assert_eq!(org.to_string(), "acme");
        assert_eq!(org.as_ref(), "acme");

        let space = SpaceId::from("engineering".to_owned());
        assert_eq!(space.to_string(), "engineering");

        let user = UserId::from("alice");
        assert_eq!(user.as_ref(), "alice");
    }
}
