//! `assistant webui serve` — thin shim that forwards to `assistant-web-ui`.

use anyhow::Result;

use crate::args::WebUiCommand;

pub async fn cmd_webui(command: &WebUiCommand) -> Result<()> {
    let args = match command {
        WebUiCommand::Serve { args } => args,
    };

    let argv = std::iter::once("assistant-web-ui".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    assistant_web_ui::run_from_iter(argv).await
}
