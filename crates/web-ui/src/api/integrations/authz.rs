//! Who may see and change which integration.
//!
//! Pure functions over the caller's [`AuthContext`], so the rules are
//! testable without a router. Every route goes through them.

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Role, SpaceId};
use assistant_core::integrations::{
    Integration, IntegrationLayer, IntegrationPolicy, SpaceAccess, SpaceAccessMode,
};

/// What a caller may do with one integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Access {
    /// The caller may not know it exists (`404`).
    Hidden,
    View,
    /// An org admin and another user's integration: they may remove it, but
    /// never change it.
    ViewAndDelete,
    Manage,
}

impl Access {
    pub fn can_view(self) -> bool {
        self != Self::Hidden
    }

    pub fn can_delete(self) -> bool {
        matches!(self, Self::ViewAndDelete | Self::Manage)
    }

    pub fn can_manage(self) -> bool {
        self == Self::Manage
    }
}

/// `rows` are the space access rows of `integration` (used for org-layer ones).
pub(super) fn access(ctx: &AuthContext, integration: &Integration, rows: &[SpaceAccess]) -> Access {
    if integration.org_id != ctx.org_id {
        return Access::Hidden;
    }
    match integration.layer {
        IntegrationLayer::Org if ctx.is_org_admin() => Access::Manage,
        IntegrationLayer::Org if offered_in_a_space_of(ctx, rows) => Access::View,
        IntegrationLayer::Org => Access::Hidden,
        IntegrationLayer::Space => match &integration.space_id {
            Some(space) if can_administer_space(ctx, space) => Access::Manage,
            Some(space) if ctx.role_in(space).is_some() => Access::View,
            _ => Access::Hidden,
        },
        IntegrationLayer::User if integration.owner_user_id.as_ref() == Some(&ctx.user_id) => {
            Access::Manage
        }
        IntegrationLayer::User if ctx.is_org_admin() => Access::ViewAndDelete,
        IntegrationLayer::User => Access::Hidden,
    }
}

pub(super) fn can_administer_space(ctx: &AuthContext, space: &SpaceId) -> bool {
    ctx.is_org_admin() || matches!(ctx.role_in(space), Some(Role::SpaceAdmin))
}

/// Whether the caller may install at `layer` (before the org policy).
pub(super) fn can_install(
    ctx: &AuthContext,
    layer: IntegrationLayer,
    space: Option<&SpaceId>,
) -> bool {
    match layer {
        IntegrationLayer::Org => ctx.is_org_admin(),
        IntegrationLayer::Space => space.is_some_and(|space| can_administer_space(ctx, space)),
        IntegrationLayer::User => true,
    }
}

/// The layers the caller may install catalog entry `entry_id` at, now.
pub(super) fn installable_layers(
    ctx: &AuthContext,
    policy: &IntegrationPolicy,
    entry_id: &str,
) -> Vec<IntegrationLayer> {
    let administers_a_space = ctx.is_org_admin()
        || ctx
            .space_roles
            .values()
            .any(|role| matches!(role, Role::SpaceAdmin));
    [
        (IntegrationLayer::Org, ctx.is_org_admin()),
        (IntegrationLayer::Space, administers_a_space),
        (IntegrationLayer::User, true),
    ]
    .into_iter()
    .filter(|(layer, allowed)| *allowed && policy.install_denial(*layer, Some(entry_id)).is_none())
    .map(|(layer, _)| layer)
    .collect()
}

/// Whether an org-layer integration with these rows is offered to `space`,
/// ignoring the space's own opt-out (an opted-out space still sees it, to opt
/// back in).
pub(super) fn offered_in(rows: &[SpaceAccess], space: &SpaceId) -> bool {
    let mut allows = rows
        .iter()
        .filter(|row| row.mode == SpaceAccessMode::Allow)
        .peekable();
    allows.peek().is_none() || allows.any(|row| &row.space_id == space)
}

fn offered_in_a_space_of(ctx: &AuthContext, rows: &[SpaceAccess]) -> bool {
    ctx.space_roles.keys().any(|space| offered_in(rows, space))
}
