//! Per-turn context that providers may attach to their requests.
//!
//! The orchestrator scopes each turn with [`LlmCallContext::scope`]. Providers
//! read it with [`LlmCallContext::current`] to send routing hints such as a
//! session ID. The value is a task-local, so it reaches every LLM call awaited
//! inside the turn's future but not work moved to another task with
//! `tokio::spawn`.

use std::future::Future;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::identity::{OrgId, UserId};
use crate::types::conversation::TurnIdentity;

tokio::task_local! {
    static CURRENT: LlmCallContext;
}

/// Who an LLM call is made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmCallContext {
    pub conversation_id: Uuid,
    /// [`user_hash`] of the user who started the turn, if known.
    pub user_hash: Option<String>,
}

impl LlmCallContext {
    pub fn for_turn(conversation_id: Uuid, identity: &TurnIdentity) -> Self {
        Self {
            conversation_id,
            user_hash: identity
                .user_id
                .as_ref()
                .map(|user| user_hash(identity.org_id.as_ref(), user)),
        }
    }

    /// Run `fut` with `self` as the current context.
    pub async fn scope<F: Future>(self, fut: F) -> F::Output {
        CURRENT.scope(self, fut).await
    }

    /// The context of the turn being run, or `None` outside any turn.
    pub fn current() -> Option<Self> {
        CURRENT.try_with(Clone::clone).ok()
    }
}

/// A stable identifier for `user` that cannot be turned back into the user ID
/// or linked to their email. User IDs are random (`usr_<uuid v4>`), so an
/// unkeyed hash is enough; the org is part of the input so one user ID never
/// hashes the same in two orgs.
pub fn user_hash(org: Option<&OrgId>, user: &UserId) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"assistant-llm-user-v1\0");
    hasher.update(org.map_or("", |o| o.0.as_str()).as_bytes());
    hasher.update(b"\0");
    hasher.update(user.0.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(org: &str, user: &str) -> TurnIdentity {
        TurnIdentity {
            user_id: Some(UserId::from(user)),
            org_id: Some(OrgId(org.to_string())),
            space_id: None,
        }
    }

    #[tokio::test]
    async fn current_is_none_outside_a_scope() {
        assert_eq!(LlmCallContext::current(), None);
    }

    #[tokio::test]
    async fn scope_makes_the_context_current() {
        let conversation_id = Uuid::new_v4();
        let ctx = LlmCallContext::for_turn(conversation_id, &identity("org_1", "usr_a"));
        let seen = ctx.clone().scope(async { LlmCallContext::current() }).await;
        assert_eq!(seen, Some(ctx));
        assert_eq!(
            LlmCallContext::current(),
            None,
            "the scope ends with the future"
        );
    }

    #[tokio::test]
    async fn inner_scope_wins() {
        let outer = LlmCallContext::for_turn(Uuid::new_v4(), &TurnIdentity::default());
        let inner = LlmCallContext::for_turn(Uuid::new_v4(), &TurnIdentity::default());
        let seen = outer
            .scope(inner.clone().scope(async { LlmCallContext::current() }))
            .await;
        assert_eq!(seen, Some(inner));
    }

    #[test]
    fn turn_without_a_user_has_no_hash() {
        let ctx = LlmCallContext::for_turn(Uuid::new_v4(), &TurnIdentity::default());
        assert_eq!(ctx.user_hash, None);
    }

    #[test]
    fn user_hash_is_stable_opaque_and_org_scoped() {
        let org_a = OrgId("org_a".into());
        let org_b = OrgId("org_b".into());
        let user = UserId::from("usr_7d1c");

        let hash = user_hash(Some(&org_a), &user);
        assert_eq!(hash, user_hash(Some(&org_a), &user), "stable across calls");
        assert_eq!(hash.len(), 32, "32 hex characters");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "{hash}");
        assert!(!hash.contains("usr_7d1c"), "must not embed the raw ID");
        assert_ne!(
            hash,
            user_hash(Some(&org_b), &user),
            "the same user ID in another org hashes differently"
        );
        assert_eq!(
            LlmCallContext::for_turn(Uuid::nil(), &identity("org_a", "usr_7d1c")).user_hash,
            Some(hash),
            "for_turn uses user_hash"
        );
    }
}
