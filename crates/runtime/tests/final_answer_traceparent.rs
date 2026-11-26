//! The saved final-answer row carries the traceparent of the `chat {model}`
//! span that produced it.
//!
//! Own test binary: it installs the global tracer provider, so it cannot share
//! a process with `final_answer_traceparent_disabled.rs`. One test per file.

#[path = "support/final_answer_rig.rs"]
mod final_answer_rig;

use opentelemetry::global;
use opentelemetry_sdk::trace::InMemorySpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;

#[tokio::test]
async fn final_answer_row_carries_the_chat_span_traceparent() {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    global::set_tracer_provider(provider.clone());

    let answer = final_answer_rig::run_turn_and_load_answer().await;

    let _ = provider.force_flush();
    let spans = exporter.get_finished_spans().unwrap();
    let chat: Vec<_> = spans
        .iter()
        .filter(|s| s.name.starts_with("chat "))
        .collect();
    assert_eq!(chat.len(), 1, "exactly one LLM span: {spans:?}");
    let ctx = &chat[0].span_context;
    assert_eq!(
        answer.traceparent,
        Some(format!("00-{}-{}-01", ctx.trace_id(), ctx.span_id())),
        "saved row carries the chat span's traceparent"
    );
}
