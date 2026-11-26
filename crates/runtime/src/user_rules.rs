//! The calling user's saved tool-permission rules, as the permission policy
//! reads them (ADR-0017, D4).

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use assistant_core::identity::UserId;
use assistant_core::permission::{Rule, UserRuleSource, UserRules};
use assistant_storage::{RuleEffect, ToolPermissionRuleStore};

/// [`UserRuleSource`] over a space's [`ToolPermissionRuleStore`].
///
/// A failed lookup or a rule that no longer parses yields fewer rules, never
/// more, so a storage fault cannot grant anything.
pub struct StoredUserRules {
    store: Arc<dyn ToolPermissionRuleStore>,
}

impl StoredUserRules {
    pub fn new(store: Arc<dyn ToolPermissionRuleStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl UserRuleSource for StoredUserRules {
    async fn rules_for(&self, user_id: &UserId) -> UserRules {
        let rows = match self.store.list_for_user(&user_id.0).await {
            Ok(rows) => rows,
            Err(e) => {
                warn!(error = %e, "could not load saved tool permission rules; none apply");
                return UserRules::default();
            }
        };
        let mut rules = UserRules::default();
        for row in rows {
            match row.rule.parse::<Rule>() {
                Ok(rule) => match row.effect {
                    RuleEffect::Allow => rules.allow.push(rule),
                    RuleEffect::Deny => rules.deny.push(rule),
                },
                Err(e) => warn!(rule = %row.rule, error = %e, "skipping an unparsable saved rule"),
            }
        }
        rules
    }
}

#[cfg(test)]
mod tests {
    use assistant_storage::InMemoryToolPermissionRuleStore;

    use super::*;

    #[tokio::test]
    async fn splits_one_users_rules_into_allow_and_deny() {
        let store = Arc::new(InMemoryToolPermissionRuleStore::new());
        store
            .upsert("alice", RuleEffect::Allow, "bash(git status*)", None)
            .await
            .unwrap();
        store
            .upsert("alice", RuleEffect::Deny, "bash(make clean*)", None)
            .await
            .unwrap();
        store
            .upsert("bob", RuleEffect::Allow, "web-fetch", None)
            .await
            .unwrap();

        let rules = StoredUserRules::new(store)
            .rules_for(&UserId("alice".into()))
            .await;

        let allow: Vec<_> = rules.allow.iter().map(Rule::as_str).collect();
        let deny: Vec<_> = rules.deny.iter().map(Rule::as_str).collect();
        assert_eq!(allow, ["bash(git status*)"]);
        assert_eq!(deny, ["bash(make clean*)"]);
    }

    #[tokio::test]
    async fn a_user_without_rules_gets_none() {
        let rules = StoredUserRules::new(Arc::new(InMemoryToolPermissionRuleStore::new()))
            .rules_for(&UserId("alice".into()))
            .await;
        assert_eq!(rules, UserRules::default());
    }
}
