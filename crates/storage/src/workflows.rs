//! Workflow persistence for graph-style trigger/action automation.

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
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

/// SQLite-backed store for workflows.
pub struct WorkflowStore {
    pool: SqlitePool,
    agent_id: String,
}

impl WorkflowStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            agent_id: "default".to_string(),
        }
    }

    pub fn for_agent(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
        }
    }

    /// Insert a new workflow.
    pub async fn create(
        &self,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<Uuid> {
        validate_workflow_graph(graph)?;

        let id = Uuid::new_v4();
        let now = Utc::now();
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

    /// List workflows for this agent.
    pub async fn list(&self) -> Result<Vec<WorkflowRecord>> {
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
    pub async fn get(&self, id: Uuid) -> Result<Option<WorkflowRecord>> {
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
    pub async fn update(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        graph: &WorkflowGraph,
        active: bool,
    ) -> Result<bool> {
        validate_workflow_graph(graph)?;

        let now = Utc::now();
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
    pub async fn set_active(&self, id: Uuid, active: bool) -> Result<bool> {
        let now = Utc::now();
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

    /// Delete a workflow.
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
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
                    config: serde_json::json!({"type": "until_done"}),
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

    #[tokio::test]
    async fn create_and_get_roundtrip() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = WorkflowStore::new(storage.pool.clone());
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
        let store = WorkflowStore::new(storage.pool.clone());
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
}
