//! HMAC-SHA256 signing and verification for Nextcloud Talk Bot API.
//!
//! All webhook requests from Nextcloud are signed, and all outgoing bot
//! requests must also be signed using the shared secret.

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verify an incoming webhook request signature.
///
/// Computes `HMAC-SHA256(secret, random + body)` and compares it to the
/// provided signature using constant-time comparison.
pub fn verify_signature(secret: &str, random: &str, body: &[u8], signature: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(random.as_bytes());
    mac.update(body);

    let expected = hex::encode(mac.finalize().into_bytes());
    constant_time_eq(expected.as_bytes(), signature.to_lowercase().as_bytes())
}

/// Generate a signature for an outgoing bot request.
///
/// Returns `(random, signature)` where `random` is a 64-char hex string
/// and `signature` is the HMAC-SHA256 of `random + body` using the secret.
pub fn sign_request(secret: &str, body: &str) -> Result<(String, String)> {
    let random = generate_random().context("failed to generate random bytes for HMAC signing")?;
    let signature = compute_signature(secret, &random, body.as_bytes());
    Ok((random, signature))
}

/// Compute the HMAC-SHA256 signature of `random + body` with the given secret.
fn compute_signature(secret: &str, random: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(random.as_bytes());
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// Generate a 64-character hex random string.
fn generate_random() -> Result<String> {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).context("getrandom failed")?;
    Ok(hex::encode(buf))
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let secret = "my-secret-key";
        let body = r#"{"type":"Create","actor":{"type":"Person"}}"#;

        let (random, signature) = sign_request(secret, body).unwrap();
        assert!(verify_signature(
            secret,
            &random,
            body.as_bytes(),
            &signature
        ));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let body = "hello";
        let (random, signature) = sign_request("correct-secret", body).unwrap();
        assert!(!verify_signature(
            "wrong-secret",
            &random,
            body.as_bytes(),
            &signature
        ));
    }

    #[test]
    fn verify_rejects_tampered_body() {
        let secret = "secret";
        let (random, signature) = sign_request(secret, "original").unwrap();
        assert!(!verify_signature(secret, &random, b"tampered", &signature));
    }

    #[test]
    fn verify_case_insensitive_signature() {
        let secret = "test";
        let body = "data";
        let (random, signature) = sign_request(secret, body).unwrap();
        let upper = signature.to_uppercase();
        assert!(verify_signature(secret, &random, body.as_bytes(), &upper));
    }
}
