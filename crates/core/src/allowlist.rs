//! Allowlist filtering for interface message dispatchers.

/// A simple allowlist gate for string IDs (channels, users, rooms, senders).
///
/// An **empty** allowlist means "all IDs are permitted".
/// A **non-empty** allowlist means "only these IDs are permitted".
///
/// # Example
///
/// ```rust
/// use assistant_core::AllowlistFilter;
///
/// let filter = AllowlistFilter::new(vec!["bot-test".to_string()]);
/// assert!(filter.is_allowed("bot-test"));
/// assert!(!filter.is_allowed("general"));
///
/// // Empty allowlist accepts everything.
/// let open = AllowlistFilter::default();
/// assert!(open.is_allowed("any-channel"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct AllowlistFilter {
    allowed: Vec<String>,
}

impl AllowlistFilter {
    /// Create a new filter from an explicit list.
    ///
    /// Pass an empty `Vec` to create an open filter (accepts all IDs).
    pub fn new(allowed: Vec<String>) -> Self {
        Self { allowed }
    }

    /// Returns `true` when `id` is permitted by this filter.
    pub fn is_allowed(&self, id: &str) -> bool {
        self.allowed.is_empty() || self.allowed.iter().any(|a| a == id)
    }
}

#[cfg(test)]
mod tests {
    use super::AllowlistFilter;

    #[test]
    fn empty_allowlist_accepts_all() {
        let f = AllowlistFilter::default();
        assert!(f.is_allowed("general"));
        assert!(f.is_allowed("bot-test"));
        assert!(f.is_allowed(""));
    }

    #[test]
    fn non_empty_allowlist_accepts_known_id() {
        let f = AllowlistFilter::new(vec!["bot-test".to_string()]);
        assert!(f.is_allowed("bot-test"));
    }

    #[test]
    fn non_empty_allowlist_blocks_unknown_id() {
        let f = AllowlistFilter::new(vec!["bot-test".to_string()]);
        assert!(!f.is_allowed("general"));
    }

    #[test]
    fn multiple_allowed_ids() {
        let f = AllowlistFilter::new(vec!["a".to_string(), "b".to_string()]);
        assert!(f.is_allowed("a"));
        assert!(f.is_allowed("b"));
        assert!(!f.is_allowed("c"));
    }
}
