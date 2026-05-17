//! Skill refinement proposals — the `/review` workflow.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// A single skill refinement proposal row.
#[derive(Debug, Clone)]
pub struct SkillRefinement {
    pub id: Uuid,
    pub target_skill: String,
    pub proposed_skill_md: String,
    pub rationale: String,
    pub status: RefinementStatus,
    pub review_note: Option<String>,
    /// The skill body before this refinement was applied (for revert-on-regression).
    pub previous_skill_md: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefinementStatus {
    Pending,
    Accepted,
    Rejected,
    /// Auto-applied refinement confirmed as non-regressing after the revert window.
    Confirmed,
    /// Auto-applied refinement reverted due to regression detected after apply.
    Reverted,
}

impl RefinementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefinementStatus::Pending => "pending",
            RefinementStatus::Accepted => "accepted",
            RefinementStatus::Rejected => "rejected",
            RefinementStatus::Confirmed => "confirmed",
            RefinementStatus::Reverted => "reverted",
        }
    }
}

impl std::fmt::Display for RefinementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn parse_status(s: &str) -> RefinementStatus {
    match s {
        "accepted" => RefinementStatus::Accepted,
        "rejected" => RefinementStatus::Rejected,
        "confirmed" => RefinementStatus::Confirmed,
        "reverted" => RefinementStatus::Reverted,
        _ => RefinementStatus::Pending,
    }
}

/// SQLite-backed store for skill refinement proposals.
pub struct RefinementsStore {
    pool: SqlitePool,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl RefinementsStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Insert a new pending refinement proposal.
    pub async fn insert(
        &self,
        target_skill: &str,
        proposed_skill_md: &str,
        rationale: &str,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = self.clock.now();

        sqlx::query(
            "INSERT INTO skill_refinements \
                (id, target_skill, proposed_skill_md, rationale, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5)",
        )
        .bind(&id_str)
        .bind(target_skill)
        .bind(proposed_skill_md)
        .bind(rationale)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// List all refinement proposals with a given status.
    pub async fn list_by_status(&self, status: &RefinementStatus) -> Result<Vec<SkillRefinement>> {
        let rows = sqlx::query(
            "SELECT id, target_skill, proposed_skill_md, rationale, status, \
                    review_note, previous_skill_md, created_at, reviewed_at \
             FROM skill_refinements \
             WHERE status = ?1 \
             ORDER BY created_at ASC",
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let raw_id: String = r.get("id");
                let status_str: String = r.get("status");
                Ok(SkillRefinement {
                    id: Uuid::parse_str(&raw_id)?,
                    target_skill: r.get("target_skill"),
                    proposed_skill_md: r.get("proposed_skill_md"),
                    rationale: r.get("rationale"),
                    status: parse_status(&status_str),
                    review_note: r.get("review_note"),
                    previous_skill_md: r.get("previous_skill_md"),
                    created_at: r.get("created_at"),
                    reviewed_at: r.get("reviewed_at"),
                })
            })
            .collect()
    }

    /// Accept or reject a refinement proposal.
    pub async fn review(&self, id: Uuid, accepted: bool, note: Option<&str>) -> Result<()> {
        let status = if accepted { "accepted" } else { "rejected" };
        let id_str = id.to_string();
        let now = self.clock.now();

        sqlx::query(
            "UPDATE skill_refinements \
             SET status = ?1, review_note = ?2, reviewed_at = ?3 \
             WHERE id = ?4",
        )
        .bind(status)
        .bind(note)
        .bind(now)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a refinement with the previous skill body stored for rollback.
    pub async fn insert_with_previous(
        &self,
        target_skill: &str,
        proposed_skill_md: &str,
        rationale: &str,
        previous_skill_md: &str,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = self.clock.now();

        sqlx::query(
            "INSERT INTO skill_refinements \
                (id, target_skill, proposed_skill_md, rationale, status, previous_skill_md, created_at) \
             VALUES (?1, ?2, ?3, ?4, 'accepted', ?5, ?6)",
        )
        .bind(&id_str)
        .bind(target_skill)
        .bind(proposed_skill_md)
        .bind(rationale)
        .bind(previous_skill_md)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Update the status of a refinement (used by revert/confirm logic).
    pub async fn set_status(&self, id: Uuid, status: &RefinementStatus) -> Result<()> {
        let id_str = id.to_string();
        let now = self.clock.now();

        sqlx::query(
            "UPDATE skill_refinements \
             SET status = ?1, reviewed_at = ?2 \
             WHERE id = ?3",
        )
        .bind(status.as_str())
        .bind(now)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
