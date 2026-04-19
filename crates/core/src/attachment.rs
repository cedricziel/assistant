//! Persisted attachment metadata and validation helpers.
//!
//! The actual file bytes live on the filesystem under
//! `~/.assistant/agents/{agent_id}/attachments/{conversation_id}/{id}.{ext}`.
//! Only metadata is stored in SQLite.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// -- Constants ----------------------------------------------------------------

/// MIME types accepted for attachment upload.
///
/// The storage layer and API are file-type-agnostic — this list gates what the
/// upload endpoint allows.  Images go through the resize pipeline; documents
/// and text files are forwarded to the LLM via `ContentBlock::Document` or
/// inlined `ContentBlock::Text`.
pub const SUPPORTED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "application/pdf",
    "text/plain",
    "text/markdown",
    "text/csv",
    "application/json",
];

/// MIME types that support server-side resizing via the `image` crate.
pub const RESIZABLE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Maximum attachment size in bytes (25 MB).
pub const MAX_ATTACHMENT_SIZE: u64 = 25 * 1024 * 1024;

// -- AttachmentMeta -----------------------------------------------------------

/// Persisted metadata for a file attachment.  The actual bytes live on disk.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AttachmentMeta {
    pub id: Uuid,
    /// `None` during upload — set when the message is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Uuid>,
    pub conversation_id: Uuid,
    pub agent_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

impl AttachmentMeta {
    /// File extension derived from `mime_type`.
    pub fn extension(&self) -> &str {
        extension_for_mime(&self.mime_type)
    }
}

// -- Helpers ------------------------------------------------------------------

/// Returns `true` if `mime_type` is in [`SUPPORTED_MIME_TYPES`].
pub fn is_supported_mime_type(mime_type: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime_type)
}

/// Returns `true` if `mime_type` supports server-side resizing.
pub fn is_resizable_mime_type(mime_type: &str) -> bool {
    RESIZABLE_MIME_TYPES.contains(&mime_type)
}

/// Returns `true` if `mime_type` is a text-based type that can be inlined.
pub fn is_text_mime_type(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "text/plain" | "text/markdown" | "text/csv" | "application/json"
    )
}

/// Map a MIME type to a canonical file extension.
pub fn extension_for_mime(mime_type: &str) -> &str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "text/markdown" => "md",
        "text/csv" => "csv",
        "application/json" => "json",
        _ => "bin",
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_for_known_types() {
        assert_eq!(extension_for_mime("image/png"), "png");
        assert_eq!(extension_for_mime("image/jpeg"), "jpg");
        assert_eq!(extension_for_mime("image/gif"), "gif");
        assert_eq!(extension_for_mime("image/webp"), "webp");
        assert_eq!(extension_for_mime("application/pdf"), "pdf");
        assert_eq!(extension_for_mime("text/plain"), "txt");
        assert_eq!(extension_for_mime("text/markdown"), "md");
        assert_eq!(extension_for_mime("text/csv"), "csv");
        assert_eq!(extension_for_mime("application/json"), "json");
    }

    #[test]
    fn extension_fallback() {
        assert_eq!(extension_for_mime("application/zip"), "bin");
        assert_eq!(extension_for_mime("video/mp4"), "bin");
    }

    #[test]
    fn attachment_meta_extension() {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: Uuid::new_v4(),
            agent_id: "default".into(),
            filename: "photo.jpg".into(),
            mime_type: "image/jpeg".into(),
            size_bytes: 1024,
            created_at: Utc::now(),
        };
        assert_eq!(meta.extension(), "jpg");
    }

    #[test]
    fn is_supported_mime_type_accepts_all_supported() {
        assert!(is_supported_mime_type("image/png"));
        assert!(is_supported_mime_type("image/jpeg"));
        assert!(is_supported_mime_type("image/gif"));
        assert!(is_supported_mime_type("image/webp"));
        assert!(is_supported_mime_type("application/pdf"));
        assert!(is_supported_mime_type("text/plain"));
        assert!(is_supported_mime_type("text/markdown"));
        assert!(is_supported_mime_type("text/csv"));
        assert!(is_supported_mime_type("application/json"));
    }

    #[test]
    fn is_supported_mime_type_rejects_others() {
        assert!(!is_supported_mime_type("application/zip"));
        assert!(!is_supported_mime_type("video/mp4"));
        assert!(!is_supported_mime_type("image/svg+xml"));
    }

    #[test]
    fn is_resizable_only_images() {
        for mime in RESIZABLE_MIME_TYPES {
            assert!(is_resizable_mime_type(mime), "{mime} should be resizable");
        }
        assert!(!is_resizable_mime_type("application/pdf"));
        assert!(!is_resizable_mime_type("text/plain"));
    }

    #[test]
    fn is_text_mime_type_works() {
        assert!(is_text_mime_type("text/plain"));
        assert!(is_text_mime_type("text/markdown"));
        assert!(is_text_mime_type("text/csv"));
        assert!(is_text_mime_type("application/json"));
        assert!(!is_text_mime_type("image/png"));
        assert!(!is_text_mime_type("application/pdf"));
    }

    #[test]
    fn constants_are_consistent() {
        assert_eq!(SUPPORTED_MIME_TYPES.len(), 9);
        assert_eq!(RESIZABLE_MIME_TYPES.len(), 4);
        assert_eq!(MAX_ATTACHMENT_SIZE, 25 * 1024 * 1024);
    }

    #[test]
    fn attachment_meta_serialization_roundtrip() {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: Some(Uuid::new_v4()),
            conversation_id: Uuid::new_v4(),
            agent_id: "test-agent".into(),
            filename: "screenshot.png".into(),
            mime_type: "image/png".into(),
            size_bytes: 245_760,
            created_at: Utc::now(),
        };
        let json = serde_json::to_value(&meta).unwrap();
        let back: AttachmentMeta = serde_json::from_value(json).unwrap();
        assert_eq!(back.id, meta.id);
        assert_eq!(back.message_id, meta.message_id);
        assert_eq!(back.agent_id, meta.agent_id);
    }

    #[test]
    fn attachment_meta_message_id_none_omitted() {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: Uuid::new_v4(),
            agent_id: "default".into(),
            filename: "f.png".into(),
            mime_type: "image/png".into(),
            size_bytes: 100,
            created_at: Utc::now(),
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert!(
            json.get("message_id").is_none(),
            "message_id should be omitted when None"
        );
    }
}
