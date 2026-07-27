use std::{borrow::Cow, fmt, iter::Peekable, path::PathBuf};

use crate::{
    ast::{EnvVar, Step, Task, TaskFile},
    tokeniser::{Token, Tokeniser},
};

#[derive(Debug)]
pub struct ParseError {
    message: Cow<'static, str>,
}

impl ParseError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self {
            message: msg.into(),
        }
    }

    /// The tokeniser owns indentation, so `Indent`/`Dedent` reaching a place
    /// the grammar doesn't allow means the two disagree — not a user error.
    fn indentation() -> Self {
        Self::new(
            "BUG! Parser got confused by indentation, please report the file that caused this bug",
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct Parser<'a> {
    tokens: Peekable<Tokeniser<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Tokeniser<'a>) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    // Recursive descent parser
    pub fn parse(&mut self) -> Result<TaskFile<'a>, ParseError> {
        let mut file = TaskFile::default();

        while let Some(token) = self.tokens.next() {
            match token {
                Token::NewLine => {}
                Token::Word(name) => file.tasks.push(self.task(name)?),
                Token::Indent | Token::Dedent => return Err(ParseError::indentation()),
                t => return Err(ParseError::new(format!("Expected a task name, got {t:?}"))),
            }
        }

        Ok(file)
    }

    fn task(&mut self, name: &'a str) -> Result<Task<'a>, ParseError> {
        self.expect_next_token_to_be(Token::Colon)?;
        self.expect_next_token_to_be(Token::NewLine)?;

        let steps = self.block(&format!("Task {name}"))?;

        Ok(Task::new(name, steps))
    }

    /// Parses the indented block of steps that follows a `task:`, `in dir:` or
    /// `with VAR=value:` header.
    ///
    /// `owner` names that header for the "needs at least one step" error. The
    /// closing `Dedent` is consumed here; the last block in a file ends at the
    /// end of the token stream instead.
    fn block(&mut self, owner: &str) -> Result<Vec<Step<'a>>, ParseError> {
        self.skip_new_lines();

        if self.tokens.next_if_eq(&Token::Indent).is_none() {
            return Err(ParseError::new(format!(
                "{owner} should have at least one step"
            )));
        }

        let mut steps = Vec::new();

        loop {
            self.skip_new_lines();

            match self.tokens.peek() {
                None => break,
                Some(Token::Dedent) => {
                    self.tokens.next();
                    break;
                }
                Some(_) => steps.push(self.step()?),
            }
        }

        if steps.is_empty() {
            return Err(ParseError::new(format!(
                "{owner} should have at least one step"
            )));
        }

        Ok(steps)
    }

    fn step(&mut self) -> Result<Step<'a>, ParseError> {
        let token = self.tokens.next();

        match token {
            Some(Token::With) => self.with_step(),
            Some(Token::In) => self.in_step(),
            Some(Token::Word(w)) => self.command(w),
            Some(Token::Indent) | Some(Token::Dedent) => Err(ParseError::indentation()),
            _ => Err(ParseError::new(format!(
                "Unexpected token when parsing a step: {token:?}"
            ))),
        }
    }

    /// Everything up to the end of the line, with the characters the tokeniser
    /// split on put back. A command that holds no `:` or `=` is a single
    /// `Word`, so the common case borrows straight from the source.
    fn command(&mut self, first_word: &'a str) -> Result<Step<'a>, ParseError> {
        let mut line = Cow::Borrowed(first_word);

        loop {
            let token = self.tokens.next();

            match token {
                Some(Token::NewLine) | None => return Ok(Step::Command(line)),
                // `with `/`in ` keep the space the tokeniser matched them on
                Some(Token::With) => line.to_mut().push_str("with "),
                Some(Token::In) => line.to_mut().push_str("in "),
                Some(Token::Word(w)) => line.to_mut().push_str(w),
                Some(Token::Colon) => line.to_mut().push(':'),
                Some(Token::Equals) => line.to_mut().push('='),
                Some(Token::Indent) | Some(Token::Dedent) => {
                    return Err(ParseError::indentation());
                }
            }
        }
    }

    /// `with NAME=value [NAME=value ...]:` followed by a block.
    fn with_step(&mut self) -> Result<Step<'a>, ParseError> {
        let mut env = Vec::new();
        let mut name = self.env_var_word("Expected an environment variable name after `with`")?;

        loop {
            self.expect_next_token_to_be(Token::Equals)?;

            let value = self.env_var_word(&format!("Expected a value for `{name}`"))?;

            // `A=1 B=2` tokenises as `A`, `=`, `1 B`, `=`, `2`, so another
            // `=` means that word held this value *and* the next name.
            if self.tokens.peek() != Some(&Token::Equals) {
                env.push(EnvVar::new(name, value));
                break;
            }

            let Some((value, next_name)) = value.rsplit_once(char::is_whitespace) else {
                return Err(ParseError::new(format!(
                    "Expected a space between environment variables, got `{name}={value}=`"
                )));
            };

            env.push(EnvVar::new(name, value.trim()));
            name = next_name.trim();
        }

        self.expect_next_token_to_be(Token::Colon)?;
        self.expect_next_token_to_be(Token::NewLine)?;

        let steps = self.block("A `with` block")?;

        Ok(Step::With { env, steps })
    }

    /// `in <path>:` followed by a block.
    fn in_step(&mut self) -> Result<Step<'a>, ParseError> {
        let path = match self.tokens.next() {
            Some(Token::Word(w)) => PathBuf::from(w.trim()),
            t => {
                return Err(ParseError::new(format!(
                    "Expected a directory after `in`, got {t:?}"
                )));
            }
        };

        self.expect_next_token_to_be(Token::Colon)?;
        self.expect_next_token_to_be(Token::NewLine)?;

        let steps = self.block(&format!("`in {}`", path.display()))?;

        Ok(Step::InSubDir { path, steps })
    }

    /// A `Word` used as an environment variable name or value. The tokeniser
    /// splits on `=` and `:` without trimming, so `A = 1` keeps its spaces.
    fn env_var_word(&mut self, expectation: &str) -> Result<&'a str, ParseError> {
        match self.tokens.next() {
            Some(Token::Word(w)) => Ok(w.trim()),
            t => Err(ParseError::new(format!("{expectation}, got {t:?}"))),
        }
    }

    fn skip_new_lines(&mut self) {
        while self.tokens.next_if_eq(&Token::NewLine).is_some() {}
    }

    fn expect_next_token_to_be(&mut self, expected: Token<'a>) -> Result<Token<'a>, ParseError> {
        match self.tokens.next() {
            Some(t) if t == expected => Ok(t),
            Some(t) => Err(ParseError::new(format!("Expected {expected:?}, got {t:?}"))),
            None => Err(ParseError::new(format!(
                "Expected {expected:?} but reached end of the file"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{EnvVar, Step};
    use std::{fs, path::Path, path::PathBuf};

    /// Reads a file from `examples/`, relative to the package root so the
    /// result doesn't depend on the working directory the test runs in.
    fn read_example_file(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(name);

        fs::read_to_string(path).unwrap()
    }

    /// Parses a source string, for cases too small to be worth an example file.
    fn parse(source: &str) -> Result<TaskFile<'_>, ParseError> {
        Parser::new(Tokeniser::new(source)).parse()
    }

    /// The single step of the single task in `source`.
    fn parse_one_step(source: &str) -> Step<'_> {
        let mut tasks = parse(source).expect("source should parse").tasks;

        assert_eq!(tasks.len(), 1, "expected exactly one task");

        let steps = tasks.remove(0).steps;

        assert_eq!(steps.len(), 1, "expected exactly one step");

        steps.into_iter().next().unwrap()
    }

    #[test]
    fn parses_example_001() {
        let source = read_example_file("001.jute");

        let file = Parser::new(Tokeniser::new(&source))
            .parse()
            .expect("001.jute should parse");

        let expected = TaskFile {
            tasks: vec![
                // simple:
                //   in app/server:
                //     cargo install
                Task::new(
                    "simple",
                    vec![Step::InSubDir {
                        path: PathBuf::from("app/server"),
                        steps: vec![Step::Command("cargo install".into())],
                    }],
                ),
                // with-dir:
                //   in app/server:
                //     pnpm run build
                Task::new(
                    "with-dir",
                    vec![Step::InSubDir {
                        path: PathBuf::from("app/server"),
                        steps: vec![Step::Command("pnpm run build".into())],
                    }],
                ),
                // clean:
                //   in packages/shared:
                //     pnpm run clean
                //
                //   in app/client:
                //     pnpm run clean
                Task::new(
                    "clean",
                    vec![
                        Step::InSubDir {
                            path: PathBuf::from("packages/shared"),
                            steps: vec![Step::Command("pnpm run clean".into())],
                        },
                        Step::InSubDir {
                            path: PathBuf::from("app/client"),
                            steps: vec![Step::Command("pnpm run clean".into())],
                        },
                    ],
                ),
                // test-create-db:
                //   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
                Task::new(
                    "test-create-db",
                    vec![Step::Command(
                        "createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'"
                            .into(),
                    )],
                ),
                // test-migrate:
                //   jute test-create-db
                //   with NODE_ENV=test:
                //     pnpm exec migrate
                Task::new(
                    "test-migrate",
                    vec![
                        Step::Command("jute test-create-db".into()),
                        Step::With {
                            env: vec![EnvVar::new("NODE_ENV", "test")],
                            steps: vec![Step::Command("pnpm exec migrate".into())],
                        },
                    ],
                ),
            ],
        };

        assert_eq!(file, expected);
    }

    #[test]
    fn parses_several_env_vars_in_one_with() {
        let step = parse_one_step(
            "\
build:
  with NODE_ENV=production DEBUG=0:
    pnpm run build
",
        );

        assert_eq!(
            step,
            Step::With {
                env: vec![
                    EnvVar::new("NODE_ENV", "production"),
                    EnvVar::new("DEBUG", "0"),
                ],
                steps: vec![Step::Command("pnpm run build".into())],
            }
        );
    }

    #[test]
    fn parses_a_with_block_nested_in_an_in_block() {
        let step = parse_one_step(
            "\
migrate:
  in app/server:
    with NODE_ENV=test:
      pnpm exec migrate
",
        );

        assert_eq!(
            step,
            Step::InSubDir {
                path: PathBuf::from("app/server"),
                steps: vec![Step::With {
                    env: vec![EnvVar::new("NODE_ENV", "test")],
                    steps: vec![Step::Command("pnpm exec migrate".into())],
                }],
            }
        );
    }

    /// The tokeniser splits commands on `:` and `=`, so the parser has to put
    /// them back — an owned string is the price of a command containing one.
    #[test]
    fn rebuilds_commands_containing_colons_and_equals() {
        let step = parse_one_step(
            "\
serve:
  docker run -p 8080:80 -e FOO=bar image
",
        );

        assert_eq!(
            step,
            Step::Command("docker run -p 8080:80 -e FOO=bar image".into())
        );

        let Step::Command(line) = step else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(line, Cow::Owned(_)));
    }

    #[test]
    fn borrows_commands_that_need_no_rebuilding() {
        let step = parse_one_step(
            "\
build:
  pnpm run build
",
        );

        let Step::Command(line) = step else {
            panic!("expected a command, got {step:?}")
        };

        assert_eq!(line, "pnpm run build");
        assert!(
            matches!(line, Cow::Borrowed(_)),
            "a command that is a single word should borrow from the source"
        );
    }

    #[test]
    fn a_task_without_steps_is_an_error() {
        let error = parse(
            "\
empty:
other:
  echo hello
",
        )
        .expect_err("a task with no steps should not parse");

        assert_eq!(
            error.to_string(),
            "Task empty should have at least one step"
        );
    }

    #[test]
    fn an_in_block_without_steps_is_an_error() {
        let error = parse(
            "\
clean:
  in packages/shared:
",
        )
        .expect_err("an `in` block with no steps should not parse");

        assert_eq!(
            error.to_string(),
            "`in packages/shared` should have at least one step"
        );
    }
}
