use std::borrow::Cow;

#[derive(Debug, PartialEq)]
pub enum CliArgs {
    RunTask {
        task_name: Cow<'static, str>,
        args: Vec<String>,
    },
    ShowHelp,
}

pub fn parse_cli_args<Args: IntoIterator<Item = String>>(cli_args: Args) -> CliArgs {
    let mut args = cli_args.into_iter();

    let _program_name = args.next();

    match args.next() {
        Some(s) if s == "--help" => CliArgs::ShowHelp,
        Some(command) => CliArgs::RunTask {
            task_name: command.into(),
            args: args.collect(),
        },
        None => CliArgs::RunTask {
            task_name: Cow::Borrowed("default"),
            args: Vec::new(),
        },
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
                task_name: Cow::Borrowed("build"),
                args: vec!["--release".to_string(), "-v".to_string()],
            }
        );
    }

    #[test]
    fn help_flag_shows_help() {
        assert_eq!(parse_cli_args(args(&["jute", "--help"])), CliArgs::ShowHelp);
    }
}
