use std::{borrow::Cow, fmt, io, path::PathBuf};

use crate::parser::ParseError;

/// The result type every fallible operation in jute returns.
pub type JuteResult<T> = Result<T, JuteError>;

#[derive(Debug)]
pub enum JuteError {
    /// No `.jute` directory in the current directory or any of its ancestors.
    NoProjectRoot {
        cwd: PathBuf,
    },

    /// The task named on the command line isn't in the tasks file.
    TaskNotFound {
        name: String,
    },

    /// A step exited non-zero. `code` becomes jute's own exit code, so that a
    /// failing `jute test` is indistinguishable from running the test command
    /// directly.
    CommandFailed {
        command: String,
        code: i32,
    },

    /// A step was killed by a signal, so there's no exit code to propagate.
    CommandKilledBySignal {
        command: String,
    },

    /// A platform binary download failed. curl's captured stdout and stderr
    /// are replayed to the user, since they were not streamed live.
    DownloadFailed {
        url: String,
        /// `None` if curl was killed by a signal.
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },

    Parse(ParseError),

    /// An io failure, labelled with what jute was trying to do at the time —
    /// the underlying error alone is rarely enough to act on, since
    /// "No such file or directory" doesn't say which file.
    Io {
        doing: Cow<'static, str>,
        source: io::Error,
    },
}

impl JuteError {
    /// The status jute exits with. A step's own exit code passes straight
    /// through; everything else is a failure of jute itself.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::CommandFailed { code, .. } => *code,
            _ => 1,
        }
    }
}

impl fmt::Display for JuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProjectRoot { cwd } => {
                write!(
                    f,
                    "no .jute folder found in {} or any parent",
                    cwd.display()
                )
            }
            Self::TaskNotFound { name } => write!(f, "Task \"{name}\" does not exist"),
            Self::CommandFailed { command, code } => {
                write!(f, "Command \"{command}\" failed with status code {code}")
            }
            Self::CommandKilledBySignal { command } => {
                write!(f, "Command \"{command}\" was terminated by a signal")
            }
            Self::DownloadFailed {
                url,
                code,
                stdout,
                stderr,
            } => {
                writeln!(f, "failed to download {url}")?;

                for (label, output) in [("stdout", stdout), ("stderr", stderr)] {
                    writeln!(f, "--- curl {label} ---")?;
                    if !output.is_empty() {
                        write!(f, "{output}")?;
                        if !output.ends_with('\n') {
                            writeln!(f)?;
                        }
                    }
                }

                match code {
                    Some(code) => write!(f, "curl failed with exit code {code}"),
                    None => write!(f, "curl was terminated by a signal"),
                }
            }
            Self::Parse(e) => e.fmt(f),
            Self::Io { doing, source } => write!(f, "{doing}: {source}"),
        }
    }
}

impl std::error::Error for JuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<ParseError> for JuteError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

/// Labels an io failure with what jute was attempting at the time.
///
/// There is deliberately no `From<io::Error> for JuteError`, so every io error
/// has to pass through here and none can reach the user unlabelled.
pub trait Context<T> {
    fn context<S>(self, doing: S) -> JuteResult<T>
    where
        S: Into<Cow<'static, str>>;

    /// As [`Context::context`], but only builds the label when there is an
    /// error to label — for the common `format!` case.
    fn with_context<S, F>(self, doing: F) -> JuteResult<T>
    where
        S: Into<Cow<'static, str>>,
        F: FnOnce() -> S;
}

impl<T> Context<T> for Result<T, io::Error> {
    fn context<S>(self, doing: S) -> JuteResult<T>
    where
        S: Into<Cow<'static, str>>,
    {
        self.map_err(|source| JuteError::Io {
            doing: doing.into(),
            source,
        })
    }

    fn with_context<S, F>(self, doing: F) -> JuteResult<T>
    where
        S: Into<Cow<'static, str>>,
        F: FnOnce() -> S,
    {
        self.map_err(|source| JuteError::Io {
            doing: doing().into(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_command_exits_with_that_commands_own_status() {
        let error = JuteError::CommandFailed {
            command: "cargo test".to_string(),
            code: 101,
        };

        assert_eq!(error.exit_code(), 101);
        assert_eq!(
            error.to_string(),
            "Command \"cargo test\" failed with status code 101"
        );
    }

    #[test]
    fn every_other_failure_exits_with_one() {
        let error = JuteError::NoProjectRoot {
            cwd: PathBuf::from("/tmp/nowhere"),
        };

        assert_eq!(error.exit_code(), 1);
        assert_eq!(
            error.to_string(),
            "no .jute folder found in /tmp/nowhere or any parent"
        );
    }

    #[test]
    fn a_failed_download_replays_curls_output_and_exits_with_one() {
        let error = JuteError::DownloadFailed {
            url: "https://example.com/jute-linux-x86_64".to_string(),
            code: Some(22),
            stdout: "".to_string(),
            stderr: "curl: (22) The requested URL returned error: 404\n".to_string(),
        };

        assert_eq!(error.exit_code(), 1);
        assert_eq!(
            error.to_string(),
            "failed to download https://example.com/jute-linux-x86_64\n\
             --- curl stdout ---\n\
             --- curl stderr ---\n\
             curl: (22) The requested URL returned error: 404\n\
             curl failed with exit code 22"
        );
    }

    #[test]
    fn curl_output_without_trailing_newlines_still_renders_separated_sections() {
        let error = JuteError::DownloadFailed {
            url: "https://example.com/jute-linux-x86_64".to_string(),
            code: Some(56),
            stdout: "partial body".to_string(),
            stderr: "connection reset".to_string(),
        };

        assert_eq!(
            error.to_string(),
            "failed to download https://example.com/jute-linux-x86_64\n\
             --- curl stdout ---\n\
             partial body\n\
             --- curl stderr ---\n\
             connection reset\n\
             curl failed with exit code 56"
        );
    }

    #[test]
    fn a_signal_killed_download_reports_the_signal_instead_of_an_exit_code() {
        let error = JuteError::DownloadFailed {
            url: "https://example.com/jute-linux-x86_64".to_string(),
            code: None,
            stdout: "".to_string(),
            stderr: "".to_string(),
        };

        assert_eq!(error.exit_code(), 1);
        assert_eq!(
            error.to_string(),
            "failed to download https://example.com/jute-linux-x86_64\n\
             --- curl stdout ---\n\
             --- curl stderr ---\n\
             curl was terminated by a signal"
        );
    }

    #[test]
    fn io_errors_are_labelled_with_what_jute_was_doing() {
        let failed: Result<(), io::Error> =
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));

        let error = failed.context("failed to write /x/.jute/run").unwrap_err();

        assert_eq!(error.to_string(), "failed to write /x/.jute/run: denied");
        assert_eq!(error.exit_code(), 1);
    }

    #[test]
    fn with_context_labels_lazily() {
        let succeeded: Result<u8, io::Error> = Ok(1);

        let result = succeeded.with_context(|| -> &'static str {
            panic!("must not be called when there is no error")
        });

        assert_eq!(result.unwrap(), 1);
    }
}
