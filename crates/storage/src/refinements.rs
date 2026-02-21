//! Skill refinement proposals — the `/review` workflow.

use anyhow::Result;
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
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefinementStatus {
    Pending,
    Accepted,
    Rejected,
}

impl RefinementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefinementStatus::Pending => "pending",
            RefinementStatus::Accepted => "accepted",
            RefinementStatus::Rejected => "rejected",
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
        _ => RefinementStatus::Pending,
    }
}

/// SQLite-backed store for skill refinement proposals.
pub struct RefinementsStore {
    pool: SqlitePool,
}

impl RefinementsStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
        let now = Utc::now();

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
                    review_note, created_at, reviewed_at \
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
        let now = Utc::now();

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
}
