use std::{
    io,
    process::{Command, ExitStatus},
};

pub trait CommandRunner {
    fn exec(&mut self, command: Command) -> io::Result<ExitStatus>;
}

pub struct BashCommandRunner {}

impl CommandRunner for BashCommandRunner {
    fn exec(&mut self, mut command: Command) -> io::Result<ExitStatus> {
        command.status()
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    use std::ffi::OsString;
    use std::os::unix::process::ExitStatusExt;

    /// Records the commands it is handed rather than running them, and reports
    /// every one of them as having succeeded.

    #[derive(Debug, Default)]
    pub struct RecordingRunner {
        pub commands: Vec<CommandSnapshot>,
    }

    impl CommandRunner for RecordingRunner {
        fn exec(&mut self, command: Command) -> io::Result<ExitStatus> {
            self.commands.push(command.into());

            Ok(ExitStatus::from_raw(0))
        }
    }

    #[derive(Debug)]
    pub struct CommandSnapshot {
        pub command: Vec<OsString>,
    }

    impl From<Command> for CommandSnapshot {
        fn from(cmd: Command) -> Self {
            Self {
                command: std::iter::once(cmd.get_program())
                    .chain(cmd.get_args())
                    .map(ToOwned::to_owned)
                    .collect(),
            }
        }
    }
}
