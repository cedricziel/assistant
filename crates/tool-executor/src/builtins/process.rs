//! Builtin handler for the `process` tool — manages long-running background processes.
//!
//! Each spawned process is assigned a UUID session ID. The tool supports six actions:
//! - `start`  — spawn a shell command and return a `session_id` + `pid`
//! - `poll`   — check whether a session is still running and retrieve its exit code
//! - `log`    — read the last N lines of buffered stdout / stderr
//! - `write`  — send data to the process's stdin
//! - `kill`   — terminate a running process
//! - `list`   — enumerate all tracked sessions

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::{oneshot, Mutex};
use tracing::debug;
use uuid::Uuid;

const MAX_LINES: usize = 1000;

// -- ProcessHandle -------------------------------------------------------------

struct ProcessHandle {
    pid: u32,
    command: String,
    started_at: String,
    stdin: Mutex<Option<ChildStdin>>,
    stdout_buf: Arc<Mutex<VecDeque<String>>>,
    stderr_buf: Arc<Mutex<VecDeque<String>>>,
    exit_code: Arc<Mutex<Option<i32>>>,
    running: Arc<AtomicBool>,
    /// Send a `()` to this channel to request the background wait task to kill
    /// the process and exit.
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
}

// -- ProcessHandler ------------------------------------------------------------

/// Tool handler for `process` — session-based background process management.
pub struct ProcessHandler {
    sessions: Arc<Mutex<HashMap<String, Arc<ProcessHandle>>>>,
}

impl ProcessHandler {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for ProcessHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ProcessHandler {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Manage long-running background processes. \
         Actions: start (launch a shell command), poll (check running/exit status), \
         log (read buffered stdout/stderr), write (send data to stdin), \
         kill (terminate), list (show all sessions)."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "poll", "log", "write", "kill", "list"],
                    "description": "The action to perform"
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to run (required for action=start)"
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory for the process (optional for action=start)"
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID returned by start (required for poll, log, write, kill)"
                },
                "lines": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Number of output lines to return (optional for action=log, default 50)"
                },
                "data": {
                    "type": "string",
                    "description": "Data to write to stdin (required for action=write)"
                }
            },
            "required": ["action"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let action = match params.get("action").and_then(|v| v.as_str()) {
            Some(a) => a.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'action'")),
        };

        match action.as_str() {
            "start" => self.action_start(&params).await,
            "poll" => self.action_poll(&params).await,
            "log" => self.action_log(&params).await,
            "write" => self.action_write(&params).await,
            "kill" => self.action_kill(&params).await,
            "list" => self.action_list().await,
            _ => Ok(ToolOutput::error(format!(
                "Unknown action '{action}'. Valid actions: start, poll, log, write, kill, list"
            ))),
        }
    }
}

// -- Action implementations ----------------------------------------------------

impl ProcessHandler {
    async fn action_start(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<ToolOutput> {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'command' for action=start",
                ))
            }
        };
        let workdir = params
            .get("workdir")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c").arg(&command);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(false);

        if let Some(ref dir) = workdir {
            cmd.current_dir(dir);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return Ok(ToolOutput::error(format!("Failed to spawn process: {e}"))),
        };

        let pid = child.id().unwrap_or(0);
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let stdout_buf: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_buf: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let exit_code: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
        let running = Arc::new(AtomicBool::new(true));
        let (kill_tx, kill_rx) = oneshot::channel::<()>();

        // Drain stdout into ring buffer.
        let stdout_buf_clone = stdout_buf.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut buf = stdout_buf_clone.lock().await;
                if buf.len() >= MAX_LINES {
                    buf.pop_front();
                }
                buf.push_back(line);
            }
        });

        // Drain stderr into ring buffer.
        let stderr_buf_clone = stderr_buf.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut buf = stderr_buf_clone.lock().await;
                if buf.len() >= MAX_LINES {
                    buf.pop_front();
                }
                buf.push_back(line);
            }
        });

        // Wait for process exit (or kill signal).
        let exit_code_clone = exit_code.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            tokio::select! {
                result = child.wait() => {
                    let code = result.ok().and_then(|s| s.code()).unwrap_or(-1);
                    *exit_code_clone.lock().await = Some(code);
                    running_clone.store(false, Ordering::SeqCst);
                }
                _ = kill_rx => {
                    let _ = child.kill().await;
                    let result = child.wait().await;
                    let code = result.ok().and_then(|s| s.code()).unwrap_or(-1);
                    *exit_code_clone.lock().await = Some(code);
                    running_clone.store(false, Ordering::SeqCst);
                }
            }
        });

        let session_id = Uuid::new_v4().to_string();
        let handle = Arc::new(ProcessHandle {
            pid,
            command: command.clone(),
            started_at: Utc::now().to_rfc3339(),
            stdin: Mutex::new(stdin),
            stdout_buf,
            stderr_buf,
            exit_code,
            running,
            kill_tx: Mutex::new(Some(kill_tx)),
        });

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), handle);

        debug!(
            "process: started session={} pid={} cmd={}",
            session_id, pid, command
        );

        Ok(ToolOutput::success(format!(
            "Started process: session_id={session_id}, pid={pid}"
        ))
        .with_data(serde_json::json!({
            "session_id": session_id,
            "pid": pid,
        })))
    }

    async fn action_poll(&self, params: &HashMap<String, serde_json::Value>) -> Result<ToolOutput> {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'session_id' for action=poll",
                ))
            }
        };

        let handle = {
            let sessions = self.sessions.lock().await;
            match sessions.get(&session_id) {
                Some(h) => h.clone(),
                None => {
                    return Ok(ToolOutput::error(format!(
                        "Session '{session_id}' not found"
                    )))
                }
            }
        };

        let running = handle.running.load(Ordering::SeqCst);
        let exit_code = *handle.exit_code.lock().await;

        let mut data = serde_json::json!({ "running": running });
        if let Some(code) = exit_code {
            data["exit_code"] = serde_json::json!(code);
        }

        let msg = match (running, exit_code) {
            (true, _) => "Process is running".to_string(),
            (false, Some(code)) => format!("Process exited with code {code}"),
            (false, None) => "Process has stopped".to_string(),
        };

        Ok(ToolOutput::success(msg).with_data(data))
    }

    async fn action_log(&self, params: &HashMap<String, serde_json::Value>) -> Result<ToolOutput> {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'session_id' for action=log",
                ))
            }
        };
        let lines = params.get("lines").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let handle = {
            let sessions = self.sessions.lock().await;
            match sessions.get(&session_id) {
                Some(h) => h.clone(),
                None => {
                    return Ok(ToolOutput::error(format!(
                        "Session '{session_id}' not found"
                    )))
                }
            }
        };

        let stdout = {
            let buf = handle.stdout_buf.lock().await;
            let skip = buf.len().saturating_sub(lines);
            buf.iter()
                .skip(skip)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };
        let stderr = {
            let buf = handle.stderr_buf.lock().await;
            let skip = buf.len().saturating_sub(lines);
            buf.iter()
                .skip(skip)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(
            ToolOutput::success(format!("stdout:\n{stdout}\nstderr:\n{stderr}")).with_data(
                serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                }),
            ),
        )
    }

    async fn action_write(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<ToolOutput> {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'session_id' for action=write",
                ))
            }
        };
        let data = match params.get("data").and_then(|v| v.as_str()) {
            Some(d) => d.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'data' for action=write",
                ))
            }
        };

        let handle = {
            let sessions = self.sessions.lock().await;
            match sessions.get(&session_id) {
                Some(h) => h.clone(),
                None => {
                    return Ok(ToolOutput::error(format!(
                        "Session '{session_id}' not found"
                    )))
                }
            }
        };

        let mut stdin_lock = handle.stdin.lock().await;
        match stdin_lock.as_mut() {
            Some(stdin) => match stdin.write_all(data.as_bytes()).await {
                Ok(()) => Ok(ToolOutput::success("Data written to process stdin")),
                Err(e) => Ok(ToolOutput::error(format!("Failed to write to stdin: {e}"))),
            },
            None => Ok(ToolOutput::error(
                "stdin is closed or not available for this process",
            )),
        }
    }

    async fn action_kill(&self, params: &HashMap<String, serde_json::Value>) -> Result<ToolOutput> {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(ToolOutput::error(
                    "Missing required parameter 'session_id' for action=kill",
                ))
            }
        };

        let handle = {
            let sessions = self.sessions.lock().await;
            match sessions.get(&session_id) {
                Some(h) => h.clone(),
                None => {
                    return Ok(ToolOutput::error(format!(
                        "Session '{session_id}' not found"
                    )))
                }
            }
        };

        if !handle.running.load(Ordering::SeqCst) {
            return Ok(ToolOutput::success("Process is already stopped"));
        }

        let mut kill_tx = handle.kill_tx.lock().await;
        match kill_tx.take() {
            Some(tx) => {
                let _ = tx.send(());
                Ok(ToolOutput::success(format!(
                    "Kill signal sent to process (pid={})",
                    handle.pid
                )))
            }
            None => Ok(ToolOutput::success("Process kill already initiated")),
        }
    }

    async fn action_list(&self) -> Result<ToolOutput> {
        let sessions = self.sessions.lock().await;

        let mut list: Vec<serde_json::Value> = sessions
            .iter()
            .map(|(session_id, handle)| {
                let running = handle.running.load(Ordering::SeqCst);
                serde_json::json!({
                    "session_id": session_id,
                    "pid": handle.pid,
                    "command": handle.command,
                    "started_at": handle.started_at,
                    "running": running,
                })
            })
            .collect();

        list.sort_by(|a, b| {
            let a_time = a.get("started_at").and_then(|v| v.as_str()).unwrap_or("");
            let b_time = b.get("started_at").and_then(|v| v.as_str()).unwrap_or("");
            a_time.cmp(b_time)
        });

        let msg = if list.is_empty() {
            "No active process sessions".to_string()
        } else {
            format!("{} process session(s)", list.len())
        };

        Ok(ToolOutput::success(msg).with_data(serde_json::json!(list)))
    }
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use uuid::Uuid;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    fn p(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn test_start_poll_log_kill_lifecycle() {
        let handler = ProcessHandler::new();

        // Start a process that prints a line then sleeps.
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("start")),
                    ("command", serde_json::json!("echo hello; sleep 30")),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "start should succeed");

        let data = out.data.unwrap();
        let session_id = data["session_id"].as_str().unwrap().to_string();
        let pid = data["pid"].as_u64().unwrap();
        assert!(pid > 0, "pid should be positive");

        // Give the echo command time to run and be buffered.
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Poll — should still be running (sleep 30).
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("poll")),
                    ("session_id", serde_json::json!(&session_id)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "poll should succeed");
        assert_eq!(
            out.data.unwrap()["running"],
            serde_json::json!(true),
            "process should still be running"
        );

        // Log — stdout should contain "hello".
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("log")),
                    ("session_id", serde_json::json!(&session_id)),
                    ("lines", serde_json::json!(10)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "log should succeed");
        let log_data = out.data.unwrap();
        assert!(
            log_data["stdout"].as_str().unwrap().contains("hello"),
            "stdout should contain 'hello', got: {}",
            log_data["stdout"]
        );

        // Kill.
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("kill")),
                    ("session_id", serde_json::json!(&session_id)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "kill should succeed");

        // Give the kill time to propagate.
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Poll after kill — should not be running.
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("poll")),
                    ("session_id", serde_json::json!(&session_id)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "poll after kill should succeed");
        assert_eq!(
            out.data.unwrap()["running"],
            serde_json::json!(false),
            "process should not be running after kill"
        );
    }

    #[tokio::test]
    async fn test_process_natural_exit() {
        let handler = ProcessHandler::new();

        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("start")),
                    ("command", serde_json::json!("echo done")),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        let session_id = out.data.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();

        // Wait for process to exit naturally.
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("poll")),
                    ("session_id", serde_json::json!(&session_id)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        let data = out.data.unwrap();
        assert_eq!(
            data["running"],
            serde_json::json!(false),
            "process should have exited"
        );
        assert_eq!(
            data["exit_code"],
            serde_json::json!(0),
            "exit code should be 0"
        );
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let handler = ProcessHandler::new();

        // Initially empty.
        let out = handler
            .run(p(&[("action", serde_json::json!("list"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
        assert_eq!(
            out.data.unwrap().as_array().unwrap().len(),
            0,
            "should start with no sessions"
        );

        // Start a process.
        handler
            .run(
                p(&[
                    ("action", serde_json::json!("start")),
                    ("command", serde_json::json!("sleep 10")),
                ]),
                &ctx(),
            )
            .await
            .unwrap();

        // List should show one session.
        let out = handler
            .run(p(&[("action", serde_json::json!("list"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
        assert_eq!(
            out.data.unwrap().as_array().unwrap().len(),
            1,
            "should have one session"
        );
    }

    #[tokio::test]
    async fn test_unknown_session_returns_error() {
        let handler = ProcessHandler::new();
        let out = handler
            .run(
                p(&[
                    ("action", serde_json::json!("poll")),
                    ("session_id", serde_json::json!("nonexistent-session")),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(
            !out.success,
            "polling unknown session should return an error"
        );
        assert!(
            out.content.contains("not found"),
            "error message should mention 'not found'"
        );
    }
}
