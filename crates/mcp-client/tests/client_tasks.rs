//! Tool calls that an external server answers with a task: polling cadence,
//! terminal outcomes, cancellation and the tool timeout. The clock is paused,
//! so poll gaps are exact.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_mcp_client::StaleTools;
use assistant_mcp_client::bridge::McpToolHandler;
use assistant_mcp_client::client::McpClient;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, CancelTaskParams, ContentBlock,
    CreateTaskResult, DetailedTask, GetTaskParams, GetTaskResult, ServerCapabilities, ServerConfig,
    Task, TaskPayload, TaskStatus, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::json;
use tokio::sync::Notify;
use tokio::time::Instant;
use uuid::Uuid;

const TASK_ID: &str = "task-1";

/// A server whose `slow` tool always returns a task. `tasks/get` walks
/// through a script of payloads and repeats the last one.
#[derive(Clone)]
struct TaskServer {
    /// `pollIntervalMs` the server suggests, `None` for no suggestion.
    poll_interval_ms: Option<u64>,
    script: Arc<Mutex<VecDeque<TaskPayload>>>,
    polls: Arc<Mutex<Vec<Instant>>>,
    cancelled: Arc<Mutex<Vec<String>>>,
    cancel_seen: Arc<Notify>,
}

impl TaskServer {
    fn new(poll_interval_ms: Option<u64>, script: Vec<TaskPayload>) -> Self {
        Self {
            poll_interval_ms,
            script: Arc::new(Mutex::new(script.into())),
            polls: Arc::default(),
            cancelled: Arc::default(),
            cancel_seen: Arc::default(),
        }
    }

    fn task(&self, status: TaskStatus) -> Task {
        let mut task = Task::new(
            TASK_ID,
            status,
            "2026-07-28T10:00:00Z",
            "2026-07-28T10:00:00Z",
        );
        if let Some(ms) = self.poll_interval_ms {
            task = task.with_poll_interval_ms(ms);
        }
        task
    }

    /// The gaps between successive `tasks/get` calls, and from the first one
    /// to the call that created the task.
    fn poll_gaps(&self, created: Instant) -> Vec<Duration> {
        let polls = self.polls.lock().unwrap();
        let mut previous = created;
        polls
            .iter()
            .map(|at| {
                let gap = *at - previous;
                previous = *at;
                gap
            })
            .collect()
    }
}

impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tasks()
                .build(),
        )
    }

    async fn call_tool(
        &self,
        _request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        Ok(CallToolResponse::Task(CreateTaskResult::new(
            self.task(TaskStatus::Working),
        )))
    }

    fn get_task(
        &self,
        _request: GetTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetTaskResult, ErrorData>> + MaybeSendFuture + '_ {
        self.polls.lock().unwrap().push(Instant::now());
        let mut script = self.script.lock().unwrap();
        let payload = if script.len() > 1 {
            script.pop_front()
        } else {
            script.front().cloned()
        }
        .unwrap_or(TaskPayload::Working);
        let task = self.task(payload.status());
        std::future::ready(Ok(GetTaskResult::new(DetailedTask::new(task, payload))))
    }

    fn cancel_task(
        &self,
        request: CancelTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + MaybeSendFuture + '_ {
        self.cancelled.lock().unwrap().push(request.task_id);
        self.cancel_seen.notify_one();
        std::future::ready(Ok(()))
    }
}

/// A handler for the `slow` tool of `server`, with the given tool timeout,
/// and the instant the clock stood at when the connection opened.
async fn handler_for(server: &TaskServer, tool_timeout: Duration) -> McpToolHandler {
    let (server_io, client_io) = tokio::io::duplex(8192);
    let server = server.clone();
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let client = McpClient::connect("remote", client_io, StaleTools::default())
        .await
        .expect("the client connects")
        .with_tool_timeout(tool_timeout);
    let tool = Tool::new("slow", "a slow tool", Arc::new(serde_json::Map::new()));
    McpToolHandler::from_remote("remote", &tool, Arc::new(client), false)
}

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: Uuid::new_v4(),
        agent_id: "tester".into(),
        turn: 0,
        interface: Interface::Mcp,
        interactive: false,
        run_id: None,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

fn completed(text: &str) -> TaskPayload {
    let result = CallToolResult::success(vec![ContentBlock::text(text)]);
    let serde_json::Value::Object(result) = serde_json::to_value(result).unwrap() else {
        unreachable!("a tool result is an object")
    };
    TaskPayload::Completed { result }
}

fn failed(message: &str) -> TaskPayload {
    let serde_json::Value::Object(error) = json!({"code": -32603, "message": message}) else {
        unreachable!("a json object literal")
    };
    TaskPayload::Failed { error }
}

const LONG: Duration = Duration::from_secs(600);

#[tokio::test(start_paused = true)]
async fn a_task_is_polled_at_its_poll_interval_until_completed() {
    let server = TaskServer::new(
        Some(2000),
        vec![
            TaskPayload::Working,
            TaskPayload::Working,
            completed("all done"),
        ],
    );
    let handler = handler_for(&server, LONG).await;

    let started = Instant::now();
    let output = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(output.success, "got: {}", output.content);
    assert_eq!(
        output.content, "all done",
        "the task's result is the output"
    );
    assert_eq!(
        server.poll_gaps(started),
        vec![Duration::from_secs(2); 3],
        "each poll waits one suggested interval"
    );
}

#[tokio::test(start_paused = true)]
async fn without_a_suggested_interval_the_task_is_polled_every_second() {
    let server = TaskServer::new(None, vec![TaskPayload::Working, completed("ok")]);
    let handler = handler_for(&server, LONG).await;

    let started = Instant::now();
    handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert_eq!(
        server.poll_gaps(started),
        vec![Duration::from_secs(1); 2],
        "the default interval is one second"
    );
}

#[tokio::test(start_paused = true)]
async fn a_suggested_interval_is_capped_at_ten_seconds() {
    let server = TaskServer::new(Some(60_000), vec![completed("ok")]);
    let handler = handler_for(&server, LONG).await;

    let started = Instant::now();
    handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert_eq!(
        server.poll_gaps(started),
        vec![Duration::from_secs(10)],
        "a minute-long suggestion is cut to ten seconds"
    );
}

#[tokio::test(start_paused = true)]
async fn a_failed_task_becomes_a_tool_error_with_its_message() {
    let server = TaskServer::new(Some(1000), vec![failed("disk on fire")]);
    let handler = handler_for(&server, LONG).await;

    let output = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(!output.success, "a failed task is an error output");
    assert!(
        output.content.contains("disk on fire"),
        "the failure message reaches the model, got: {}",
        output.content
    );
}

#[tokio::test(start_paused = true)]
async fn a_cancelled_task_becomes_a_tool_error_saying_so() {
    let server = TaskServer::new(Some(1000), vec![TaskPayload::Cancelled]);
    let handler = handler_for(&server, LONG).await;

    let output = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(!output.success, "a cancelled task is an error output");
    assert!(
        output.content.contains("cancelled"),
        "the output says the task was cancelled, got: {}",
        output.content
    );
}

#[tokio::test(start_paused = true)]
async fn a_task_that_needs_input_fails_and_is_cancelled() {
    let server = TaskServer::new(
        Some(1000),
        vec![TaskPayload::InputRequired {
            input_requests: Default::default(),
        }],
    );
    let handler = handler_for(&server, LONG).await;

    let output = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(!output.success, "input cannot be given, so the call fails");
    assert!(
        output.content.contains("input"),
        "the message explains that input was required, got: {}",
        output.content
    );
    tokio::time::timeout(Duration::from_secs(1), server.cancel_seen.notified())
        .await
        .expect("the abandoned task is cancelled");
    assert_eq!(*server.cancelled.lock().unwrap(), vec![TASK_ID]);
}

#[tokio::test(start_paused = true)]
async fn cancelling_the_turn_cancels_the_task() {
    let server = TaskServer::new(Some(1000), vec![TaskPayload::Working]);
    let handler = handler_for(&server, LONG).await;

    // The orchestrator cancels a turn by dropping the tool's future.
    let ctx = ctx();
    let turn = tokio::time::timeout(Duration::from_secs(3), handler.run(HashMap::new(), &ctx));
    assert!(
        turn.await.is_err(),
        "the task is still working when the turn is cancelled"
    );

    tokio::time::timeout(Duration::from_secs(1), server.cancel_seen.notified())
        .await
        .expect("tasks/cancel is sent");
    assert_eq!(*server.cancelled.lock().unwrap(), vec![TASK_ID]);
}

#[tokio::test(start_paused = true)]
async fn the_tool_timeout_bounds_polling_and_cancels_the_task() {
    let server = TaskServer::new(Some(1000), vec![TaskPayload::Working]);
    let handler = handler_for(&server, Duration::from_secs(5)).await;

    let started = Instant::now();
    let output = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(!output.success, "a timed-out call is an error output");
    assert!(
        output.content.contains("timed out"),
        "the output says the call timed out, got: {}",
        output.content
    );
    assert_eq!(
        started.elapsed(),
        Duration::from_secs(5),
        "polling stops when the timeout expires"
    );
    assert_eq!(
        server.polls.lock().unwrap().len(),
        4,
        "polls at 1s, 2s, 3s and 4s"
    );
    tokio::time::timeout(Duration::from_secs(1), server.cancel_seen.notified())
        .await
        .expect("the timed-out task is cancelled");
}
