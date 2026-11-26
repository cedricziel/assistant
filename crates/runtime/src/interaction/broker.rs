//! The interaction broker (ADR-0017, D2): the turn-side half of the shared
//! client-to-server interaction channel.
//!
//! A turn calls [`InteractionBroker::request`], which persists a `pending`
//! row, announces it on the turn's event sink and waits. The HTTP side calls
//! [`InteractionBroker::respond`]. The `interactions` table in `space.db` is
//! the source of truth, so the two halves work in separate processes: the
//! waiter polls the store, and a same-process `respond` also fires a oneshot
//! so the turn resumes at once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tracing::debug;
use uuid::Uuid;

use assistant_core::clock::{Clock, SystemClock};
use assistant_storage::{
    InteractionRow, InteractionStatus, InteractionStore, NewInteraction, ResolveOutcome,
};

use crate::interaction::kinds::{InteractionKinds, InteractionOutcome};
use crate::orchestrator::OrchestratorEvent;

/// Default time an interaction stays open (`[permissions] interaction_timeout_secs`).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
/// How often a waiter checks the store for a resolution another process wrote.
const STORE_POLL_EVERY: Duration = Duration::from_secs(1);
/// How often a waiter proves it is alive.
const WAITER_TOUCH_EVERY: Duration = Duration::from_secs(15);
/// A `pending` row unseen for this long has no live waiter.
pub const WAITER_STALE_AFTER: Duration = Duration::from_secs(45);

/// What a turn asks its owner.
#[derive(Debug, Clone)]
pub struct InteractionRequest {
    pub conversation_id: String,
    pub run_id: String,
    pub user_id: String,
    pub kind: String,
    pub tool_call_id: Option<String>,
    /// The kind-specific request.
    pub request: Value,
    /// When the turn itself times out; `expires_at` never exceeds it.
    pub turn_deadline: Option<DateTime<Utc>>,
}

/// Who is answering, and which run they say they are answering for.
#[derive(Debug, Clone, Copy)]
pub struct Responder<'a> {
    pub user_id: &'a str,
    pub conversation_id: &'a str,
    pub run_id: &'a str,
}

impl Responder<'_> {
    /// Whether `row` belongs to this caller, conversation and run.
    pub fn owns(&self, row: &InteractionRow) -> bool {
        row.user_id == self.user_id
            && row.conversation_id == self.conversation_id
            && row.run_id == self.run_id
    }
}

/// Why [`InteractionBroker::respond`] refused.
#[derive(Debug)]
pub enum RespondError {
    /// Unknown interaction, or not the caller's.
    NotFound,
    /// Already resolved with a different answer; carries the stored row.
    Conflict(Box<InteractionRow>),
    /// Expired, cancelled, or no live waiter.
    Gone,
    /// The answer does not fit the interaction.
    Invalid(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for RespondError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e)
    }
}

/// Persists interactions and waits for their answers.
pub struct InteractionBroker {
    store: Arc<dyn InteractionStore>,
    kinds: Arc<InteractionKinds>,
    clock: Arc<dyn Clock>,
    timeout: Duration,
    /// Same-process fast path: interaction id to the waiter's wake-up.
    waiters: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

/// Removes the same-process waiter when the wait ends, however it ends.
struct WaiterGuard<'a> {
    broker: &'a InteractionBroker,
    id: String,
}

impl Drop for WaiterGuard<'_> {
    fn drop(&mut self) {
        self.broker.lock_waiters().remove(&self.id);
    }
}

impl InteractionBroker {
    pub fn new(store: Arc<dyn InteractionStore>, kinds: Arc<InteractionKinds>) -> Self {
        Self {
            store,
            kinds,
            clock: Arc::new(SystemClock),
            timeout: DEFAULT_TIMEOUT,
            waiters: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// How long an interaction stays open before it expires.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn kinds(&self) -> &Arc<InteractionKinds> {
        &self.kinds
    }

    pub fn store(&self) -> &Arc<dyn InteractionStore> {
        &self.store
    }

    fn lock_waiters(&self) -> std::sync::MutexGuard<'_, HashMap<String, oneshot::Sender<()>>> {
        self.waiters.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(test)]
    fn waiter_count(&self) -> usize {
        self.lock_waiters().len()
    }

    /// Put `req` to the owner and wait for how it ends. Returns the new
    /// interaction's id and the outcome.
    ///
    /// Persists a `pending` row, emits `InteractionRequested` on `sink`, then
    /// waits for a same-process [`respond`](Self::respond), a resolution that
    /// another process wrote to the store (polled once a second), or
    /// `expires_at`, whichever comes first, while refreshing `waiter_seen_at`.
    /// Emits `InteractionResolved` before returning. Dropping the future
    /// abandons the wait; the row stays `pending` for the end-of-run cleanup.
    pub async fn request(
        &self,
        req: InteractionRequest,
        sink: Option<&mpsc::Sender<OrchestratorEvent>>,
    ) -> Result<(String, InteractionOutcome)> {
        let Some(handler) = self.kinds.get(&req.kind) else {
            bail!("unknown interaction kind '{}'", req.kind);
        };
        let renders_card = handler.renders_card();
        let mut expires_at = self.clock.now()
            + chrono::Duration::from_std(self.timeout).context("interaction timeout too large")?;
        if let Some(deadline) = req.turn_deadline {
            expires_at = expires_at.min(deadline);
        }

        let row = self
            .store
            .create(NewInteraction {
                id: Uuid::new_v4().to_string(),
                conversation_id: req.conversation_id,
                run_id: req.run_id,
                user_id: req.user_id,
                kind: req.kind,
                tool_call_id: req.tool_call_id,
                request: req.request,
                expires_at,
            })
            .await?;
        let (wake, mut woken) = oneshot::channel();
        self.lock_waiters().insert(row.id.clone(), wake);
        let _guard = WaiterGuard {
            broker: self,
            id: row.id.clone(),
        };

        emit(
            sink,
            OrchestratorEvent::InteractionRequested {
                interaction_id: row.id.clone(),
                kind: row.kind.clone(),
                tool_call_id: row.tool_call_id.clone(),
                created_at: row.created_at,
                expires_at: row.expires_at,
                renders_card,
                request: row.request.clone(),
            },
        )
        .await;

        let settled = self.wait(&row, &mut woken).await?;
        let outcome = InteractionOutcome::from_row(&settled)
            .context("interaction left the wait while still pending")?;
        emit(
            sink,
            OrchestratorEvent::InteractionResolved {
                interaction_id: settled.id.clone(),
                kind: settled.kind.clone(),
                resolved_at: settled.resolved_at.unwrap_or_else(|| self.clock.now()),
                outcome: outcome.clone(),
            },
        )
        .await;
        Ok((settled.id, outcome))
    }

    /// Wait until `row` leaves `pending`; returns its settled row.
    async fn wait(
        &self,
        row: &InteractionRow,
        woken: &mut oneshot::Receiver<()>,
    ) -> Result<InteractionRow> {
        let until_expiry = (row.expires_at - self.clock.now())
            .to_std()
            .unwrap_or(Duration::ZERO);
        let expiry = tokio::time::sleep(until_expiry);
        tokio::pin!(expiry);
        let mut poll = tokio::time::interval(STORE_POLL_EVERY);
        let mut touch = tokio::time::interval(WAITER_TOUCH_EVERY);
        // Both intervals fire at once the first time; the row is fresh.
        poll.tick().await;
        touch.tick().await;
        loop {
            tokio::select! {
                _ = &mut *woken => {}
                _ = poll.tick() => {}
                _ = touch.tick() => {
                    self.store.touch_waiter(&row.id).await?;
                    continue;
                }
                _ = &mut expiry => {
                    return match self
                        .store
                        .resolve_if_pending(&row.id, InteractionStatus::Expired, None, None)
                        .await?
                    {
                        ResolveOutcome::Resolved(r) | ResolveOutcome::AlreadyResolved(r) => Ok(r),
                        ResolveOutcome::NotFound => bail!("interaction '{}' vanished", row.id),
                    };
                }
            }
            match self.store.get(&row.id).await? {
                Some(r) if r.status != InteractionStatus::Pending => return Ok(r),
                Some(_) => {}
                None => bail!("interaction '{}' vanished", row.id),
            }
        }
    }

    /// Record the owner's answer to an interaction.
    ///
    /// Checks the interaction is the caller's and still answerable, validates
    /// `response` through the kind's handler, then compare-and-sets the row.
    /// The same answer twice returns the stored row; a different answer after
    /// resolution is [`RespondError::Conflict`].
    pub async fn respond(
        &self,
        who: &Responder<'_>,
        interaction_id: &str,
        response: Value,
    ) -> Result<InteractionRow, RespondError> {
        let row = self
            .store
            .get(interaction_id)
            .await?
            .filter(|r| who.owns(r))
            .ok_or(RespondError::NotFound)?;

        if row.status != InteractionStatus::Pending {
            return Self::settled_reply(row, &response);
        }
        let now = self.clock.now();
        let stale = now - row.waiter_seen_at
            > chrono::Duration::from_std(WAITER_STALE_AFTER).unwrap_or(chrono::Duration::MAX);
        if row.expires_at <= now || stale {
            self.store
                .resolve_if_pending(&row.id, InteractionStatus::Expired, None, None)
                .await?;
            return Err(RespondError::Gone);
        }
        self.kinds
            .validate_response(&row.kind, &row.request, &response)
            .map_err(RespondError::Invalid)?;

        match self
            .store
            .resolve_if_pending(&row.id, InteractionStatus::Resolved, Some(&response), None)
            .await?
        {
            ResolveOutcome::Resolved(won) => {
                if let Some(wake) = self.lock_waiters().remove(&won.id) {
                    let _ = wake.send(());
                }
                Ok(won)
            }
            ResolveOutcome::AlreadyResolved(lost) => Self::settled_reply(lost, &response),
            ResolveOutcome::NotFound => Err(RespondError::NotFound),
        }
    }

    /// End of run: cancel every `pending` interaction of `run_id` and return
    /// each row of the run whose resolution is not yet in the event log, in
    /// creation order. That includes a row whose answer won the
    /// compare-and-set just before the turn was cancelled; its waiter never
    /// emitted the resolution.
    pub async fn abandon_run(&self, run_id: &str) -> Result<Vec<InteractionRow>> {
        self.store.cancel_pending_for_run(run_id).await?;
        self.store.list_unemitted_for_run(run_id).await
    }

    /// The reply to an answer for a row that already left `pending`.
    fn settled_reply(
        row: InteractionRow,
        response: &Value,
    ) -> Result<InteractionRow, RespondError> {
        match row.status {
            InteractionStatus::Resolved if row.response.as_ref() == Some(response) => Ok(row),
            InteractionStatus::Resolved => Err(RespondError::Conflict(Box::new(row))),
            _ => Err(RespondError::Gone),
        }
    }
}

/// The `InteractionResolved` event for a settled `row`, or `None` while it is
/// still `pending`. The end-of-run cleanup and the restart sweep use it to
/// write resolutions the waiter never got to emit.
pub fn resolved_event(row: &InteractionRow) -> Option<OrchestratorEvent> {
    Some(OrchestratorEvent::InteractionResolved {
        interaction_id: row.id.clone(),
        kind: row.kind.clone(),
        resolved_at: row.resolved_at.unwrap_or_else(|| SystemClock.now()),
        outcome: InteractionOutcome::from_row(row)?,
    })
}

/// Send `event` to the turn's sink, if it has one. A closed sink is not an
/// error: the interaction still resolves through the store.
async fn emit(sink: Option<&mpsc::Sender<OrchestratorEvent>>, event: OrchestratorEvent) {
    if let Some(sink) = sink
        && sink.send(event).await.is_err()
    {
        debug!("event sink closed; interaction continues without it");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant as StdInstant;

    use chrono::TimeZone;
    use serde_json::json;
    use tokio::task::JoinHandle;

    use assistant_storage::InMemoryInteractionStore;

    use super::*;

    /// Wall-clock time that moves with Tokio's (paused) clock, so the store's
    /// timestamps and the broker's sleeps agree.
    struct TokioClock {
        base: DateTime<Utc>,
        start: tokio::time::Instant,
    }

    impl TokioClock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                base: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap(),
                start: tokio::time::Instant::now(),
            })
        }
    }

    impl Clock for TokioClock {
        fn now(&self) -> DateTime<Utc> {
            self.base + chrono::Duration::from_std(self.start.elapsed()).unwrap()
        }
        fn now_instant(&self) -> StdInstant {
            tokio::time::Instant::now().into_std()
        }
    }

    struct Fixture {
        broker: Arc<InteractionBroker>,
        store: Arc<InMemoryInteractionStore>,
        clock: Arc<TokioClock>,
        tx: mpsc::Sender<OrchestratorEvent>,
        rx: mpsc::Receiver<OrchestratorEvent>,
    }

    fn fixture() -> Fixture {
        let clock = TokioClock::new();
        let store = Arc::new(InMemoryInteractionStore::new().with_clock(clock.clone()));
        let broker = Arc::new(
            InteractionBroker::new(store.clone(), Arc::new(InteractionKinds::with_builtin()))
                .with_clock(clock.clone()),
        );
        let (tx, rx) = mpsc::channel(32);
        Fixture {
            broker,
            store,
            clock,
            tx,
            rx,
        }
    }

    fn request() -> InteractionRequest {
        InteractionRequest {
            conversation_id: "conv".into(),
            run_id: "run".into(),
            user_id: "alice".into(),
            kind: "tool_approval".into(),
            tool_call_id: Some("toolu_1".into()),
            request: json!({
                "tool_name": "bash", "arguments": {}, "arguments_truncated": false,
                "reason": "ask", "options": ["allow_once", "reject_once"],
            }),
            turn_deadline: None,
        }
    }

    fn owner() -> Responder<'static> {
        Responder {
            user_id: "alice",
            conversation_id: "conv",
            run_id: "run",
        }
    }

    fn allow() -> Value {
        json!({"decision": "allow_once"})
    }

    type Waiting = JoinHandle<Result<(String, InteractionOutcome)>>;

    /// Start a request and wait until it is announced. Returns the id.
    async fn start(f: &mut Fixture, req: InteractionRequest) -> (String, Waiting) {
        let broker = f.broker.clone();
        let tx = f.tx.clone();
        let task = tokio::spawn(async move { broker.request(req, Some(&tx)).await });
        match f.rx.recv().await {
            Some(OrchestratorEvent::InteractionRequested { interaction_id, .. }) => {
                (interaction_id, task)
            }
            other => panic!("expected InteractionRequested, got {other:?}"),
        }
    }

    async fn status(f: &Fixture, id: &str) -> InteractionStatus {
        f.store.get(id).await.unwrap().unwrap().status
    }

    #[tokio::test(start_paused = true)]
    async fn request_persists_pending_and_announces() {
        let mut f = fixture();
        let (id, _task) = start(&mut f, request()).await;
        let row = f.store.get(&id).await.unwrap().unwrap();
        assert_eq!(row.status, InteractionStatus::Pending);
        assert_eq!(
            (row.user_id.as_str(), row.run_id.as_str()),
            ("alice", "run")
        );
        assert_eq!(
            row.expires_at - row.created_at,
            chrono::Duration::seconds(600),
            "default expiry is ten minutes"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn announcement_carries_the_wire_fields() {
        let mut f = fixture();
        let (broker, tx) = (f.broker.clone(), f.tx.clone());
        let _task = tokio::spawn(async move { broker.request(request(), Some(&tx)).await });
        let rx = &mut f.rx;
        let Some(OrchestratorEvent::InteractionRequested {
            kind,
            tool_call_id,
            renders_card,
            request,
            created_at,
            expires_at,
            ..
        }) = rx.recv().await
        else {
            panic!("expected InteractionRequested");
        };
        assert_eq!(kind, "tool_approval");
        assert_eq!(tool_call_id.as_deref(), Some("toolu_1"));
        assert!(renders_card);
        assert_eq!(request["tool_name"], "bash");
        assert!(expires_at > created_at);
    }

    #[tokio::test(start_paused = true)]
    async fn abandon_run_cancels_pending_and_returns_every_unemitted_row() {
        let mut f = fixture();
        let (pending, _task) = start(&mut f, request()).await;
        // A second row of the run whose answer won just before the turn ended.
        let (answered, task) = start(&mut f, request()).await;
        f.broker
            .respond(&owner(), &answered, allow())
            .await
            .unwrap();
        task.await.unwrap().unwrap();
        while f.rx.try_recv().is_ok() {}
        // A third, already written to the log, and one of another run.
        let (logged, task) = start(&mut f, request()).await;
        f.broker.respond(&owner(), &logged, allow()).await.unwrap();
        task.await.unwrap().unwrap();
        while f.rx.try_recv().is_ok() {}
        f.store.mark_emitted(&logged).await.unwrap();
        let mut other = request();
        other.run_id = "other-run".into();
        let (elsewhere, _t) = start(&mut f, other).await;

        let rows = f.broker.abandon_run("run").await.unwrap();

        let got: Vec<(&str, InteractionStatus)> =
            rows.iter().map(|r| (r.id.as_str(), r.status)).collect();
        assert_eq!(
            got,
            vec![
                (pending.as_str(), InteractionStatus::Cancelled),
                (answered.as_str(), InteractionStatus::Resolved),
            ],
            "the cancelled row and the won-then-cancelled row, not the emitted or foreign ones"
        );
        assert_eq!(status(&f, &elsewhere).await, InteractionStatus::Pending);
    }

    #[tokio::test(start_paused = true)]
    async fn same_process_respond_resumes_the_waiter_at_once() {
        let mut f = fixture();
        let (id, task) = start(&mut f, request()).await;
        let began = tokio::time::Instant::now();

        let row = f.broker.respond(&owner(), &id, allow()).await.unwrap();
        assert_eq!(row.status, InteractionStatus::Resolved);

        let (got_id, outcome) = task.await.unwrap().unwrap();
        assert_eq!(got_id, id);
        assert_eq!(
            outcome,
            InteractionOutcome::Resolved {
                response: allow(),
                rule_id: None
            }
        );
        assert!(
            began.elapsed() < Duration::from_millis(500),
            "the oneshot, not the poll, woke the waiter"
        );
        assert!(matches!(
            f.rx.recv().await,
            Some(OrchestratorEvent::InteractionResolved {
                outcome: InteractionOutcome::Resolved { .. },
                ..
            })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn resolution_written_to_the_store_resumes_the_waiter_within_two_seconds() {
        let mut f = fixture();
        let (id, task) = start(&mut f, request()).await;
        let began = tokio::time::Instant::now();

        // Another process answers: straight to the store, no local oneshot.
        f.store
            .resolve_if_pending(&id, InteractionStatus::Resolved, Some(&allow()), None)
            .await
            .unwrap();

        let (_, outcome) = task.await.unwrap().unwrap();
        assert!(matches!(outcome, InteractionOutcome::Resolved { .. }));
        assert!(
            began.elapsed() <= Duration::from_secs(2),
            "{:?}",
            began.elapsed()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_waiter_refreshes_waiter_seen_at_every_fifteen_seconds() {
        let mut f = fixture();
        let (id, _task) = start(&mut f, request()).await;
        let created = f.store.get(&id).await.unwrap().unwrap().waiter_seen_at;

        tokio::time::sleep(Duration::from_secs(16)).await;
        let seen = f.store.get(&id).await.unwrap().unwrap().waiter_seen_at;
        assert!(
            seen - created >= chrono::Duration::seconds(15),
            "refreshed at the 15 s tick: {created} -> {seen}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn expiry_emits_expired_and_returns_it() {
        let mut f = fixture();
        let (id, task) = start(&mut f, request()).await;

        let (_, outcome) = task.await.unwrap().unwrap();
        assert_eq!(outcome, InteractionOutcome::Expired);
        assert_eq!(status(&f, &id).await, InteractionStatus::Expired);
        assert!(matches!(
            f.rx.recv().await,
            Some(OrchestratorEvent::InteractionResolved {
                outcome: InteractionOutcome::Expired,
                ..
            })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn expires_at_is_clamped_to_the_turn_deadline() {
        let mut f = fixture();
        let mut req = request();
        req.turn_deadline = Some(f.clock.now() + chrono::Duration::seconds(90));
        let (id, task) = start(&mut f, req).await;
        let row = f.store.get(&id).await.unwrap().unwrap();
        assert_eq!(
            row.expires_at - row.created_at,
            chrono::Duration::seconds(90)
        );

        let began = tokio::time::Instant::now();
        let (_, outcome) = task.await.unwrap().unwrap();
        assert_eq!(outcome, InteractionOutcome::Expired);
        assert!(began.elapsed() < Duration::from_secs(95));
    }

    #[tokio::test(start_paused = true)]
    async fn only_the_owner_of_the_run_can_respond() {
        let mut f = fixture();
        let (id, _task) = start(&mut f, request()).await;
        let strangers = [
            Responder {
                user_id: "bob",
                ..owner()
            },
            Responder {
                conversation_id: "other",
                ..owner()
            },
            Responder {
                run_id: "other-run",
                ..owner()
            },
        ];
        for who in strangers {
            let err = f.broker.respond(&who, &id, allow()).await.unwrap_err();
            assert!(matches!(err, RespondError::NotFound), "{who:?}: {err:?}");
        }
        let err = f
            .broker
            .respond(&owner(), "no-such-id", allow())
            .await
            .unwrap_err();
        assert!(matches!(err, RespondError::NotFound));
        assert_eq!(status(&f, &id).await, InteractionStatus::Pending);
    }

    #[tokio::test(start_paused = true)]
    async fn an_answer_that_does_not_fit_leaves_the_interaction_pending() {
        let mut f = fixture();
        let (id, _task) = start(&mut f, request()).await;
        let err = f
            .broker
            .respond(&owner(), &id, json!({"decision": "allow_always"}))
            .await
            .unwrap_err();
        assert!(matches!(err, RespondError::Invalid(_)), "{err:?}");
        assert_eq!(status(&f, &id).await, InteractionStatus::Pending);
    }

    #[tokio::test(start_paused = true)]
    async fn same_answer_twice_is_ok_and_a_different_one_conflicts() {
        let mut f = fixture();
        let (id, _task) = start(&mut f, request()).await;
        let first = f.broker.respond(&owner(), &id, allow()).await.unwrap();
        let again = f.broker.respond(&owner(), &id, allow()).await.unwrap();
        assert_eq!(first, again, "a double tap returns the stored resolution");

        let err = f
            .broker
            .respond(&owner(), &id, json!({"decision": "reject_once"}))
            .await
            .unwrap_err();
        assert!(matches!(err, RespondError::Conflict(_)), "{err:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn a_waiter_that_went_silent_makes_respond_gone_and_expires_the_row() {
        let mut f = fixture();
        let (id, task) = start(&mut f, request()).await;
        task.abort(); // the turn process died: nobody refreshes waiter_seen_at
        let _ = task.await;

        tokio::time::sleep(Duration::from_secs(46)).await;
        let err = f.broker.respond(&owner(), &id, allow()).await.unwrap_err();
        assert!(matches!(err, RespondError::Gone), "{err:?}");
        assert_eq!(status(&f, &id).await, InteractionStatus::Expired);
    }

    #[tokio::test(start_paused = true)]
    async fn answers_after_expiry_or_cancellation_are_gone() {
        let mut f = fixture();
        let (cancelled, _task) = start(&mut f, request()).await;
        f.store.cancel_pending_for_run("run").await.unwrap();
        let err = f
            .broker
            .respond(&owner(), &cancelled, allow())
            .await
            .unwrap_err();
        assert!(matches!(err, RespondError::Gone), "cancelled: {err:?}");

        // A pending row past its expires_at is late even with a live waiter.
        let id = "late-row".to_string();
        f.store
            .create(NewInteraction {
                id: id.clone(),
                conversation_id: "conv".into(),
                run_id: "run".into(),
                user_id: "alice".into(),
                kind: "tool_approval".into(),
                tool_call_id: None,
                request: request().request,
                expires_at: f.clock.now() - chrono::Duration::seconds(1),
            })
            .await
            .unwrap();
        let err = f.broker.respond(&owner(), &id, allow()).await.unwrap_err();
        assert!(matches!(err, RespondError::Gone), "late: {err:?}");
        assert_eq!(status(&f, &id).await, InteractionStatus::Expired);
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_the_run_ends_the_wait_cancelled() {
        let mut f = fixture();
        let (_, task) = start(&mut f, request()).await;
        f.store.cancel_pending_for_run("run").await.unwrap();
        let (_, outcome) = task.await.unwrap().unwrap();
        assert_eq!(outcome, InteractionOutcome::Cancelled);
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_the_waiter_unregisters_it() {
        let mut f = fixture();
        let (_, task) = start(&mut f, request()).await;
        assert_eq!(f.broker.waiter_count(), 1);
        task.abort();
        let _ = task.await;
        assert_eq!(f.broker.waiter_count(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn an_unregistered_kind_cannot_be_requested() {
        let f = fixture();
        let mut req = request();
        req.kind = "future_thing".into();
        let err = f.broker.request(req, None).await.unwrap_err();
        assert!(err.to_string().contains("future_thing"), "{err}");
    }
}
