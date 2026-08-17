use crate::error::{Context, JuteResult, WRITING_TO_STDOUT};
use std::io::Write;

const HELP: &str = "\
jute is a task runner. Tasks are defined in .jute/tasks.jute

Usage:
  jute                Run the `default` task
  jute <task>         Run the named task
  jute help           Show this help
  jute self.list      List the tasks in this project
  jute self.install   Install jute into .jute in the current directory
";

pub fn show_help(out: &mut impl Write) -> JuteResult<()> {
    out.write_all(HELP.as_bytes()).context(WRITING_TO_STDOUT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::BrokenPipeWriter;

    #[test]
    fn help_describes_how_to_run_a_task_and_every_built_in_command() {
        let mut out = Vec::new();

        show_help(&mut out).unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\
jute is a task runner. Tasks are defined in .jute/tasks.jute

Usage:
  jute                Run the `default` task
  jute <task>         Run the named task
  jute help           Show this help
  jute self.list      List the tasks in this project
  jute self.install   Install jute into .jute in the current directory
"
        );
    }

    #[test]
    fn a_closed_stdout_is_reported_as_an_error_rather_than_panicking() {
        let error = show_help(&mut BrokenPipeWriter).unwrap_err();

        assert_eq!(error.to_string(), "failed to write to stdout: broken pipe");
        assert!(error.is_broken_pipe());
    }
}
