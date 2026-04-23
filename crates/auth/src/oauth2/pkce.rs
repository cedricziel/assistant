//! PKCE (Proof Key for Code Exchange) per RFC 7636.
//!
//! Implements the S256 challenge method used to protect the authorization
//! code grant against interception attacks.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// Generate a cryptographically random code verifier (43-128 chars, URL-safe).
pub fn generate_verifier() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Compute the S256 challenge from a verifier: `BASE64URL(SHA256(verifier))`.
pub fn challenge_from_verifier(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Verify that a verifier matches a previously stored S256 challenge.
pub fn verify(verifier: &str, challenge: &str) -> bool {
    challenge_from_verifier(verifier) == challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_is_url_safe_and_correct_length() {
        let verifier = generate_verifier();
        // 32 bytes -> 43 base64url chars (no padding)
        assert_eq!(verifier.len(), 43, "verifier should be 43 chars");
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "verifier should be URL-safe: {verifier}"
        );
    }

    #[test]
    fn challenge_round_trip() {
        let verifier = generate_verifier();
        let challenge = challenge_from_verifier(&verifier);
        assert!(
            verify(&verifier, &challenge),
            "verifier should match its own challenge"
        );
    }

    #[test]
    fn wrong_verifier_fails() {
        let verifier = generate_verifier();
        let challenge = challenge_from_verifier(&verifier);
        let wrong = generate_verifier();
        assert!(
            !verify(&wrong, &challenge),
            "different verifier should not match"
        );
    }

    #[test]
    fn different_verifiers_produce_different_challenges() {
        let v1 = generate_verifier();
        let v2 = generate_verifier();
        assert_ne!(
            challenge_from_verifier(&v1),
            challenge_from_verifier(&v2),
            "different verifiers should produce different challenges"
        );
    }

    #[test]
    fn rfc7636_test_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(
            challenge_from_verifier(verifier),
            expected,
            "should match RFC 7636 Appendix B test vector"
        );
    }
}
