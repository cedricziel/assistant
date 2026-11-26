//! With no tracer provider installed, the saved final-answer row carries no
//! traceparent.
//!
//! Own test binary: `final_answer_traceparent.rs` installs a global provider.

#[path = "support/final_answer_rig.rs"]
mod final_answer_rig;

#[tokio::test]
async fn final_answer_row_has_no_traceparent_without_a_tracer_provider() {
    let answer = final_answer_rig::run_turn_and_load_answer().await;
    assert_eq!(answer.traceparent, None);
}
