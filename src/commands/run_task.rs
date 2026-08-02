use anyhow::anyhow;

use crate::ast::{Step, Task};
use crate::command_runner::CommandRunner;
use crate::parser::parse_tasks_file;
use crate::project_root::ProjectRoot;
use std::env;
use std::path::Path;
use std::process::Command;

pub fn run_task<R: CommandRunner>(
    task_name: &str,
    _args: &[String],
    runner: &mut R,
) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let project_root = ProjectRoot::find_project_root_starting_from(&cwd)?;

    let tasks_file_raw = project_root.read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    let Task { name, steps } = tasks.get(task_name)?;

    println!("{name}:");

    run_steps(steps, project_root.path(), runner)
}

fn run_steps<R: CommandRunner>(steps: &[Step], root: &Path, runner: &mut R) -> anyhow::Result<()> {
    for step in steps {
        match step {
            Step::Command(s) => {
                let mut cmd = Command::new("bash");
                cmd.args(["-c", s]);
                let status = runner.exec(cmd)?;

                if !status.success() {
                    return Err(match status.code() {
                        Some(code) => anyhow!("Command \"{}\" failed with status code {}", s, code),
                        None => anyhow!("Command \"{}\" was terminated by a signal", s),
                    });
                }
            }
            Step::InSubDir { path, steps } => run_steps(steps, &root.join(path), runner)?,
            Step::With { env: _, steps } => run_steps(steps, root, runner)?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::command_runner::test_utils::RecordingRunner;

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Writes a throwaway project holding `tasks_file` and returns its root.
    fn write_project(name: &str, tasks_file: &str) -> PathBuf {
        let root = env::temp_dir().join(name);
        let jute_dir = root.join(".jute");

        fs::create_dir_all(&jute_dir).unwrap();
        fs::write(jute_dir.join("tasks.jute"), tasks_file).unwrap();

        root
    }

    #[test]
    fn runs_every_step_of_the_named_task_through_bash() {
        let root = write_project(
            "jute-runs-every-step-of-the-named-task",
            "greet:\n  echo hello\n  echo goodbye\n\nfarewell:\n  echo unused\n",
        );

        // `run_task` finds the project from the process' working directory, so
        // the test has to move there and put it back afterwards.
        let previous_cwd = env::current_dir().unwrap();
        env::set_current_dir(&root).unwrap();

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &mut runner);

        env::set_current_dir(previous_cwd).unwrap();

        result.unwrap();

        assert_eq!(runner.commands[0].command, vec!["bash", "-c", "echo hello"]);
        assert_eq!(
            runner.commands[1].command,
            vec!["bash", "-c", "echo goodbye"],
        );
    }
}
