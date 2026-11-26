//! Turn interactions: the durable record of questions a running turn asks its
//! user (tool approvals, free-form questions).
//!
//! The `interactions` table is the source of truth for authorisation,
//! idempotency, liveness, the pending list and receipts after reload. A turn
//! and the HTTP process that answers it may be different processes, so every
//! state transition is a compare-and-set on the row.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

// -- Types --

/// Lifecycle state of an interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionStatus {
    Pending,
    Resolved,
    Cancelled,
    Expired,
}

impl InteractionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "resolved" => Ok(Self::Resolved),
            "cancelled" => Ok(Self::Cancelled),
            "expired" => Ok(Self::Expired),
            other => bail!("unknown interaction status '{other}'"),
        }
    }
}

/// One row of the `interactions` table.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionRow {
    pub id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub user_id: String,
    pub kind: String,
    pub tool_call_id: Option<String>,
    pub request: Value,
    pub status: InteractionStatus,
    pub response: Option<Value>,
    pub rule_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub waiter_seen_at: DateTime<Utc>,
    pub resolution_emitted: bool,
}

/// Input to [`InteractionStore::create`].
#[derive(Debug, Clone, PartialEq)]
pub struct NewInteraction {
    pub id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub user_id: String,
    pub kind: String,
    pub tool_call_id: Option<String>,
    pub request: Value,
    pub expires_at: DateTime<Utc>,
}

/// Result of [`InteractionStore::resolve_if_pending`].
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveOutcome {
    /// This call won the compare-and-set; the row is the updated one.
    Resolved(InteractionRow),
    /// The row had already left `pending`; the row is its stored outcome.
    AlreadyResolved(InteractionRow),
    /// No row has this id.
    NotFound,
}

// -- Trait --

/// Persistence for turn interactions.
#[async_trait]
pub trait InteractionStore: Send + Sync {
    /// Insert a new `pending` row. Fails when the id already exists.
    async fn create(&self, new: NewInteraction) -> Result<InteractionRow>;

    async fn get(&self, id: &str) -> Result<Option<InteractionRow>>;

    /// Move a `pending` row to the terminal `status`. Exactly one of several
    /// racing callers gets [`ResolveOutcome::Resolved`]; the rest see the
    /// winner's row as [`ResolveOutcome::AlreadyResolved`]. `status` must not
    /// be [`InteractionStatus::Pending`].
    async fn resolve_if_pending(
        &self,
        id: &str,
        status: InteractionStatus,
        response: Option<&Value>,
        rule_id: Option<&str>,
    ) -> Result<ResolveOutcome>;

    /// Rows of a conversation in creation order, optionally of one status.
    async fn list_for_conversation(
        &self,
        conversation_id: &str,
        status: Option<InteractionStatus>,
    ) -> Result<Vec<InteractionRow>>;

    /// Record that a live waiter is still blocked on the row.
    async fn touch_waiter(&self, id: &str) -> Result<()>;

    /// Expire `pending` rows whose `waiter_seen_at` is before `older_than`.
    /// Returns the expired ids in creation order.
    async fn expire_dead(&self, older_than: DateTime<Utc>) -> Result<Vec<String>>;

    /// Cancel every `pending` row of a run. Returns the cancelled ids in
    /// creation order.
    async fn cancel_pending_for_run(&self, run_id: &str) -> Result<Vec<String>>;

    /// Every row of a run, in any status, whose resolution has not yet been
    /// written to the event log (`resolution_emitted = false`).
    async fn list_unemitted_for_run(&self, run_id: &str) -> Result<Vec<InteractionRow>>;

    /// Record that the row's resolution reached the event log.
    async fn mark_emitted(&self, id: &str) -> Result<()>;
}

// -- SQLite --

/// `SELECT <all columns> FROM interactions <tail>` as a `&'static str`.
macro_rules! select_interactions {
    ($tail:literal) => {
        concat!(
            "SELECT id, conversation_id, run_id, user_id, kind, tool_call_id, request, ",
            "status, response, rule_id, created_at, expires_at, resolved_at, ",
            "waiter_seen_at, resolution_emitted FROM interactions ",
            $tail
        )
    };
}

/// Fixed-width UTC text, so timestamps order correctly as strings.
fn ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)
        .with_context(|| format!("invalid timestamp '{s}'"))?
        .with_timezone(&Utc))
}

fn row_from_sqlite(r: &SqliteRow) -> Result<InteractionRow> {
    let request: String = r.try_get("request")?;
    let response: Option<String> = r.try_get("response")?;
    let status: String = r.try_get("status")?;
    let created_at: String = r.try_get("created_at")?;
    let expires_at: String = r.try_get("expires_at")?;
    let resolved_at: Option<String> = r.try_get("resolved_at")?;
    let waiter_seen_at: String = r.try_get("waiter_seen_at")?;
    Ok(InteractionRow {
        id: r.try_get("id")?,
        conversation_id: r.try_get("conversation_id")?,
        run_id: r.try_get("run_id")?,
        user_id: r.try_get("user_id")?,
        kind: r.try_get("kind")?,
        tool_call_id: r.try_get("tool_call_id")?,
        request: serde_json::from_str(&request).context("invalid interaction request JSON")?,
        status: InteractionStatus::parse(&status)?,
        response: response
            .map(|j| serde_json::from_str(&j))
            .transpose()
            .context("invalid interaction response JSON")?,
        rule_id: r.try_get("rule_id")?,
        created_at: parse_ts(&created_at)?,
        expires_at: parse_ts(&expires_at)?,
        resolved_at: resolved_at.as_deref().map(parse_ts).transpose()?,
        waiter_seen_at: parse_ts(&waiter_seen_at)?,
        resolution_emitted: r.try_get("resolution_emitted")?,
    })
}

/// An `UPDATE` that closes every `pending` row matching `$filter`, which binds
/// its argument as `?3` (`?1` is the new status, `?2` the resolution time).
macro_rules! close_pending {
    ($filter:literal) => {
        concat!(
            "UPDATE interactions SET status = ?1, resolved_at = ?2 ",
            "WHERE status = 'pending' AND ",
            $filter,
            " RETURNING id, created_at"
        )
    };
}

/// SQLite-backed [`InteractionStore`].
pub struct SqliteInteractionStore {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqliteInteractionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Run a `close_pending!` statement, binding the new `status`, the
    /// current time and `filter_arg`. Returns the closed ids in creation order.
    async fn close_pending(
        &self,
        sql: &'static str,
        status: InteractionStatus,
        filter_arg: &str,
    ) -> Result<Vec<String>> {
        let mut rows = sqlx::query(sql)
            .bind(status.as_str())
            .bind(ts(self.clock.now()))
            .bind(filter_arg)
            .fetch_all(&self.pool)
            .await?;
        // RETURNING order is unspecified.
        rows.sort_by(|a, b| {
            let key = |r: &SqliteRow| {
                (
                    r.try_get::<String, _>("created_at").unwrap_or_default(),
                    r.try_get::<String, _>("id").unwrap_or_default(),
                )
            };
            key(a).cmp(&key(b))
        });
        rows.iter()
            .map(|r| r.try_get("id").map_err(Into::into))
            .collect()
    }
}

#[async_trait]
impl InteractionStore for SqliteInteractionStore {
    async fn create(&self, new: NewInteraction) -> Result<InteractionRow> {
        let now = self.clock.now();
        sqlx::query(
            "INSERT INTO interactions \
             (id, conversation_id, run_id, user_id, kind, tool_call_id, request, status, \
              created_at, expires_at, waiter_seen_at, resolution_emitted) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?8, 0)",
        )
        .bind(&new.id)
        .bind(&new.conversation_id)
        .bind(&new.run_id)
        .bind(&new.user_id)
        .bind(&new.kind)
        .bind(&new.tool_call_id)
        .bind(new.request.to_string())
        .bind(ts(now))
        .bind(ts(new.expires_at))
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to create interaction \'{}\'", new.id))?;
        self.get(&new.id)
            .await?
            .context("interaction vanished right after insert")
    }

    async fn get(&self, id: &str) -> Result<Option<InteractionRow>> {
        sqlx::query(select_interactions!("WHERE id = ?1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(row_from_sqlite)
            .transpose()
    }

    async fn resolve_if_pending(
        &self,
        id: &str,
        status: InteractionStatus,
        response: Option<&Value>,
        rule_id: Option<&str>,
    ) -> Result<ResolveOutcome> {
        if status == InteractionStatus::Pending {
            bail!("an interaction cannot be resolved to \'pending\'");
        }
        let updated = sqlx::query(
            "UPDATE interactions \
             SET status = ?2, response = ?3, rule_id = ?4, resolved_at = ?5 \
             WHERE id = ?1 AND status = 'pending'",
        )
        .bind(id)
        .bind(status.as_str())
        .bind(response.map(Value::to_string))
        .bind(rule_id)
        .bind(ts(self.clock.now()))
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(match (self.get(id).await?, updated) {
            (Some(row), 1..) => ResolveOutcome::Resolved(row),
            (Some(row), 0) => ResolveOutcome::AlreadyResolved(row),
            (None, _) => ResolveOutcome::NotFound,
        })
    }

    async fn list_for_conversation(
        &self,
        conversation_id: &str,
        status: Option<InteractionStatus>,
    ) -> Result<Vec<InteractionRow>> {
        sqlx::query(select_interactions!(
            "WHERE conversation_id = ?1 AND (?2 IS NULL OR status = ?2) \
             ORDER BY created_at ASC, rowid ASC"
        ))
        .bind(conversation_id)
        .bind(status.map(InteractionStatus::as_str))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(row_from_sqlite)
        .collect()
    }

    async fn touch_waiter(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE interactions SET waiter_seen_at = ?2 WHERE id = ?1")
            .bind(id)
            .bind(ts(self.clock.now()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn expire_dead(&self, older_than: DateTime<Utc>) -> Result<Vec<String>> {
        self.close_pending(
            close_pending!("waiter_seen_at < ?3"),
            InteractionStatus::Expired,
            &ts(older_than),
        )
        .await
    }

    async fn cancel_pending_for_run(&self, run_id: &str) -> Result<Vec<String>> {
        self.close_pending(
            close_pending!("run_id = ?3"),
            InteractionStatus::Cancelled,
            run_id,
        )
        .await
    }

    async fn list_unemitted_for_run(&self, run_id: &str) -> Result<Vec<InteractionRow>> {
        sqlx::query(select_interactions!(
            "WHERE run_id = ?1 AND resolution_emitted = 0 \
             ORDER BY created_at ASC, rowid ASC"
        ))
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(row_from_sqlite)
        .collect()
    }

    async fn mark_emitted(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE interactions SET resolution_emitted = 1 WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// -- In-memory --

/// In-memory [`InteractionStore`] for tests (ADR-0009).
pub struct InMemoryInteractionStore {
    rows: Mutex<Vec<InteractionRow>>,
    clock: Arc<dyn Clock>,
}

impl InMemoryInteractionStore {
    pub fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<InteractionRow>> {
        match self.rows.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    /// Close every `pending` row for which `matches` holds, returning ids in
    /// creation (insertion) order.
    fn close_pending(
        &self,
        status: InteractionStatus,
        matches: impl Fn(&InteractionRow) -> bool,
    ) -> Vec<String> {
        let now = self.clock.now();
        self.lock()
            .iter_mut()
            .filter(|r| r.status == InteractionStatus::Pending && matches(r))
            .map(|r| {
                r.status = status;
                r.resolved_at = Some(now);
                r.id.clone()
            })
            .collect()
    }
}

impl Default for InMemoryInteractionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InteractionStore for InMemoryInteractionStore {
    async fn create(&self, new: NewInteraction) -> Result<InteractionRow> {
        let now = self.clock.now();
        let mut rows = self.lock();
        if rows.iter().any(|r| r.id == new.id) {
            bail!("interaction \'{}\' already exists", new.id);
        }
        let row = InteractionRow {
            id: new.id,
            conversation_id: new.conversation_id,
            run_id: new.run_id,
            user_id: new.user_id,
            kind: new.kind,
            tool_call_id: new.tool_call_id,
            request: new.request,
            status: InteractionStatus::Pending,
            response: None,
            rule_id: None,
            created_at: now,
            expires_at: new.expires_at,
            resolved_at: None,
            waiter_seen_at: now,
            resolution_emitted: false,
        };
        rows.push(row.clone());
        Ok(row)
    }

    async fn get(&self, id: &str) -> Result<Option<InteractionRow>> {
        Ok(self.lock().iter().find(|r| r.id == id).cloned())
    }

    async fn resolve_if_pending(
        &self,
        id: &str,
        status: InteractionStatus,
        response: Option<&Value>,
        rule_id: Option<&str>,
    ) -> Result<ResolveOutcome> {
        if status == InteractionStatus::Pending {
            bail!("an interaction cannot be resolved to \'pending\'");
        }
        let now = self.clock.now();
        let mut rows = self.lock();
        let Some(row) = rows.iter_mut().find(|r| r.id == id) else {
            return Ok(ResolveOutcome::NotFound);
        };
        if row.status != InteractionStatus::Pending {
            return Ok(ResolveOutcome::AlreadyResolved(row.clone()));
        }
        row.status = status;
        row.response = response.cloned();
        row.rule_id = rule_id.map(str::to_string);
        row.resolved_at = Some(now);
        Ok(ResolveOutcome::Resolved(row.clone()))
    }

    async fn list_for_conversation(
        &self,
        conversation_id: &str,
        status: Option<InteractionStatus>,
    ) -> Result<Vec<InteractionRow>> {
        Ok(self
            .lock()
            .iter()
            .filter(|r| r.conversation_id == conversation_id)
            .filter(|r| status.is_none_or(|s| r.status == s))
            .cloned()
            .collect())
    }

    async fn touch_waiter(&self, id: &str) -> Result<()> {
        let now = self.clock.now();
        if let Some(row) = self.lock().iter_mut().find(|r| r.id == id) {
            row.waiter_seen_at = now;
        }
        Ok(())
    }

    async fn expire_dead(&self, older_than: DateTime<Utc>) -> Result<Vec<String>> {
        Ok(self.close_pending(InteractionStatus::Expired, |r| {
            r.waiter_seen_at < older_than
        }))
    }

    async fn cancel_pending_for_run(&self, run_id: &str) -> Result<Vec<String>> {
        Ok(self.close_pending(InteractionStatus::Cancelled, |r| r.run_id == run_id))
    }

    async fn list_unemitted_for_run(&self, run_id: &str) -> Result<Vec<InteractionRow>> {
        Ok(self
            .lock()
            .iter()
            .filter(|r| r.run_id == run_id && !r.resolution_emitted)
            .cloned()
            .collect())
    }

    async fn mark_emitted(&self, id: &str) -> Result<()> {
        if let Some(row) = self.lock().iter_mut().find(|r| r.id == id) {
            row.resolution_emitted = true;
        }
        Ok(())
    }
}
