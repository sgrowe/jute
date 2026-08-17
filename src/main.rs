use crate::cli_args::{CliArgs, parse_cli_args};
use crate::command_runner::BashCommandRunner;
use crate::commands::{install, list_tasks, run_task, show_help};
use crate::error::{Context, JuteResult, WRITING_TO_STDOUT};
use std::env;
use std::io::{self, Write};
use std::process;

mod ast;
mod cli_args;
mod command_runner;
mod commands;
mod error;
mod parser;
mod project_root;
#[cfg(test)]
mod test_utils;
mod tokeniser;

fn main() {
    if let Err(e) = run() {
        // A broken pipe means whatever was reading jute's output has gone, so
        // there is nobody left to report to — but the run still failed, and
        // the exit code still says so. `eprintln!` is avoided because it
        // panics if stderr is closed too.
        if !e.is_broken_pipe() {
            let _ = writeln!(io::stderr(), "Error: {e}");
        }

        process::exit(e.exit_code());
    }
}

fn run() -> JuteResult<()> {
    let args = parse_cli_args(env::args_os());

    let mut out = io::stdout().lock();

    let result = match args {
        // The namespace is parsed but not yet used: tasks still all come from
        // `.jute/tasks.jute`.
        CliArgs::RunTask {
            namespace: _,
            task_name,
            args,
        } => {
            let mut cmd_runner = BashCommandRunner {};
            let cwd = current_dir()?;

            run_task(&task_name, &args, &cwd, &mut cmd_runner, &mut out)
        }
        CliArgs::ShowHelp => show_help(&mut out),
        CliArgs::ListCommands => list_tasks(&mut out),
        CliArgs::Install => {
            let mut cmd_runner = BashCommandRunner {};
            let cwd = current_dir()?;

            install(&cwd, &mut cmd_runner, &mut out)
        }
    };

    result?;

    out.flush().context(WRITING_TO_STDOUT)
}

fn current_dir() -> JuteResult<std::path::PathBuf> {
    env::current_dir().context("failed to determine the current directory")
}
