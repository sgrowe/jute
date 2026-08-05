use crate::cli_args::{CliArgs, parse_cli_args};
use crate::command_runner::BashCommandRunner;
use crate::commands::{install, list_tasks, run_task, show_help};
use crate::error::{Context, JuteResult};
use std::env;
use std::process;

mod ast;
mod cli_args;
mod command_runner;
mod commands;
mod error;
mod parser;
mod project_root;
mod tokeniser;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");

        process::exit(e.exit_code());
    }
}

fn run() -> JuteResult<()> {
    let args = parse_cli_args(env::args());

    match args {
        CliArgs::RunTask { task_name, args } => {
            let mut cmd_runner = BashCommandRunner {};
            let cwd = current_dir()?;

            run_task(&task_name, &args, &cwd, &mut cmd_runner)
        }
        CliArgs::ShowHelp => show_help(),
        CliArgs::ListCommands => list_tasks(),
        CliArgs::Install => {
            let cwd = current_dir()?;

            install(&cwd)
        }
    }
}

fn current_dir() -> JuteResult<std::path::PathBuf> {
    env::current_dir().context("failed to determine the current directory")
}
