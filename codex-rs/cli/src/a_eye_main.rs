use clap::Parser;
use codex_arg0::arg0_dispatch_or_else;
use codex_cli::aeye;
use codex_cli::multitool;
use std::ffi::OsString;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        let args: Vec<OsString> = std::env::args_os().collect();
        if should_run_workflow_cli(&args) {
            let cli = aeye::AEyeCli::parse();
            aeye::run_main(cli, codex_linux_sandbox_exe).await?;
        } else {
            multitool::run_with_sandbox_exe(codex_linux_sandbox_exe).await?;
        }
        Ok(())
    })
}

fn should_run_workflow_cli(args: &[OsString]) -> bool {
    let Some(command) = first_command_token(args) else {
        return false;
    };

    match command.as_str() {
        "scan" | "plan" | "explain" | "patch" | "verify" | "run" | "status" | "learn" => true,
        "apply" => is_workflow_apply_command(args),
        _ => false,
    }
}

fn first_command_token(args: &[OsString]) -> Option<String> {
    let mut idx = 1usize;
    while idx < args.len() {
        let token = args[idx].to_string_lossy();
        match token.as_ref() {
            "-c" | "--config" => {
                idx += 2;
            }
            _ if token.starts_with("--config=") => {
                idx += 1;
            }
            _ if token.starts_with('-') => {
                idx += 1;
            }
            _ => return Some(token.into_owned()),
        }
    }
    None
}

fn is_workflow_apply_command(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        let token = arg.to_string_lossy();
        token == "--from" || token.starts_with("--from=")
    })
}
