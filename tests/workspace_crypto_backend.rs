//! Asserts the workspace compiles a single TLS/crypto backend: ring.
//!
//! `aws-lc-rs`/`aws-lc-sys` build a C library through cmake, slowing every cold
//! build, and having both backends in the tree makes rustls unable to pick a
//! default provider. Configure the offending dependency with its `ring` (or
//! `*-no-provider`) feature instead. `deny.toml` bans the same crates; this test
//! fails earlier and without cargo-deny.

use std::fs;
use std::path::PathBuf;

use toml::Value;

const BANNED_PACKAGES: &[&str] = &["aws-lc-rs", "aws-lc-sys", "cmake"];

#[test]
fn cargo_lock_has_no_aws_lc_or_cmake() {
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let raw = fs::read_to_string(&lock_path).expect("read Cargo.lock");
    let lock: Value = toml::from_str(&raw).expect("parse Cargo.lock");

    let present: Vec<&str> = lock
        .get("package")
        .and_then(Value::as_array)
        .expect("Cargo.lock has no [[package]] entries")
        .iter()
        .filter_map(|p| p.get("name").and_then(Value::as_str))
        .filter(|name| BANNED_PACKAGES.contains(name))
        .collect();

    assert!(
        present.is_empty(),
        "Cargo.lock contains {present:?}; ring is the only allowed crypto backend"
    );
}
