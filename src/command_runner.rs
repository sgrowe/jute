use std::{
    io,
    process::{Command, ExitStatus, Output},
};

pub trait CommandRunner {
    /// Runs the command with inherited stdio, streaming its output live.
    fn exec(&mut self, command: Command) -> io::Result<ExitStatus>;

    /// Runs the command capturing stdout and stderr instead of streaming them.
    fn output(&mut self, command: Command) -> io::Result<Output>;
}

pub struct BashCommandRunner {}

impl CommandRunner for BashCommandRunner {
    fn exec(&mut self, mut command: Command) -> io::Result<ExitStatus> {
        command.status()
    }

    fn output(&mut self, mut command: Command) -> io::Result<Output> {
        command.output()
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;

    /// Records the commands it is handed rather than running them. `output`
    /// calls pop their result from `queued_outputs`; an empty queue (and every
    /// `exec` call) reports success.

    #[derive(Debug, Default)]
    pub struct RecordingRunner {
        pub commands: Vec<CommandSnapshot>,
        pub queued_outputs: VecDeque<Output>,
    }

    impl CommandRunner for RecordingRunner {
        fn exec(&mut self, command: Command) -> io::Result<ExitStatus> {
            self.commands.push(command.into());

            Ok(ExitStatus::from_raw(0))
        }

        fn output(&mut self, command: Command) -> io::Result<Output> {
            let snapshot = CommandSnapshot::from(command);

            // Mirror curl's observable contract: it writes its `--output`
            // file even when the transfer later fails partway.
            if let Some(pos) = snapshot.command.iter().position(|arg| arg == "--output")
                && let Some(path) = snapshot.command.get(pos + 1)
            {
                std::fs::write(path, b"")?;
            }

            self.commands.push(snapshot);

            Ok(self.queued_outputs.pop_front().unwrap_or(Output {
                status: ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }))
        }
    }

    /// A unix wait status keeps the exit code in its high byte.
    pub fn output_with_exit_code(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// A wait status holding just a signal number, so `.code()` is `None`.
    pub fn output_killed_by_signal(signal: i32) -> Output {
        Output {
            status: ExitStatus::from_raw(signal),
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    #[derive(Debug)]
    pub struct CommandSnapshot {
        pub command: Vec<OsString>,
        pub cwd: Option<PathBuf>,
        pub envs: Vec<(OsString, OsString)>,
    }

    impl From<Command> for CommandSnapshot {
        fn from(cmd: Command) -> Self {
            Self {
                command: std::iter::once(cmd.get_program())
                    .chain(cmd.get_args())
                    .map(ToOwned::to_owned)
                    .collect(),
                cwd: cmd.get_current_dir().map(ToOwned::to_owned),
                envs: cmd
                    .get_envs()
                    .filter_map(|(k, v)| Some((k.to_owned(), v?.to_owned())))
                    .collect(),
            }
        }
    }
}
