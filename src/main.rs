use crate::ast::{Step, Task};
use crate::cli_args::{CliArgs, parse_cli_args};
use crate::parser::parse_tasks_file;
use crate::project_root::ProjectRoot;
use std::env;
use std::process::Command;

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
            let tasks_file_raw = find_and_read_tasks_file()?;

            let tasks = parse_tasks_file(&tasks_file_raw)?;

            let task = tasks.get(&task_name)?;

            run_task(task)
        }
        CliArgs::ShowHelp => print_help_text(),
        CliArgs::ListCommands => list_tasks(),
    }
}

fn run_task(Task { name, steps }: &Task) -> anyhow::Result<()> {
    println!("{name}:");

    for step in steps {
        if let Step::Command(s) = step {
            let result = Command::new("bash").arg("-c").arg(s.as_ref()).output()?;

            print!("{}", str::from_utf8(&result.stdout).unwrap());
            eprint!("{}", str::from_utf8(&result.stderr).unwrap());
        }
    }

    Ok(())
}

fn find_and_read_tasks_file() -> anyhow::Result<String> {
    let cwd = env::current_dir()?;
    let project_root = ProjectRoot::find_project_root_starting_from(&cwd)?;

    dbg!(&project_root);

    project_root.read_tasks_file()
}

fn list_tasks() -> anyhow::Result<()> {
    let tasks_file_raw = find_and_read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    println!("Available tasks:");

    for task_name in tasks.list_tasks() {
        println!("- {task_name}");
    }

    Ok(())
}

fn print_help_text() -> anyhow::Result<()> {
    unimplemented!()
}
