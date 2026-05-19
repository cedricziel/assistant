//! Coverage for `run_scheduler_loop` and the built-in `PeriodicTask`
//! impls in `scheduler/mod.rs`.
//!
//! Uses a tracking `PeriodicTask` impl that counts how many times it was
//! invoked under a very short poll interval, so the loop body is exercised
//! at runtime without depending on the real `Orchestrator` 30-minute
//! heartbeat or 60-minute prune intervals.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use wiremock::MockServer;

use crate::scheduler::{PeriodicTask, SchedulerCtx, run_scheduler_loop};

use super::build;

struct CountingTask {
    name: &'static str,
    interval: Duration,
    run_before: bool,
    call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl PeriodicTask for CountingTask {
    fn name(&self) -> &'static str {
        self.name
    }
    fn interval(&self) -> Duration {
        self.interval
    }
    fn run_before_loop(&self) -> bool {
        self.run_before
    }
    async fn run(&self, _ctx: &SchedulerCtx) -> anyhow::Result<()> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct FailingTask {
    call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl PeriodicTask for FailingTask {
    fn name(&self) -> &'static str {
        "failing"
    }
    fn interval(&self) -> Duration {
        Duration::from_millis(10)
    }
    fn run_before_loop(&self) -> bool {
        true
    }
    async fn run(&self, _ctx: &SchedulerCtx) -> anyhow::Result<()> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("intentional failure for test")
    }
}

#[tokio::test]
async fn run_scheduler_loop_fires_run_before_then_polls() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;
    let ctx = Arc::new(SchedulerCtx {
        storage,
        orchestrator: orch,
    });

    let before_count = Arc::new(AtomicUsize::new(0));
    let after_count = Arc::new(AtomicUsize::new(0));

    let tasks: Vec<Box<dyn PeriodicTask>> = vec![
        Box::new(CountingTask {
            name: "with_run_before",
            interval: Duration::from_secs(3600), // never fires in the loop again
            run_before: true,
            call_count: before_count.clone(),
        }),
        Box::new(CountingTask {
            name: "regular",
            interval: Duration::from_millis(20),
            run_before: false,
            call_count: after_count.clone(),
        }),
    ];

    // Drive the loop with a 10ms poll for ~120ms then abort. We expect:
    // - the run_before task to fire exactly once at startup.
    // - the regular task to fire several times during polling.
    let handle = tokio::spawn(run_scheduler_loop(ctx, tasks, Duration::from_millis(10)));
    tokio::time::sleep(Duration::from_millis(120)).await;
    handle.abort();
    let _ = handle.await;

    // run_before fires once at startup; then last_run is set to "already
    // elapsed", so the task fires once more on the first poll tick. After
    // that it doesn't fire because its interval is 1h. So we expect 2.
    assert_eq!(
        before_count.load(Ordering::SeqCst),
        2,
        "run_before_loop=true task fires at startup AND on first poll tick"
    );
    let after = after_count.load(Ordering::SeqCst);
    assert!(
        after >= 3,
        "regular polling task should fire several times in 120ms; got {after}"
    );
}

#[tokio::test]
async fn run_scheduler_loop_logs_and_continues_when_task_errors() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;
    let ctx = Arc::new(SchedulerCtx {
        storage,
        orchestrator: orch,
    });

    let fail_count = Arc::new(AtomicUsize::new(0));
    let tasks: Vec<Box<dyn PeriodicTask>> = vec![Box::new(FailingTask {
        call_count: fail_count.clone(),
    })];

    let handle = tokio::spawn(run_scheduler_loop(ctx, tasks, Duration::from_millis(10)));
    tokio::time::sleep(Duration::from_millis(80)).await;
    handle.abort();
    let _ = handle.await;

    // Run-before fires once + polling fires several times; even though
    // every call errors, the loop keeps going.
    let count = fail_count.load(Ordering::SeqCst);
    assert!(
        count >= 3,
        "failing task should be re-invoked despite errors; got {count}"
    );
}

#[tokio::test]
async fn periodic_task_respects_its_interval() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;
    let ctx = Arc::new(SchedulerCtx {
        storage,
        orchestrator: orch,
    });

    let count = Arc::new(AtomicUsize::new(0));
    let tasks: Vec<Box<dyn PeriodicTask>> = vec![Box::new(CountingTask {
        name: "slow",
        interval: Duration::from_secs(3600), // basically never fires after startup
        run_before: false,
        call_count: count.clone(),
    })];

    let handle = tokio::spawn(run_scheduler_loop(ctx, tasks, Duration::from_millis(10)));
    tokio::time::sleep(Duration::from_millis(80)).await;
    handle.abort();
    let _ = handle.await;

    // last_run is initialised to "already elapsed", so the task fires once
    // on the first poll tick (no run_before path), then never again because
    // its interval is 3600s.
    let n = count.load(Ordering::SeqCst);
    assert_eq!(
        n, 1,
        "task with long interval and no run_before must fire only once; got {n}"
    );
}
