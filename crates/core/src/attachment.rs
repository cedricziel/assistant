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
/// LLM integration path and resize pipeline currently handle.  Expand as new
/// content types are supported (e.g. `application/pdf`).
pub const SUPPORTED_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];

/// MIME types that support server-side resizing via the `image` crate.
pub const RESIZABLE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Maximum attachment size in bytes (10 MB).
pub const MAX_ATTACHMENT_SIZE: u64 = 10 * 1024 * 1024;

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

/// Map a MIME type to a canonical file extension.
pub fn extension_for_mime(mime_type: &str) -> &str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
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
    }

    #[test]
    fn extension_fallback() {
        assert_eq!(extension_for_mime("application/pdf"), "bin");
        assert_eq!(extension_for_mime("text/plain"), "bin");
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
    fn is_supported_mime_type_accepts_images() {
        assert!(is_supported_mime_type("image/png"));
        assert!(is_supported_mime_type("image/jpeg"));
        assert!(is_supported_mime_type("image/gif"));
        assert!(is_supported_mime_type("image/webp"));
    }

    #[test]
    fn is_supported_mime_type_rejects_others() {
        assert!(!is_supported_mime_type("application/pdf"));
        assert!(!is_supported_mime_type("text/plain"));
        assert!(!is_supported_mime_type("image/svg+xml"));
    }

    #[test]
    fn is_resizable_matches_supported() {
        for mime in SUPPORTED_MIME_TYPES {
            assert!(is_resizable_mime_type(mime), "{mime} should be resizable");
        }
    }

    #[test]
    fn constants_are_consistent() {
        assert_eq!(SUPPORTED_MIME_TYPES.len(), 4);
        assert_eq!(RESIZABLE_MIME_TYPES.len(), 4);
        assert_eq!(MAX_ATTACHMENT_SIZE, 10 * 1024 * 1024);
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
