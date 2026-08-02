use anyhow::anyhow;

use crate::ast::{Step, Task};
use crate::parser::parse_tasks_file;
use crate::project_root::find_and_read_tasks_file;
use std::process::Command;

pub fn run_task(task_name: &str, _args: &[String]) -> anyhow::Result<()> {
    let tasks_file_raw = find_and_read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    let Task { name, steps } = tasks.get(task_name)?;

    println!("{name}:");

    run_steps(steps)
}

fn run_steps(steps: &[Step]) -> anyhow::Result<()> {
    for step in steps {
        if let Step::Command(s) = step {
            let status = Command::new("bash").arg("-c").arg(s.as_ref()).status()?;

            if !status.success() {
                return Err(match status.code() {
                    Some(code) => anyhow!("Command \"{}\" failed with status code {}", s, code),
                    None => anyhow!("Command \"{}\" was terminated by a signal", s),
                });
            }
        }
    }

    Ok(())
}
