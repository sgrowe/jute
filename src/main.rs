use crate::cli_args::{CliArgs, parse_cli_args};
use crate::command_runner::BashCommandRunner;
use crate::commands::{list_tasks, run_task, show_help};
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

    let mut cmd_runner = BashCommandRunner {};

    match args {
        CliArgs::RunTask { task_name, args } => run_task(&task_name, &args, &mut cmd_runner),
        CliArgs::ShowHelp => show_help(),
        CliArgs::ListCommands => list_tasks(),
    }
}
