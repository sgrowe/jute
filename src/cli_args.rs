use std::borrow::Cow;
use std::ffi::OsString;

#[derive(Debug, PartialEq)]
pub enum CliArgs {
    RunTask {
        /// The `<namespace>` of a `<namespace>.<task>` command, naming the
        /// `.jute/<namespace>.jute` file the task lives in. `None` for the
        /// default namespace.
        namespace: Option<Cow<'static, str>>,
        task_name: Cow<'static, str>,
        /// Kept as given, since these are passed on to the commands the task
        /// runs and need not be valid unicode.
        args: Vec<OsString>,
    },
    ShowHelp,
    ListCommands,
    Install,
}

/// Takes `OsString`s because `env::args()` panics on an argument that isn't
/// valid unicode.
pub fn parse_cli_args<Args: IntoIterator<Item = OsString>>(cli_args: Args) -> CliArgs {
    let mut args = cli_args.into_iter();

    let _program_name = args.next();

    let Some(command) = args.next() else {
        return CliArgs::RunTask {
            namespace: None,
            task_name: Cow::Borrowed("default"),
            args: Vec::new(),
        };
    };

    // A tasks file is read as unicode, so a command that isn't valid unicode
    // matches no task either way; replacing the bad bytes just leaves it
    // printable in the "no such task" error.
    let command = command.to_string_lossy().into_owned();

    match command.as_str() {
        "help" | "--help" => CliArgs::ShowHelp,
        "self.list" => CliArgs::ListCommands,
        "self.install" => CliArgs::Install,
        _ => run_task(&command, args.collect()),
    }
}

/// Splits `command` at the first `.`, so that everything after it is the task
/// name. A command with no `.` names a task in the default namespace.
fn run_task(command: &str, args: Vec<OsString>) -> CliArgs {
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
    use std::os::unix::ffi::OsStringExt;

    fn args(strs: &[&str]) -> Vec<OsString> {
        strs.iter().map(OsString::from).collect()
    }

    /// `env::args()` panics on an argument that isn't valid unicode, so jute
    /// reads its arguments as `OsString`s. Such a command names no task, and
    /// the replacement characters only have to make the error legible.
    #[test]
    fn a_command_that_is_not_valid_utf8_is_reported_rather_than_panicking() {
        let command = OsString::from_vec(vec![b'b', 0xff, b'd']);

        assert_eq!(
            parse_cli_args(vec![OsString::from("jute"), command]),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Owned("b\u{fffd}d".to_string()),
                args: Vec::new(),
            }
        );
    }

    /// A task's arguments are handed to the commands it runs, so they keep
    /// the exact bytes they were given.
    #[test]
    fn arguments_that_are_not_valid_utf8_are_kept_verbatim() {
        let raw = OsString::from_vec(vec![0xff, 0xfe]);

        assert_eq!(
            parse_cli_args(vec![
                OsString::from("jute"),
                OsString::from("build"),
                raw.clone(),
            ]),
            CliArgs::RunTask {
                namespace: None,
                task_name: Cow::Borrowed("build"),
                args: vec![raw],
            }
        );
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
            parse_cli_args(Vec::<OsString>::new()),
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
                args: args(&["--release", "-v"]),
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
                args: args(&["--release"]),
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
