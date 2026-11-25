//! Constant-time comparison for secrets (tokens, PKCE challenges).

use subtle::ConstantTimeEq;

/// Compare two secrets without short-circuiting on the first differing byte.
///
/// Inputs of different length compare unequal; the length itself is not
/// treated as secret.
pub fn eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::eq;

    #[test]
    fn equal_strings_match() {
        assert!(eq("secret-token", "secret-token"));
        assert!(eq("", ""));
    }

    #[test]
    fn different_strings_do_not_match() {
        assert!(!eq("secret-token", "secret-tokeN"));
        assert!(!eq("secret-token", "secret-token-longer"));
        assert!(!eq("secret-token", ""));
    }
}
