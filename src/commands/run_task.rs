use crate::ast::{Step, Task};
use crate::command_runner::CommandRunner;
use crate::error::{Context, JuteError, JuteResult};
use crate::parser::parse_tasks_file;
use crate::project_root::ProjectRoot;
use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

pub fn run_task<R: CommandRunner>(
    task_name: &str,
    _args: &[String],
    cwd: &Path,
    runner: &mut R,
) -> JuteResult<()> {
    let project_root = ProjectRoot::find_project_root_starting_from(cwd)?;

    let tasks_file_raw = project_root.read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    let Task { name, steps } = tasks.get(task_name)?;

    println!("{name}:");

    run_steps(steps, project_root.path(), runner)
}

fn run_steps<R: CommandRunner>(steps: &[Step], root: &Path, runner: &mut R) -> JuteResult<()> {
    for step in steps {
        match step {
            Step::Command(program, args) => {
                let mut cmd = Command::new(program.as_ref());
                cmd.args(args.iter().map(AsRef::as_ref));

                let status = runner.exec(cmd).with_context(|| {
                    format!("failed to run \"{}\"", command_line(program, args))
                })?;

                if !status.success() {
                    let command = command_line(program, args);

                    return Err(match status.code() {
                        Some(code) => JuteError::CommandFailed { command, code },
                        None => JuteError::CommandKilledBySignal { command },
                    });
                }
            }
            Step::InSubDir { path, steps } => run_steps(steps, &root.join(path), runner)?,
            Step::With { env: _, steps } => run_steps(steps, root, runner)?,
        }
    }

    Ok(())
}

/// Renders a step back into the command line it was written as, for error
/// messages.
fn command_line(program: &str, args: &[Cow<'_, str>]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use crate::command_runner::test_utils::RecordingRunner;

    use super::*;
    use std::env;
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

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].command, vec!["echo", "hello"]);
        assert_eq!(runner.commands[1].command, vec!["echo", "goodbye"],);
    }
}
