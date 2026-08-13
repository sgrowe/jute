use crate::ast::{EnvVar, Step, Task};
use crate::command_runner::CommandRunner;
use crate::error::{Context, JuteError, JuteResult};
use crate::parser::parse_tasks_file;
use crate::project_root::ProjectRoot;
use std::borrow::Cow;
use std::collections::BTreeMap;
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

    let env = BTreeMap::new();

    run_steps(steps, project_root.path(), &env, runner)
}

fn run_steps<R: CommandRunner>(
    steps: &[Step],
    cwd: &Path,
    env: &BTreeMap<&str, &str>,
    runner: &mut R,
) -> JuteResult<()> {
    for step in steps {
        match step {
            Step::Command(program, args) => {
                let mut cmd = Command::new(program.as_ref());
                cmd.args(args.iter().map(AsRef::as_ref));

                cmd.current_dir(cwd);
                cmd.envs(env);

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
            Step::InSubDir { path, steps } => run_steps(steps, &cwd.join(path), env, runner)?,
            Step::With {
                env: env_vars,
                steps,
            } => {
                let mut new_env = env.clone();

                new_env.extend(env_vars.iter().map(EnvVar::to_tuple));

                run_steps(steps, cwd, &new_env, runner)?
            }
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
    use std::ffi::OsString;
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
            "\
greet:
  echo hello
  echo goodbye

farewell:
  echo unused
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].command, vec!["echo", "hello"]);
        assert_eq!(runner.commands[1].command, vec!["echo", "goodbye"],);
    }

    /// A quoted argument reaches the command as one argument, so the space
    /// inside it doesn't become an argument separator.
    #[test]
    fn a_quoted_argument_is_passed_to_the_command_as_one_argument() {
        let root = write_project(
            "jute-a-quoted-argument-is-passed-as-one-argument",
            "\
greet:
  echo 'hello world'
  echo \"hello world\"
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].command, vec!["echo", "hello world"]);
        assert_eq!(runner.commands[1].command, vec!["echo", "hello world"]);
    }

    #[test]
    fn with_step_sets_env_vars_for_commands_inside_it() {
        let root = write_project(
            "jute-with-step-sets-env-vars-for-commands-inside-it",
            "\
greet:
  echo before
  with GREETING=hi:
    echo inside
  echo after
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].envs, vec![]);
        assert_eq!(
            runner.commands[1].envs,
            vec![(OsString::from("GREETING"), OsString::from("hi"))]
        );
        assert_eq!(runner.commands[2].envs, vec![]);
    }

    #[test]
    fn nested_with_blocks_merge_and_inner_shadows_outer() {
        let root = write_project(
            "jute-nested-with-blocks-merge-and-inner-shadows-outer",
            "\
greet:
  with A=1 B=2:
    with A=3:
      echo hello
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(
            runner.commands[0].envs,
            vec![
                (OsString::from("A"), OsString::from("3")),
                (OsString::from("B"), OsString::from("2")),
            ]
        );
    }

    #[test]
    fn in_step_changes_cwd_for_commands_inside_it() {
        let root = write_project(
            "jute-in-step-changes-cwd-for-commands-inside-it",
            "\
greet:
  echo before
  in sub:
    echo inside
  echo after
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].cwd, Some(root.clone()));
        assert_eq!(runner.commands[1].cwd, Some(root.join("sub")));
        assert_eq!(runner.commands[2].cwd, Some(root.clone()));
    }

    #[test]
    fn nested_in_blocks_join_directories() {
        let root = write_project(
            "jute-nested-in-blocks-join-directories",
            "\
greet:
  in a:
    in b:
      echo hello
",
        );

        let mut runner = RecordingRunner::default();
        let result = run_task("greet", &[], &root, &mut runner);

        result.unwrap();

        assert_eq!(runner.commands[0].cwd, Some(root.join("a").join("b")));
    }
}
