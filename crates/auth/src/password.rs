//! Password hashing and verification using argon2id.
//!
//! Uses the OWASP-recommended argon2id algorithm with sensible defaults.

use anyhow::{Result, bail};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

/// Hash a plaintext password using argon2id.
///
/// Returns the PHC-formatted hash string (includes algorithm, salt, and hash).
pub fn hash_password(plain: &str) -> Result<String> {
    if plain.is_empty() {
        bail!("password must not be empty");
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // argon2id with default params

    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hashing failed: {e}"))?;

    Ok(hash.to_string())
}

/// Verify a plaintext password against a PHC-formatted argon2id hash.
pub fn verify_password(plain: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("invalid password hash format: {e}"))?;

    let argon2 = Argon2::default();
    match argon2.verify_password(plain.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("password verification error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_correct_password() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails_verification() {
        let hash = hash_password("hunter2").unwrap();
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn different_hashes_for_same_input() {
        let h1 = hash_password("same-password").unwrap();
        let h2 = hash_password("same-password").unwrap();
        // Different salts → different hashes.
        assert_ne!(h1, h2);
        // But both verify.
        assert!(verify_password("same-password", &h1).unwrap());
        assert!(verify_password("same-password", &h2).unwrap());
    }

    #[test]
    fn empty_password_is_rejected() {
        assert!(hash_password("").is_err());
    }

    #[test]
    fn hash_is_phc_format() {
        let hash = hash_password("test").unwrap();
        // PHC format starts with $argon2id$
        assert!(hash.starts_with("$argon2id$"), "unexpected format: {hash}");
    }

    #[test]
    fn invalid_hash_format_returns_error() {
        assert!(verify_password("test", "not-a-hash").is_err());
    }
}
