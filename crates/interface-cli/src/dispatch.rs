//! One-shot subcommands that run to completion without bootstrapping the
//! orchestrator (reset, persona, backup, login, ...).

use std::path::Path;

use anyhow::Result;

use assistant_core::types::agent::OrgConfig;

use crate::args::{AccountCommand, AdminCommand, ApiKeysCommand, Cli, Command, SkillCommand};
use crate::{
    cmd_account, cmd_admin, cmd_backup, cmd_doctor, cmd_login, cmd_persona, cmd_reset, cmd_skill,
    cmd_webui,
};

/// Run the subcommand in `cli` if it is a one-shot one.
///
/// Returns `None` when the command needs the full runtime (REPL, interfaces,
/// workers, MCP, `skill generate`), so the caller continues bootstrapping.
pub async fn run_one_shot(
    cli: &Cli,
    assistant_dir: &Path,
    db_path: &Path,
    config: &OrgConfig,
) -> Option<Result<()>> {
    let result = match cli.command.as_ref()? {
        Command::Reset { yes } => cmd_reset::cmd_reset(db_path, config, *yes),
        Command::Persona { command } => cmd_persona::cmd_persona(db_path, command).await,
        // Generate needs the full Orchestrator, so it is handled after bootstrap.
        Command::Skill {
            command: SkillCommand::Generate { .. },
        } => return None,
        Command::Skill { command } => cmd_skill::cmd_skill(db_path, config, command).await,
        Command::Backup(args) => cmd_backup::cmd_backup(args).await,
        Command::Restore(args) => cmd_backup::cmd_restore(args).await,
        Command::Webui { command } => cmd_webui::cmd_webui(command).await,
        Command::Doctor => cmd_doctor::cmd_doctor(assistant_dir, db_path).await,
        Command::Login { server } => cmd_login::cmd_login(server).await,
        Command::Logout => cmd_login::cmd_logout().await,
        Command::Status => cmd_login::cmd_status(),
        Command::ApiKeys { command } => match command {
            ApiKeysCommand::Create { name, scopes } => {
                cmd_login::cmd_api_keys_create(name, scopes, &cli.api_key, &cli.server).await
            }
            ApiKeysCommand::List => cmd_login::cmd_api_keys_list(&cli.api_key, &cli.server).await,
            ApiKeysCommand::Revoke { id } => {
                cmd_login::cmd_api_keys_revoke(id, &cli.api_key, &cli.server).await
            }
        },
        Command::Account { command } => match command {
            AccountCommand::Show { json } => {
                cmd_account::cmd_account_show(*json, &cli.api_key, &cli.server).await
            }
            AccountCommand::SetEmail { email } => {
                cmd_account::cmd_account_set_email(email, &cli.api_key, &cli.server).await
            }
            AccountCommand::SetName { name } => {
                cmd_account::cmd_account_set_name(name, &cli.api_key, &cli.server).await
            }
            AccountCommand::ChangePassword => {
                cmd_account::cmd_account_change_password(&cli.api_key, &cli.server).await
            }
        },
        Command::Admin { command } => match command {
            AdminCommand::ResetPassword { email, org } => {
                cmd_admin::cmd_admin_reset_password(email, org).await
            }
        },
        _ => return None,
    };
    Some(result)
}
