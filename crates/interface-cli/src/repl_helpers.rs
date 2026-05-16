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
        use assistant_runtime::OrchestratorEvent;
        let mut stdout = io::stdout();
        while let Some(event) = rx.recv().await {
            if let OrchestratorEvent::Token(token) = event {
                print!("{token}");
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
