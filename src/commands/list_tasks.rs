use crate::ast::TaskFile;
use crate::error::{Context, JuteResult, WRITING_TO_STDOUT};
use crate::parser::parse_tasks_file;
use crate::project_root::find_and_read_tasks_file;
use std::io::Write;

pub fn list_tasks(out: &mut impl Write) -> JuteResult<()> {
    let tasks_file_raw = find_and_read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    write_task_list(out, &tasks)
}

fn write_task_list(out: &mut impl Write, tasks: &TaskFile) -> JuteResult<()> {
    writeln!(out, "Available tasks:").context(WRITING_TO_STDOUT)?;

    for task_name in tasks.list_tasks() {
        writeln!(out, "- {task_name}").context(WRITING_TO_STDOUT)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::BrokenPipeWriter;

    const TASKS_FILE: &str = "\
build:
  cargo build

test:
  cargo test
";

    #[test]
    fn every_task_in_the_file_is_listed() {
        let tasks = parse_tasks_file(TASKS_FILE).expect("source should parse");
        let mut out = Vec::new();

        write_task_list(&mut out, &tasks).unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "Available tasks:\n- build\n- test\n"
        );
    }

    /// Writing with `println!` panics when the write fails, which aborts jute
    /// outright once the reader of a pipe has exited.
    #[test]
    fn a_closed_stdout_is_reported_as_an_error_rather_than_panicking() {
        let tasks = parse_tasks_file(TASKS_FILE).expect("source should parse");

        let error = write_task_list(&mut BrokenPipeWriter, &tasks).unwrap_err();

        assert_eq!(error.to_string(), "failed to write to stdout: broken pipe");
        assert!(error.is_broken_pipe());
    }
}
