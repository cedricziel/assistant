//! Shared helpers for resolving file-upload content from tool parameters.
//!
//! Used by interface crates (Slack, Mattermost, etc.) that accept uploads
//! via `path`, `content_base64`, or `content` parameters.

use std::collections::HashMap;

use serde_json::Value;

/// Resolve upload bytes from tool parameters.
///
/// Checks the following keys in priority order:
/// 1. `path` — reads the file from disk.
/// 2. `content_base64` — decodes base64 content.
/// 3. `content` — interprets as UTF-8 text bytes.
///
/// Handles the following real-world LLM encoding quirks for `content_base64`:
/// - Data-URI prefixes such as `data:image/png;base64,` are stripped.
/// - ASCII whitespace (newlines, spaces) inserted for readability is stripped.
/// - Missing trailing `=` padding is tolerated via a `STANDARD_NO_PAD`
///   fallback so both padded and unpadded base64 are accepted.
pub fn resolve_upload_bytes(params: &HashMap<String, Value>) -> Result<Vec<u8>, String> {
    // path -> read file from disk (highest priority, LLM should always use this for binary)
    if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
        return std::fs::read(path).map_err(|e| format!("Cannot read file at path '{path}': {e}"));
    }

    if let Some(b64) = params.get("content_base64").and_then(|v| v.as_str()) {
        use base64::Engine as _;

        // Strip "data:<mime>;base64," prefix produced by some callers.
        let b64 = match b64.find(";base64,") {
            Some(idx) => &b64[idx + ";base64,".len()..],
            None => b64,
        };

        // Remove ASCII whitespace (newlines, spaces) that LLMs commonly
        // insert into long base64 strings for readability.
        let b64_clean: String = b64.chars().filter(|c| !c.is_ascii_whitespace()).collect();

        // Try padded decode first; fall back to no-pad for inputs where the
        // trailing '=' characters have been omitted by the LLM.
        base64::engine::general_purpose::STANDARD
            .decode(&b64_clean)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&b64_clean))
            .map_err(|e| format!("Invalid base64 in content_base64: {e}"))
    } else if let Some(text) = params.get("content").and_then(|v| v.as_str()) {
        Ok(text.as_bytes().to_vec())
    } else {
        Err("Either 'path', 'content', or 'content_base64' must be provided".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_upload_bytes;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn upload_bytes_from_text_content() {
        let p = params(&[("content", json!("hello world"))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn upload_bytes_from_base64_content() {
        use base64::Engine as _;
        let original = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        let p = params(&[("content_base64", json!(encoded))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(bytes, original);
    }

    #[test]
    fn upload_bytes_base64_takes_priority() {
        use base64::Engine as _;
        let binary = vec![1, 2, 3];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&binary);
        let p = params(&[
            ("content", json!("text fallback")),
            ("content_base64", json!(encoded)),
        ]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, binary,
            "base64 should take priority when both present"
        );
    }

    #[test]
    fn upload_bytes_from_path() {
        let tmp_path = std::env::temp_dir().join("core_upload_test_path.bin");
        std::fs::write(&tmp_path, b"file from path").unwrap();
        let p = params(&[("path", json!(tmp_path.to_str().unwrap()))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
        assert_eq!(bytes, b"file from path");
    }

    #[test]
    fn upload_bytes_path_takes_priority_over_content() {
        let tmp_path = std::env::temp_dir().join("core_upload_test_priority.bin");
        std::fs::write(&tmp_path, b"from path").unwrap();
        let p = params(&[
            ("path", json!(tmp_path.to_str().unwrap())),
            ("content", json!("from content")),
        ]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
        assert_eq!(
            bytes, b"from path",
            "path should take priority over content"
        );
    }

    #[test]
    fn upload_bytes_path_nonexistent_returns_error() {
        let p = params(&[("path", json!("/nonexistent/file/path.bin"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(
            err.contains("Cannot read file at path"),
            "error should mention path: {err}"
        );
    }

    #[test]
    fn upload_bytes_missing_both_returns_error() {
        let p = params(&[("filename", json!("test.txt"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(err.contains("content"), "error should mention content");
    }

    #[test]
    fn upload_bytes_invalid_base64_returns_error() {
        let p = params(&[("content_base64", json!("not-valid-base64!!!"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(err.contains("Invalid base64"));
    }

    #[test]
    fn upload_bytes_empty_text_returns_empty_vec() {
        let p = params(&[("content", json!(""))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn upload_bytes_empty_base64_returns_empty_vec() {
        let p = params(&[("content_base64", json!(""))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn upload_bytes_base64_with_newlines_decodes_correctly() {
        use base64::Engine as _;
        let original = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG magic
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        // Simulate an LLM inserting a newline in the middle of the string.
        let with_newline = format!("{}\n{}", &encoded[..4], &encoded[4..]);
        let p = params(&[("content_base64", json!(with_newline))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "whitespace in base64 should be stripped before decoding"
        );
    }

    #[test]
    fn upload_bytes_base64_without_padding_decodes_correctly() {
        use base64::Engine as _;
        let original = vec![1u8, 2]; // canonical base64 requires '=' padding
        let padded = base64::engine::general_purpose::STANDARD.encode(&original);
        let encoded = padded.trim_end_matches('=').to_string();
        assert!(
            !encoded.contains('='),
            "test setup: encoded must lack padding"
        );
        let p = params(&[("content_base64", json!(encoded))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "base64 without trailing '=' padding should decode correctly"
        );
    }

    #[test]
    fn upload_bytes_data_uri_prefix_stripped() {
        use base64::Engine as _;
        let original = vec![0x89u8, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        let data_uri = format!("data:image/png;base64,{encoded}");
        let p = params(&[("content_base64", json!(data_uri))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "data-URI prefix should be stripped before base64 decoding"
        );
    }
}
