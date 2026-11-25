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

/// Argon2id hash of a throwaway password, with the default cost parameters.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$uCaKNJGrnRj4jbFWtWTxog$9FhC/RiJVH8/HLMmzJ84JWfCI82gJD0oCPDTIE+x4g0";

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

/// Spend the cost of a real verification on a fixed hash, so callers can
/// answer "no such user" in the same time as "wrong password".
pub fn verify_dummy(plain: &str) {
    let _ = verify_password(plain, DUMMY_HASH);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_hash_uses_the_same_cost_parameters_as_real_hashes() {
        let real = hash_password("x").unwrap();
        let real = PasswordHash::new(&real).unwrap();
        let dummy = PasswordHash::new(DUMMY_HASH).unwrap();
        assert_eq!(dummy.algorithm, real.algorithm);
        assert_eq!(dummy.version, real.version);
        assert_eq!(dummy.params, real.params);
    }

    #[test]
    fn dummy_verification_never_matches() {
        assert!(!verify_password("anything", DUMMY_HASH).unwrap());
        assert!(!verify_password("", DUMMY_HASH).unwrap());
    }

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
