use crate::cli_args::{CliArgs, parse_cli_args};
use crate::command_runner::BashCommandRunner;
use crate::commands::{install, list_tasks, run_task, show_help};
use std::env;

mod ast;
mod cli_args;
mod command_runner;
mod commands;
mod parser;
mod project_root;
mod tokeniser;

fn main() -> anyhow::Result<()> {
    let args = parse_cli_args(env::args());

    match args {
        CliArgs::RunTask { task_name, args } => {
            let mut cmd_runner = BashCommandRunner {};
            let cwd = env::current_dir()?;

            run_task(&task_name, &args, &cwd, &mut cmd_runner)
        }
        CliArgs::ShowHelp => show_help(),
        CliArgs::ListCommands => list_tasks(),
        CliArgs::Install => {
            let cwd = env::current_dir()?;

            install(&cwd)
        }
    }
}
