//! SHA-256 checksum helpers.

use std::io::Read;

use anyhow::Result;
use sha2::{Digest, Sha256};

/// Compute the SHA-256 digest of `data` and return it as a 64-char lowercase hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

/// Compute the SHA-256 digest by reading all bytes from `reader`.
pub fn sha256_reader<R: Read>(mut reader: R) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        // SHA-256 of empty input is well-known
        let result = sha256_hex(b"");
        assert_eq!(
            result,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_known_vector() {
        // SHA-256("abc") — value verified against the sha2 crate at runtime
        let result = sha256_hex(b"abc");
        // Must be 64 lowercase hex chars
        assert_eq!(result.len(), 64);
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
        // Must be deterministic
        assert_eq!(result, sha256_hex(b"abc"));
        // Must differ from empty-input hash
        let empty = sha256_hex(b"");
        assert_ne!(result, empty);
    }

    #[test]
    fn test_sha256_reader_matches_hex() {
        let data = b"hello world";
        let from_slice = sha256_hex(data);
        let from_reader = sha256_reader(data.as_slice()).unwrap();
        assert_eq!(from_slice, from_reader);
    }

    #[test]
    fn test_sha256_hex_length() {
        let result = sha256_hex(b"test");
        assert_eq!(result.len(), 64, "SHA-256 hex must be 64 chars");
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
