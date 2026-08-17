use std::borrow::Cow;

#[derive(Debug, PartialEq)]
pub enum CliArgs {
    RunTask {
        /// The `<namespace>` of a `<namespace>.<task>` command, naming the
        /// `.jute/<namespace>.jute` file the task lives in. `None` for the
        /// default namespace.
        namespace: Option<Cow<'static, str>>,
        task_name: Cow<'static, str>,
        args: Vec<String>,
    },
    ShowHelp,
    ListCommands,
    Install,
}

pub fn parse_cli_args<Args: IntoIterator<Item = String>>(cli_args: Args) -> CliArgs {
    let mut args = cli_args.into_iter();

    let _program_name = args.next();

    let Some(command) = args.next() else {
        return CliArgs::RunTask {
            namespace: None,
            task_name: Cow::Borrowed("default"),
            args: Vec::new(),
        };
    };

    match command.as_str() {
        "help" | "--help" => CliArgs::ShowHelp,
        "self.list" => CliArgs::ListCommands,
        "self.install" => CliArgs::Install,
        _ => run_task(&command, args.collect()),
    }
}

/// Splits `command` at the first `.`, so that everything after it is the task
/// name. A command with no `.` names a task in the default namespace.
fn run_task(command: &str, args: Vec<String>) -> CliArgs {
    let (namespace, task_name) = match command.split_once('.') {
        Some((namespace, task_name)) => (Some(namespace.to_string().into()), task_name.to_string()),
        None => (None, command.to_string()),
    };

    CliArgs::RunTask {
        namespace,
        task_name: task_name.into(),
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_args_runs_default_command() {
        assert_eq!(
            parse_cli_args(args(&["jute"])),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Borrowed("default"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn empty_iterator_runs_default_command() {
        assert_eq!(
            parse_cli_args(Vec::<String>::new()),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Borrowed("default"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_with_no_args() {
        assert_eq!(
            parse_cli_args(args(&["jute", "build"])),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Borrowed("build"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_with_args() {
        assert_eq!(
            parse_cli_args(args(&["jute", "build", "--release", "-v"])),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Borrowed("build"),
                args: vec!["--release".to_string(), "-v".to_string()],
            }
        );
    }

    #[test]
    fn namespaced_command() {
        assert_eq!(
            parse_cli_args(args(&["jute", "backend.build"])),
            CliArgs::RunTask {
                namespace: Some(Cow::Borrowed("backend")),
                task_name: Cow::Borrowed("build"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn namespaced_command_with_args() {
        assert_eq!(
            parse_cli_args(args(&["jute", "backend.build", "--release"])),
            CliArgs::RunTask {
                namespace: Some(Cow::Borrowed("backend")),
                task_name: Cow::Borrowed("build"),
                args: vec!["--release".to_string()],
            }
        );
    }

    #[test]
    fn a_command_is_split_at_the_first_dot_only() {
        assert_eq!(
            parse_cli_args(args(&["jute", "a.b.c"])),
            CliArgs::RunTask {
                namespace: Some(Cow::Borrowed("a")),
                task_name: Cow::Borrowed("b.c"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn an_unknown_self_command_is_treated_as_a_task() {
        assert_eq!(
            parse_cli_args(args(&["jute", "self.foo"])),
            CliArgs::RunTask {
                namespace: Some(Cow::Borrowed("self")),
                task_name: Cow::Borrowed("foo"),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn help_command_shows_help() {
        assert_eq!(parse_cli_args(args(&["jute", "help"])), CliArgs::ShowHelp);
    }

    #[test]
    fn help_flag_is_an_alias_for_the_help_command() {
        assert_eq!(parse_cli_args(args(&["jute", "--help"])), CliArgs::ShowHelp);
    }

    #[test]
    fn self_list_lists_tasks() {
        assert_eq!(
            parse_cli_args(args(&["jute", "self.list"])),
            CliArgs::ListCommands
        );
    }

    #[test]
    fn self_install_runs_install() {
        assert_eq!(
            parse_cli_args(args(&["jute", "self.install"])),
            CliArgs::Install
        );
    }
}
