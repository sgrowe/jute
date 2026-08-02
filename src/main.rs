use crate::cli_args::{CliArgs, parse_cli_args};
use crate::commands::{list_tasks, run_task, show_help};
use std::env;

mod ast;
mod cli_args;
mod commands;
mod parser;
mod project_root;
mod tokeniser;

fn main() -> anyhow::Result<()> {
    let args = parse_cli_args(env::args());

    match args {
        CliArgs::RunTask { task_name, args } => run_task(&task_name, &args),
        CliArgs::ShowHelp => show_help(),
        CliArgs::ListCommands => list_tasks(),
    }
}
