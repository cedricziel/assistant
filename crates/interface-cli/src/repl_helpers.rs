//! REPL helpers used by the CLI's interactive loop: attachment delivery,
//! token streaming printer, and the in-REPL `/help` text.

use std::io::{self, Write as IoWrite};
use std::path::Path;

use tokio::sync::mpsc;
use uuid::Uuid;

/// Save attachments from a turn result to `~/.assistant/attachments/` and print
/// their file paths so the user knows where to find them.
pub fn deliver_attachments(attachments: &[assistant_core::Attachment], assistant_dir: &Path) {
    let attach_dir = assistant_dir.join("attachments");
    if let Err(e) = std::fs::create_dir_all(&attach_dir) {
        eprintln!("Failed to create attachments directory: {e}");
        return;
    }

    for attachment in attachments {
        // Disambiguate filenames by prepending a short UUID prefix.
        let unique_name = format!(
            "{}_{}",
            &Uuid::new_v4().to_string()[..8],
            attachment.filename
        );
        let dest = attach_dir.join(&unique_name);

        match std::fs::write(&dest, &attachment.data) {
            Ok(()) => {
                let size = attachment.data.len();
                let kind = if attachment.is_image() {
                    "image"
                } else {
                    "file"
                };
                println!(
                    "  [{kind}] {} ({}, {size} bytes)",
                    dest.display(),
                    attachment.mime_type,
                );
            }
            Err(e) => {
                eprintln!("Failed to save attachment '{}': {e}", attachment.filename);
            }
        }
    }
}

/// Spawn a background task that prints tokens from `rx` to stdout as they
/// arrive.  Returns a join handle; the task exits when the channel is closed
/// (i.e. when the orchestrator drops its `Sender`).
pub fn start_token_printer(
    mut rx: mpsc::Receiver<assistant_runtime::OrchestratorEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use assistant_runtime::projection::{CliProjector, StreamProjector};
        let mut stdout = io::stdout();
        while let Some(event) = rx.recv().await {
            for line in CliProjector.project(&event) {
                print!("{line}");
                let _ = stdout.flush();
            }
        }
        // Trailing newline so the next prompt appears on its own line.
        println!("\n");
        let _ = stdout.flush();
    })
}

pub fn print_help() {
    println!(
        "\nAssistant REPL commands:\n\
         \n\
         /new                          Start a new conversation\n\
         /stop                         Cancel the current turn\n\
         /model [name]                 Show or switch the model for this conversation\n\
         /compact                      Compress conversation context\n\
         /status                       Show conversation status\n\
         /help                         Show this help message\n\
         /skills [name]                List all skills, or show detail for one\n\
         /review                       Review pending skill refinement proposals\n\
         /install <path|owner/repo>    Install a skill from disk or GitHub\n\
         /quit | /exit                 Exit the assistant\n\
         \n\
         Any other input is sent to the AI assistant.\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Attachment;

    #[test]
    fn deliver_attachments_writes_files_under_attachments_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let attach = Attachment::new("photo.png", "image/png", b"binary".to_vec());
        deliver_attachments(&[attach], dir.path());

        let attach_dir = dir.path().join("attachments");
        assert!(attach_dir.exists(), "attachments subdir created");
        let entries: Vec<_> = std::fs::read_dir(&attach_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);

        let entry = entries.into_iter().next().unwrap().unwrap();
        let name = entry.file_name().into_string().unwrap();
        assert!(name.ends_with("_photo.png"), "got: {name}");
        // Filename starts with an 8-char UUID prefix + underscore.
        assert!(name.len() > "photo.png".len() + 1);
        let bytes = std::fs::read(entry.path()).unwrap();
        assert_eq!(bytes, b"binary");
    }

    #[test]
    fn deliver_attachments_unique_names_for_same_filename() {
        let dir = tempfile::tempdir().unwrap();
        let a1 = Attachment::new("doc.txt", "text/plain", b"first".to_vec());
        let a2 = Attachment::new("doc.txt", "text/plain", b"second".to_vec());
        deliver_attachments(&[a1, a2], dir.path());

        let attach_dir = dir.path().join("attachments");
        let count = std::fs::read_dir(&attach_dir).unwrap().count();
        assert_eq!(count, 2, "duplicate filenames must coexist via UUID prefix");
    }

    #[test]
    fn deliver_attachments_handles_empty_input() {
        let dir = tempfile::tempdir().unwrap();
        // Empty input: function should still create the dir but no files.
        deliver_attachments(&[], dir.path());
        let attach_dir = dir.path().join("attachments");
        assert!(attach_dir.exists());
        assert_eq!(std::fs::read_dir(&attach_dir).unwrap().count(), 0);
    }

    #[test]
    fn print_help_runs_without_panic() {
        // Just ensures the formatted string compiles + executes.
        print_help();
    }

    #[tokio::test]
    async fn start_token_printer_exits_when_channel_closed() {
        use assistant_runtime::OrchestratorEvent;
        let (tx, rx) = mpsc::channel(4);
        let handle = start_token_printer(rx);

        tx.send(OrchestratorEvent::Token("hi".to_string()))
            .await
            .unwrap();
        // Drop the sender — the task should finish.
        drop(tx);
        handle.await.unwrap();
    }
}
