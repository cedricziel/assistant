//! Tests for `compute_next_run` — accepts 5- and 7-field cron expressions.

use chrono::{Timelike, Utc};

use crate::scheduler::compute_next_run;

#[test]
fn next_run_seven_field_cron() {
    // "0 0 9 * * *" — 7-field, fires at 09:00 every day
    let next = compute_next_run("0 0 9 * * *");
    assert!(next.is_some(), "should parse 7-field cron");
    let next = next.unwrap();
    assert!(next > Utc::now(), "next run must be in the future");
    assert_eq!(next.time().hour(), 9, "should fire at 09:xx");
}

#[test]
fn next_run_five_field_cron() {
    // "0 9 * * *" — standard 5-field, also fires at 09:00
    let next = compute_next_run("0 9 * * *");
    assert!(next.is_some(), "should parse 5-field cron");
    let next = next.unwrap();
    assert!(next > Utc::now(), "next run must be in the future");
    assert_eq!(next.time().hour(), 9, "should fire at 09:xx");
}

#[test]
fn next_run_invalid_expr_returns_none() {
    assert!(compute_next_run("not a cron").is_none());
}
