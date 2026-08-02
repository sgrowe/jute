use crate::ast::Task;
use crate::cli_args::{CliArgs, parse_cli_args};
use crate::parser::Parser;
use crate::project_root::ProjectRoot;
use crate::tokeniser::Tokeniser;
use std::env;

// `main` doesn't read or parse the task file yet, so these three modules are
// only reachable from their own tests. Drop the attributes once it does.
mod ast;
mod cli_args;
mod parser;
mod project_root;
mod tokeniser;

fn main() -> anyhow::Result<()> {
    let args = parse_cli_args(env::args());

    dbg!(&args);

    match args {
        CliArgs::RunTask {
            task_name,
            args: _args,
        } => {
            let cwd = env::current_dir()?;
            let project_root = ProjectRoot::find_project_root_starting_from(&cwd)?;

            dbg!(&project_root);

            let tasks_file_raw = project_root.read_tasks_file()?;

            let tasks = Parser::new(Tokeniser::new(&tasks_file_raw)).parse()?;

            let task = tasks.get(&task_name)?;

            run_task(task)
        }
        CliArgs::ShowHelp => print_help_text(),
    }
}

fn run_task(task: &Task) -> anyhow::Result<()> {
    unimplemented!()
}

fn print_help_text() -> anyhow::Result<()> {
    unimplemented!()
}
