//! Workflow persistence for graph-style trigger/action automation.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Workflow node kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    Trigger,
    Action,
    Condition,
}

/// A single node in the workflow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub kind: WorkflowNodeKind,
    #[serde(default)]
    pub config: Value,
}

/// Optional edge condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdgeCondition {
    pub key: String,
    pub equals: Value,
}

/// Directed edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub on: Option<String>,
    #[serde(default)]
    pub condition: Option<WorkflowEdgeCondition>,
}

/// Runtime guard rails for loop-enabled workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionLimits {
    pub max_steps: u32,
    pub max_visits_per_node: u32,
}

impl Default for WorkflowExecutionLimits {
    fn default() -> Self {
        Self {
            max_steps: 200,
            max_visits_per_node: 25,
        }
    }
}

/// Versioned graph definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    #[serde(default = "default_graph_version")]
    pub version: u32,
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub execution: WorkflowExecutionLimits,
}

fn default_graph_version() -> u32 {
    1
}

/// Persisted workflow row.
#[derive(Debug, Clone)]
pub struct WorkflowRecord {
    pub id: Uuid,
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub graph: WorkflowGraph,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Persisted inbound webhook endpoint for a workflow trigger.
#[derive(Debug, Clone)]
pub struct WorkflowWebhookEndpoint {
    pub workflow_id: Uuid,
    pub token: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trigger kinds used when recording workflow runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowTriggerKind {
    Manual,
    Webhook,
    Schedule,
    Event,
}

impl WorkflowTriggerKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Webhook => "webhook",
            Self::Schedule => "schedule",
            Self::Event => "event",
        }
    }
}

/// Persisted run record for workflow execution telemetry.
#[derive(Debug, Clone)]
pub struct WorkflowRunRecord {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: String,
    pub trigger_type: String,
    pub trigger_payload: Value,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Step-level event within a run execution.
#[derive(Debug, Clone)]
pub struct WorkflowRunStepRecord {
    pub id: Uuid,
    pub run_id: Uuid,
    pub step_index: i64,
    pub node_id: String,
    pub node_kind: String,
    pub note: Option<String>,
    pub output: Option<Value>,
    pub occurred_at: DateTime<Utc>,
}

/// SQLite-backed store for workflows.
/// Trait-based interface for workflow persistence.
#[async_trait]
pub trait WorkflowStore: Send + Sync {
    async fn create(
        &self,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<Uuid>;

    async fn ensure_webhook_endpoint(&self, workflow_id: Uuid) -> Result<WorkflowWebhookEndpoint>;

    async fn get_webhook_endpoint(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>>;

    async fn rotate_webhook_token(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>>;

    async fn resolve_by_webhook(
        &self,
        workflow_id: Uuid,
        token: &str,
    ) -> Result<Option<WorkflowRecord>>;

    async fn create_run(
        &self,
        workflow_id: Uuid,
        trigger: WorkflowTriggerKind,
        trigger_payload: &Value,
    ) -> Result<Uuid>;

    async fn list_runs(&self, workflow_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunRecord>>;

    async fn get_run(&self, workflow_id: Uuid, run_id: Uuid) -> Result<Option<WorkflowRunRecord>>;

    async fn list_runnable_runs(&self, limit: i64) -> Result<Vec<WorkflowRunRecord>>;

    async fn workflow_for_run(&self, run_id: Uuid) -> Result<Option<WorkflowRecord>>;

    async fn append_run_step(
        &self,
        run_id: Uuid,
        step_index: i64,
        node_id: &str,
        node_kind: &str,
        note: Option<&str>,
        output: Option<&Value>,
    ) -> Result<()>;

    async fn finish_run(
        &self,
        run_id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<bool>;

    async fn list_run_steps(&self, run_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunStepRecord>>;

    async fn list(&self) -> Result<Vec<WorkflowRecord>>;

    async fn get(&self, id: Uuid) -> Result<Option<WorkflowRecord>>;

    async fn update(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<bool>;

    async fn set_active(&self, id: Uuid, active: bool) -> Result<bool>;

    async fn toggle_active(&self, id: Uuid) -> Result<Option<bool>>;

    async fn delete(&self, id: Uuid) -> Result<bool>;
}

pub struct SqliteWorkflowStore {
    pool: SqlitePool,
    agent_id: String,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl SqliteWorkflowStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            agent_id: "default".to_string(),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn for_agent(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation. Tests pass `Arc::new(FakeClock::new(...))`
    /// to assert row-timestamp behavior against virtual time.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

#[async_trait]
impl WorkflowStore for SqliteWorkflowStore {
    /// Insert a new workflow.
    async fn create(
        &self,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<Uuid> {
        validate_workflow_graph(graph)?;

        let id = Uuid::new_v4();
        let now = self.clock.now();
        let graph_json = serde_json::to_string(graph)?;

        sqlx::query(
            "INSERT INTO workflows \
             (id, agent_id, name, description, graph_json, active, max_steps, max_visits_per_node, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        )
        .bind(id.to_string())
        .bind(&self.agent_id)
        .bind(name)
        .bind(description)
        .bind(graph_json)
        .bind(active)
        .bind(graph.execution.max_steps as i64)
        .bind(graph.execution.max_visits_per_node as i64)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Returns webhook endpoint metadata for a workflow, creating one if missing.
    async fn ensure_webhook_endpoint(&self, workflow_id: Uuid) -> Result<WorkflowWebhookEndpoint> {
        if let Some(existing) = self.get_webhook_endpoint(workflow_id).await? {
            return Ok(existing);
        }

        let now = self.clock.now();
        let token = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO workflow_webhook_endpoints
             (workflow_id, token, active, created_at, updated_at)
             VALUES (?1, ?2, 1, ?3, ?3)",
        )
        .bind(workflow_id.to_string())
        .bind(&token)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_webhook_endpoint(workflow_id)
            .await?
            .context("workflow webhook endpoint missing after insert")
    }

    /// Returns inbound webhook endpoint metadata for a workflow.
    async fn get_webhook_endpoint(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>> {
        let row = sqlx::query(
            "SELECT workflow_id, token, active, created_at, updated_at
             FROM workflow_webhook_endpoints
             WHERE workflow_id = ?1",
        )
        .bind(workflow_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(parse_webhook_row).transpose()
    }

    /// Rotates the webhook token for a workflow endpoint.
    async fn rotate_webhook_token(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>> {
        let now = self.clock.now();
        let token = Uuid::new_v4().to_string();
        let result = sqlx::query(
            "UPDATE workflow_webhook_endpoints
             SET token = ?1, updated_at = ?2
             WHERE workflow_id = ?3",
        )
        .bind(&token)
        .bind(now)
        .bind(workflow_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_webhook_endpoint(workflow_id).await
    }

    /// Resolves a workflow by webhook credentials.
    async fn resolve_by_webhook(
        &self,
        workflow_id: Uuid,
        token: &str,
    ) -> Result<Option<WorkflowRecord>> {
        let allowed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM workflow_webhook_endpoints we
             JOIN workflows w ON w.id = we.workflow_id
             WHERE we.workflow_id = ?1
               AND we.token = ?2
               AND we.active = 1
               AND w.active = 1
               AND w.agent_id = ?3",
        )
        .bind(workflow_id.to_string())
        .bind(token)
        .bind(&self.agent_id)
        .fetch_one(&self.pool)
        .await?;

        if allowed == 0 {
            return Ok(None);
        }

        self.get(workflow_id).await
    }

    /// Records a workflow run triggered by manual, webhook, schedule, or events.
    async fn create_run(
        &self,
        workflow_id: Uuid,
        trigger: WorkflowTriggerKind,
        trigger_payload: &Value,
    ) -> Result<Uuid> {
        let workflow = self.get(workflow_id).await?.context("workflow not found")?;
        let run_id = Uuid::new_v4();
        let payload_json = serde_json::to_string(trigger_payload)?;

        sqlx::query(
            "INSERT INTO workflow_runs
             (id, workflow_id, agent_id, trigger_type, trigger_payload_json, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6)",
        )
        .bind(run_id.to_string())
        .bind(workflow_id.to_string())
        .bind(&workflow.agent_id)
        .bind(trigger.as_str())
        .bind(payload_json)
        .bind(self.clock.now())
        .execute(&self.pool)
        .await?;

        Ok(run_id)
    }

    /// Lists recent runs for a workflow.
    async fn list_runs(&self, workflow_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunRecord>> {
        let rows = sqlx::query(
            "SELECT id, workflow_id, agent_id, trigger_type, trigger_payload_json, status, started_at, finished_at, error_message
             FROM workflow_runs
             WHERE workflow_id = ?1 AND agent_id = ?2
             ORDER BY started_at DESC
             LIMIT ?3",
        )
        .bind(workflow_id.to_string())
        .bind(&self.agent_id)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_run_row).collect()
    }

    /// Fetch one workflow run by workflow and run id.
    async fn get_run(&self, workflow_id: Uuid, run_id: Uuid) -> Result<Option<WorkflowRunRecord>> {
        let row = sqlx::query(
            "SELECT id, workflow_id, agent_id, trigger_type, trigger_payload_json, status, started_at, finished_at, error_message
             FROM workflow_runs
             WHERE workflow_id = ?1 AND id = ?2 AND agent_id = ?3",
        )
        .bind(workflow_id.to_string())
        .bind(run_id.to_string())
        .bind(&self.agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(parse_run_row).transpose()
    }

    /// Lists globally runnable workflow runs.
    async fn list_runnable_runs(&self, limit: i64) -> Result<Vec<WorkflowRunRecord>> {
        let rows = sqlx::query(
            "SELECT id, workflow_id, agent_id, trigger_type, trigger_payload_json, status, started_at, finished_at, error_message
             FROM workflow_runs
             WHERE status = 'running' AND finished_at IS NULL
             ORDER BY started_at ASC
             LIMIT ?1",
        )
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_run_row).collect()
    }

    /// Load the workflow definition associated with a run.
    async fn workflow_for_run(&self, run_id: Uuid) -> Result<Option<WorkflowRecord>> {
        let row = sqlx::query(
            "SELECT w.id, w.agent_id, w.name, w.description, w.graph_json, w.active, w.created_at, w.updated_at
             FROM workflow_runs wr
             JOIN workflows w ON w.id = wr.workflow_id
             WHERE wr.id = ?1",
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(parse_row).transpose()
    }

    /// Append one execution step record for a workflow run.
    async fn append_run_step(
        &self,
        run_id: Uuid,
        step_index: i64,
        node_id: &str,
        node_kind: &str,
        note: Option<&str>,
        output: Option<&Value>,
    ) -> Result<()> {
        let output_json = output.map(serde_json::to_string).transpose()?;
        sqlx::query(
            "INSERT INTO workflow_run_steps
             (id, run_id, step_index, node_id, node_kind, note, output_json, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(run_id.to_string())
        .bind(step_index)
        .bind(node_id)
        .bind(node_kind)
        .bind(note)
        .bind(output_json)
        .bind(self.clock.now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark a run finished with final status and optional error message.
    async fn finish_run(
        &self,
        run_id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE workflow_runs
             SET status = ?1, finished_at = ?2, error_message = ?3
             WHERE id = ?4 AND finished_at IS NULL",
        )
        .bind(status)
        .bind(self.clock.now())
        .bind(error_message)
        .bind(run_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// List recorded step events for a run.
    async fn list_run_steps(&self, run_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunStepRecord>> {
        let rows = sqlx::query(
            "SELECT id, run_id, step_index, node_id, node_kind, note, output_json, occurred_at
             FROM workflow_run_steps
             WHERE run_id = ?1
             ORDER BY step_index ASC
             LIMIT ?2",
        )
        .bind(run_id.to_string())
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_step_row).collect()
    }

    /// List workflows for this agent.
    async fn list(&self) -> Result<Vec<WorkflowRecord>> {
        let rows = sqlx::query(
            "SELECT id, agent_id, name, description, graph_json, active, created_at, updated_at \
             FROM workflows WHERE agent_id = ?1 ORDER BY created_at DESC",
        )
        .bind(&self.agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }

    /// Fetch one workflow by ID.
    async fn get(&self, id: Uuid) -> Result<Option<WorkflowRecord>> {
        let row = sqlx::query(
            "SELECT id, agent_id, name, description, graph_json, active, created_at, updated_at \
             FROM workflows WHERE id = ?1 AND agent_id = ?2",
        )
        .bind(id.to_string())
        .bind(&self.agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(parse_row).transpose()
    }

    /// Update mutable workflow fields.
    async fn update(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<bool> {
        validate_workflow_graph(graph)?;

        let now = self.clock.now();
        let graph_json = serde_json::to_string(graph)?;
        let result = sqlx::query(
            "UPDATE workflows
             SET name = ?1,
                 description = ?2,
                 graph_json = ?3,
                 active = ?4,
                 max_steps = ?5,
                 max_visits_per_node = ?6,
                 updated_at = ?7
             WHERE id = ?8 AND agent_id = ?9",
        )
        .bind(name)
        .bind(description)
        .bind(graph_json)
        .bind(active)
        .bind(graph.execution.max_steps as i64)
        .bind(graph.execution.max_visits_per_node as i64)
        .bind(now)
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Toggle active status.
    async fn set_active(&self, id: Uuid, active: bool) -> Result<bool> {
        let now = self.clock.now();
        let result = sqlx::query(
            "UPDATE workflows
             SET active = ?1, updated_at = ?2
             WHERE id = ?3 AND agent_id = ?4",
        )
        .bind(active)
        .bind(now)
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Atomically toggle active status and return the new state.
    async fn toggle_active(&self, id: Uuid) -> Result<Option<bool>> {
        let now = self.clock.now();
        let result = sqlx::query(
            "UPDATE workflows
             SET active = CASE WHEN active = 0 THEN 1 ELSE 0 END,
                 updated_at = ?1
             WHERE id = ?2 AND agent_id = ?3",
        )
        .bind(now)
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let active_int: i64 =
            sqlx::query_scalar("SELECT active FROM workflows WHERE id = ?1 AND agent_id = ?2")
                .bind(id.to_string())
                .bind(&self.agent_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(Some(active_int != 0))
    }

    /// Delete a workflow.
    async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM workflows WHERE id = ?1 AND agent_id = ?2")
            .bind(id.to_string())
            .bind(&self.agent_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Validates a graph definition.
///
/// Cycles are explicitly allowed (option 2) and controlled at execution time
/// via `max_steps` and `max_visits_per_node`.
pub fn validate_workflow_graph(graph: &WorkflowGraph) -> Result<()> {
    if graph.nodes.is_empty() {
        bail!("graph must contain at least one node");
    }

    if graph.execution.max_steps == 0 {
        bail!("execution.max_steps must be greater than zero");
    }
    if graph.execution.max_steps > 10_000 {
        bail!("execution.max_steps exceeds hard limit (10000)");
    }

    if graph.execution.max_visits_per_node == 0 {
        bail!("execution.max_visits_per_node must be greater than zero");
    }
    if graph.execution.max_visits_per_node > 1_000 {
        bail!("execution.max_visits_per_node exceeds hard limit (1000)");
    }

    let mut node_ids = HashSet::new();
    let mut trigger_count = 0usize;
    let mut action_count = 0usize;

    for node in &graph.nodes {
        if node.id.trim().is_empty() {
            bail!("node id cannot be empty");
        }

        if !node_ids.insert(node.id.clone()) {
            bail!(
                "duplicate node id '{}': each node id must be unique",
                node.id
            );
        }

        match node.kind {
            WorkflowNodeKind::Trigger => trigger_count += 1,
            WorkflowNodeKind::Action => action_count += 1,
            WorkflowNodeKind::Condition => {}
        }

        validate_node_contract(node)?;
    }

    if trigger_count == 0 {
        bail!("graph must contain at least one trigger node");
    }
    if action_count == 0 {
        bail!("graph must contain at least one action node");
    }

    for edge in &graph.edges {
        if !node_ids.contains(&edge.from) {
            bail!("edge source '{}' does not exist", edge.from);
        }
        if !node_ids.contains(&edge.to) {
            bail!("edge target '{}' does not exist", edge.to);
        }

        if let Some(label) = edge.on.as_deref() {
            let source = graph
                .nodes
                .iter()
                .find(|n| n.id == edge.from)
                .context("edge source vanished during validation")?;
            let allowed = allowed_edge_outcomes(source)?;
            if !allowed
                .iter()
                .any(|allowed_label| allowed_label.eq_ignore_ascii_case(label))
            {
                bail!(
                    "edge label '{}' is invalid for source node '{}' (allowed: {})",
                    label,
                    source.id,
                    allowed.join(", ")
                );
            }
        }
    }

    Ok(())
}

fn parse_row(row: sqlx::sqlite::SqliteRow) -> Result<WorkflowRecord> {
    let id_raw: String = row.try_get("id")?;
    let graph_json: String = row.try_get("graph_json")?;
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .with_context(|| format!("failed to parse graph_json for workflow {id_raw}"))?;
    let active_int: i64 = row.try_get("active")?;

    Ok(WorkflowRecord {
        id: Uuid::parse_str(&id_raw)?,
        agent_id: row.try_get("agent_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        graph,
        active: active_int != 0,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_webhook_row(row: sqlx::sqlite::SqliteRow) -> Result<WorkflowWebhookEndpoint> {
    let workflow_id_raw: String = row.try_get("workflow_id")?;
    let active_int: i64 = row.try_get("active")?;
    Ok(WorkflowWebhookEndpoint {
        workflow_id: Uuid::parse_str(&workflow_id_raw)?,
        token: row.try_get("token")?,
        active: active_int != 0,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_run_row(row: sqlx::sqlite::SqliteRow) -> Result<WorkflowRunRecord> {
    let id_raw: String = row.try_get("id")?;
    let workflow_id_raw: String = row.try_get("workflow_id")?;
    let payload_json: String = row.try_get("trigger_payload_json")?;
    let trigger_payload = serde_json::from_str(&payload_json)
        .with_context(|| format!("failed to parse trigger_payload_json for run {id_raw}"))?;

    Ok(WorkflowRunRecord {
        id: Uuid::parse_str(&id_raw)?,
        workflow_id: Uuid::parse_str(&workflow_id_raw)?,
        agent_id: row.try_get("agent_id")?,
        trigger_type: row.try_get("trigger_type")?,
        trigger_payload,
        status: row.try_get("status")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        error_message: row.try_get("error_message")?,
    })
}

fn parse_step_row(row: sqlx::sqlite::SqliteRow) -> Result<WorkflowRunStepRecord> {
    let id_raw: String = row.try_get("id")?;
    let run_id_raw: String = row.try_get("run_id")?;
    let output_json: Option<String> = row.try_get("output_json")?;
    let output = match output_json {
        Some(raw) => Some(serde_json::from_str(&raw).with_context(|| {
            format!("failed to parse output_json for workflow run step {id_raw}")
        })?),
        None => None,
    };
    Ok(WorkflowRunStepRecord {
        id: Uuid::parse_str(&id_raw)?,
        run_id: Uuid::parse_str(&run_id_raw)?,
        step_index: row.try_get("step_index")?,
        node_id: row.try_get("node_id")?,
        node_kind: row.try_get("node_kind")?,
        note: row.try_get("note")?,
        output,
        occurred_at: row.try_get("occurred_at")?,
    })
}

fn validate_node_contract(node: &WorkflowNode) -> Result<()> {
    let node_type = node
        .config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match node.kind {
        WorkflowNodeKind::Trigger => match node_type {
            "manual" | "webhook" => Ok(()),
            "schedule" => {
                let cron_expr = node
                    .config
                    .get("cron")
                    .and_then(|v| v.as_str())
                    .context("schedule trigger requires config.cron")?;
                parse_schedule(cron_expr)
                    .with_context(|| format!("invalid schedule cron expression '{}'", cron_expr))
                    .map(|_| ())
            }
            "event" => {
                let event_topic = node
                    .config
                    .get("event")
                    .or_else(|| node.config.get("topic"))
                    .and_then(|v| v.as_str())
                    .context("event trigger requires config.event (or config.topic)")?;
                if event_topic.trim().is_empty() {
                    bail!("event trigger requires non-empty config.event");
                }
                Ok(())
            }
            _ => bail!(
                "trigger node '{}' has unsupported type '{}'",
                node.id,
                node_type
            ),
        },
        WorkflowNodeKind::Action => match node_type {
            "assistant_turn" => {
                if node
                    .config
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
                {
                    bail!(
                        "assistant_turn action '{}' requires non-empty config.prompt",
                        node.id
                    );
                }
                Ok(())
            }
            "http_request" => {
                let url = node
                    .config
                    .get("url")
                    .and_then(|v| v.as_str())
                    .context("http_request action requires config.url")?;
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    bail!("http_request action '{}' has invalid config.url", node.id);
                }
                if let Some(method) = node.config.get("method").and_then(|v| v.as_str()) {
                    let upper = method.to_ascii_uppercase();
                    if !["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                        .contains(&upper.as_str())
                    {
                        bail!("http_request action '{}' has invalid HTTP method", node.id);
                    }
                }
                Ok(())
            }
            _ => bail!(
                "action node '{}' has unsupported type '{}'",
                node.id,
                node_type
            ),
        },
        WorkflowNodeKind::Condition => match node_type {
            "always_true" | "always_false" => Ok(()),
            "has_trigger_payload_key" => {
                let key = node
                    .config
                    .get("key")
                    .and_then(|v| v.as_str())
                    .context("has_trigger_payload_key condition requires config.key")?;
                if key.trim().is_empty() {
                    bail!("has_trigger_payload_key condition requires non-empty config.key");
                }
                Ok(())
            }
            _ => bail!(
                "condition node '{}' has unsupported type '{}'",
                node.id,
                node_type
            ),
        },
    }
}

fn allowed_edge_outcomes(node: &WorkflowNode) -> Result<Vec<String>> {
    let node_type = node
        .config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let outcomes = match node.kind {
        WorkflowNodeKind::Trigger => vec!["trigger"],
        WorkflowNodeKind::Action => match node_type {
            "assistant_turn" | "http_request" => vec!["success", "failure"],
            _ => bail!("unsupported action type for edge validation: {node_type}"),
        },
        WorkflowNodeKind::Condition => vec!["true", "false"],
    };
    Ok(outcomes.into_iter().map(|s| s.to_string()).collect())
}

fn parse_schedule(expr: &str) -> Option<Schedule> {
    Schedule::from_str(expr)
        .or_else(|_| Schedule::from_str(&format!("0 {expr}")))
        .ok()
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct WorkflowMemoryState {
    workflows: HashMap<Uuid, WorkflowRecord>,
    webhooks: HashMap<Uuid, WorkflowWebhookEndpoint>,
    runs: HashMap<Uuid, WorkflowRunRecord>,
    steps: HashMap<Uuid, Vec<WorkflowRunStepRecord>>,
}

/// In-memory [`WorkflowStore`].
pub struct InMemoryWorkflowStore {
    state: Arc<Mutex<WorkflowMemoryState>>,
    agent_id: String,
    clock: Arc<dyn Clock>,
}

impl InMemoryWorkflowStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WorkflowMemoryState::default())),
            agent_id: "default".to_string(),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            ..Self::new()
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WorkflowMemoryState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

impl Default for InMemoryWorkflowStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowStore for InMemoryWorkflowStore {
    async fn create(
        &self,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<Uuid> {
        validate_workflow_graph(graph)?;
        let id = Uuid::new_v4();
        let now = self.clock.now();
        let mut state = self.lock();
        state.workflows.insert(
            id,
            WorkflowRecord {
                id,
                agent_id: self.agent_id.clone(),
                name: name.to_string(),
                description: description.to_string(),
                graph: graph.clone(),
                active,
                created_at: now,
                updated_at: now,
            },
        );
        Ok(id)
    }

    async fn ensure_webhook_endpoint(&self, workflow_id: Uuid) -> Result<WorkflowWebhookEndpoint> {
        if let Some(existing) = self.get_webhook_endpoint(workflow_id).await? {
            return Ok(existing);
        }
        let now = self.clock.now();
        let endpoint = WorkflowWebhookEndpoint {
            workflow_id,
            token: Uuid::new_v4().to_string(),
            active: true,
            created_at: now,
            updated_at: now,
        };
        let mut state = self.lock();
        state.webhooks.insert(workflow_id, endpoint.clone());
        Ok(endpoint)
    }

    async fn get_webhook_endpoint(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>> {
        let state = self.lock();
        Ok(state.webhooks.get(&workflow_id).cloned())
    }

    async fn rotate_webhook_token(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowWebhookEndpoint>> {
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(e) = state.webhooks.get_mut(&workflow_id) {
            e.token = Uuid::new_v4().to_string();
            e.updated_at = now;
            return Ok(Some(e.clone()));
        }
        Ok(None)
    }

    async fn resolve_by_webhook(
        &self,
        workflow_id: Uuid,
        token: &str,
    ) -> Result<Option<WorkflowRecord>> {
        let state = self.lock();
        let endpoint_ok = state
            .webhooks
            .get(&workflow_id)
            .map(|e| e.active && e.token == token)
            .unwrap_or(false);
        if !endpoint_ok {
            return Ok(None);
        }
        Ok(state
            .workflows
            .get(&workflow_id)
            .filter(|w| w.active && w.agent_id == self.agent_id)
            .cloned())
    }

    async fn create_run(
        &self,
        workflow_id: Uuid,
        trigger: WorkflowTriggerKind,
        trigger_payload: &Value,
    ) -> Result<Uuid> {
        let _w = self.get(workflow_id).await?.context("workflow not found")?;
        let run_id = Uuid::new_v4();
        let now = self.clock.now();
        let mut state = self.lock();
        state.runs.insert(
            run_id,
            WorkflowRunRecord {
                id: run_id,
                workflow_id,
                agent_id: self.agent_id.clone(),
                trigger_type: trigger.as_str().to_string(),
                trigger_payload: trigger_payload.clone(),
                status: "pending".to_string(),
                started_at: now,
                finished_at: None,
                error_message: None,
            },
        );
        Ok(run_id)
    }

    async fn list_runs(&self, workflow_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunRecord>> {
        let state = self.lock();
        let mut out: Vec<WorkflowRunRecord> = state
            .runs
            .values()
            .filter(|r| r.workflow_id == workflow_id && r.agent_id == self.agent_id)
            .cloned()
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn get_run(&self, workflow_id: Uuid, run_id: Uuid) -> Result<Option<WorkflowRunRecord>> {
        let state = self.lock();
        Ok(state
            .runs
            .get(&run_id)
            .filter(|r| r.workflow_id == workflow_id && r.agent_id == self.agent_id)
            .cloned())
    }

    async fn list_runnable_runs(&self, limit: i64) -> Result<Vec<WorkflowRunRecord>> {
        let state = self.lock();
        let mut out: Vec<WorkflowRunRecord> = state
            .runs
            .values()
            .filter(|r| r.status == "pending" && r.finished_at.is_none())
            .cloned()
            .collect();
        out.sort_by_key(|r| r.started_at);
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn workflow_for_run(&self, run_id: Uuid) -> Result<Option<WorkflowRecord>> {
        let state = self.lock();
        let Some(run) = state.runs.get(&run_id) else {
            return Ok(None);
        };
        Ok(state.workflows.get(&run.workflow_id).cloned())
    }

    async fn append_run_step(
        &self,
        run_id: Uuid,
        step_index: i64,
        node_id: &str,
        node_kind: &str,
        note: Option<&str>,
        output: Option<&Value>,
    ) -> Result<()> {
        let now = self.clock.now();
        let mut state = self.lock();
        state
            .steps
            .entry(run_id)
            .or_default()
            .push(WorkflowRunStepRecord {
                id: Uuid::new_v4(),
                run_id,
                step_index,
                node_id: node_id.to_string(),
                node_kind: node_kind.to_string(),
                note: note.map(str::to_string),
                output: output.cloned(),
                occurred_at: now,
            });
        Ok(())
    }

    async fn finish_run(
        &self,
        run_id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<bool> {
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(run) = state.runs.get_mut(&run_id)
            && run.finished_at.is_none()
        {
            run.status = status.to_string();
            run.finished_at = Some(now);
            run.error_message = error_message.map(str::to_string);
            return Ok(true);
        }
        Ok(false)
    }

    async fn list_run_steps(&self, run_id: Uuid, limit: i64) -> Result<Vec<WorkflowRunStepRecord>> {
        let state = self.lock();
        let mut out: Vec<WorkflowRunStepRecord> =
            state.steps.get(&run_id).cloned().unwrap_or_default();
        out.sort_by_key(|s| s.occurred_at);
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn list(&self) -> Result<Vec<WorkflowRecord>> {
        let state = self.lock();
        let mut out: Vec<WorkflowRecord> = state
            .workflows
            .values()
            .filter(|w| w.agent_id == self.agent_id)
            .cloned()
            .collect();
        out.sort_by_key(|w| std::cmp::Reverse(w.created_at));
        Ok(out)
    }

    async fn get(&self, id: Uuid) -> Result<Option<WorkflowRecord>> {
        let state = self.lock();
        Ok(state
            .workflows
            .get(&id)
            .filter(|w| w.agent_id == self.agent_id)
            .cloned())
    }

    async fn update(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<bool> {
        validate_workflow_graph(graph)?;
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(w) = state.workflows.get_mut(&id)
            && w.agent_id == self.agent_id
        {
            w.name = name.to_string();
            w.description = description.to_string();
            w.graph = graph.clone();
            w.active = active;
            w.updated_at = now;
            return Ok(true);
        }
        Ok(false)
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<bool> {
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(w) = state.workflows.get_mut(&id)
            && w.agent_id == self.agent_id
        {
            w.active = active;
            w.updated_at = now;
            return Ok(true);
        }
        Ok(false)
    }

    async fn toggle_active(&self, id: Uuid) -> Result<Option<bool>> {
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(w) = state.workflows.get_mut(&id)
            && w.agent_id == self.agent_id
        {
            w.active = !w.active;
            w.updated_at = now;
            return Ok(Some(w.active));
        }
        Ok(None)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let mut state = self.lock();
        if state
            .workflows
            .get(&id)
            .map(|w| w.agent_id == self.agent_id)
            .unwrap_or(false)
        {
            state.workflows.remove(&id);
            state.webhooks.remove(&id);
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    fn graph_with_loop() -> WorkflowGraph {
        WorkflowGraph {
            version: 1,
            nodes: vec![
                WorkflowNode {
                    id: "trigger_1".to_string(),
                    kind: WorkflowNodeKind::Trigger,
                    config: serde_json::json!({"type": "manual"}),
                },
                WorkflowNode {
                    id: "action_1".to_string(),
                    kind: WorkflowNodeKind::Action,
                    config: serde_json::json!({"type": "assistant_turn", "prompt": "do work"}),
                },
                WorkflowNode {
                    id: "condition_1".to_string(),
                    kind: WorkflowNodeKind::Condition,
                    config: serde_json::json!({"type": "always_true"}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "trigger_1".to_string(),
                    to: "action_1".to_string(),
                    on: None,
                    condition: None,
                },
                WorkflowEdge {
                    from: "action_1".to_string(),
                    to: "condition_1".to_string(),
                    on: Some("success".to_string()),
                    condition: None,
                },
                WorkflowEdge {
                    from: "condition_1".to_string(),
                    to: "action_1".to_string(),
                    on: Some("true".to_string()),
                    condition: None,
                },
            ],
            execution: WorkflowExecutionLimits {
                max_steps: 50,
                max_visits_per_node: 10,
            },
        }
    }

    #[test]
    fn validate_allows_loop_edges() {
        let graph = graph_with_loop();
        validate_workflow_graph(&graph).expect("loop edges should be valid with execution limits");
    }

    #[test]
    fn validate_rejects_missing_trigger() {
        let mut graph = graph_with_loop();
        graph.nodes.retain(|n| n.kind != WorkflowNodeKind::Trigger);
        let err = validate_workflow_graph(&graph).unwrap_err().to_string();
        assert!(err.contains("trigger"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_invalid_edge_label_for_condition() {
        let mut graph = graph_with_loop();
        graph.edges[2].on = Some("success".to_string());
        let err = validate_workflow_graph(&graph).unwrap_err().to_string();
        assert!(err.contains("invalid"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_http_action_without_url() {
        let mut graph = graph_with_loop();
        graph.nodes[1].config = serde_json::json!({"type": "http_request"});
        let err = validate_workflow_graph(&graph).unwrap_err().to_string();
        assert!(err.contains("config.url"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn create_and_get_roundtrip() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteWorkflowStore::new(storage.pool.clone());
        let graph = graph_with_loop();

        let id = store
            .create("looping-workflow", "demo", &graph, true)
            .await
            .unwrap();

        let loaded = store.get(id).await.unwrap().expect("workflow exists");
        assert_eq!(loaded.name, "looping-workflow");
        assert_eq!(loaded.graph.edges.len(), 3);
        assert!(loaded.active);
    }

    #[tokio::test]
    async fn update_and_toggle() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteWorkflowStore::new(storage.pool.clone());
        let graph = graph_with_loop();

        let id = store.create("wf", "desc", &graph, true).await.unwrap();
        let mut next = graph.clone();
        next.execution.max_steps = 75;

        let updated = store
            .update(id, "wf-2", "desc-2", &next, false)
            .await
            .unwrap();
        assert!(updated);

        let loaded = store.get(id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "wf-2");
        assert_eq!(loaded.graph.execution.max_steps, 75);
        assert!(!loaded.active);

        let toggled = store.set_active(id, true).await.unwrap();
        assert!(toggled);
        assert!(store.get(id).await.unwrap().unwrap().active);
    }

    #[tokio::test]
    async fn webhook_endpoint_and_runs_roundtrip() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = SqliteWorkflowStore::new(storage.pool.clone());
        let graph = graph_with_loop();
        let workflow_id = store.create("wf", "desc", &graph, true).await.unwrap();

        let endpoint = store.ensure_webhook_endpoint(workflow_id).await.unwrap();
        assert_eq!(endpoint.workflow_id, workflow_id);
        assert!(!endpoint.token.is_empty());

        let resolved = store
            .resolve_by_webhook(workflow_id, &endpoint.token)
            .await
            .unwrap();
        assert!(resolved.is_some());

        let run_id = store
            .create_run(
                workflow_id,
                WorkflowTriggerKind::Webhook,
                &serde_json::json!({"hello": "world"}),
            )
            .await
            .unwrap();

        let runnable = store.list_runnable_runs(10).await.unwrap();
        assert!(runnable.iter().any(|r| r.id == run_id));

        store
            .append_run_step(run_id, 1, "trigger_1", "trigger", Some("start"), None)
            .await
            .unwrap();
        let steps = store.list_run_steps(run_id, 10).await.unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].node_id, "trigger_1");

        let wf_for_run = store.workflow_for_run(run_id).await.unwrap();
        assert!(wf_for_run.is_some());

        let runs = store.list_runs(workflow_id, 20).await.unwrap();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].id, run_id);
        assert_eq!(runs[0].trigger_type, "webhook");

        let finished = store.finish_run(run_id, "completed", None).await.unwrap();
        assert!(finished);

        let rotated = store
            .rotate_webhook_token(workflow_id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(endpoint.token, rotated.token);
    }
}
