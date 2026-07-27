use std::{borrow::Cow, iter::Peekable};

use crate::{
    ast::{Step, Task, TaskFile},
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
}

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

        while let Some(t) = self.tokens.next() {
            match t {
                Token::Word(w) => {
                    file.tasks.push(self.task(w)?);
                }
                Token::NewLine => {}
                t => unimplemented!("{t:?}"),
            }
        }

        Ok(file)
    }

    fn task(&mut self, name: &'a str) -> Result<Task<'a>, ParseError> {
        let mut steps: Vec<Step<'a>> = Vec::new();

        self.expect_next_token_to_be(Token::Colon)?;
        self.expect_next_token_to_be(Token::NewLine)?;
        self.expect_next_token_to_be(Token::Indent)?;

        while let Some(step) = self.step(1)? {
            steps.push(step);
        }

        if steps.is_empty() {
            return Err(ParseError::new(format!(
                "Task {name} should have at least one step"
            )));
        }

        Ok(Task::new(name, steps))
    }

    fn step(&mut self, indent_level: usize) -> Result<Option<Step<'a>>, ParseError> {
        let token = self.tokens.next();

        match token {
            None | Some(Token::NewLine) => Ok(None),
            Some(Token::With) => self.with_step(indent_level).map(Some),
            Some(Token::In) => self.in_step(indent_level).map(Some),
            Some(Token::Word(w)) => self.command(w).map(Some),
            Some(Token::Colon) | Some(Token::Equals) => Err(ParseError::new(format!(
                "Unexpected token when parsing a step: {token:?}"
            ))),
            Some(Token::Indent) | Some(Token::Dedent) => Err(ParseError::new(
                "BUG! Parser got confused by indentation, please report the file that caused this bug",
            )),
        }
    }

    fn command(&mut self, first_word: &'a str) -> Result<Step<'a>, ParseError> {
        let mut line = first_word.to_string(); // TODO: ideally make a Cow if possible

        loop {
            let token = self.tokens.next();

            match token {
                Some(Token::NewLine) | None => return Ok(Step::Command(line.into())),
                Some(Token::With) => {
                    line.push_str("with");
                }
                Some(Token::In) => {
                    line.push_str("in");
                }
                Some(Token::Word(w)) => {
                    line.push_str(w);
                }
                Some(Token::Colon) => line.push(':'),
                Some(Token::Equals) => line.push('='),
                Some(Token::Indent) | Some(Token::Dedent) => {
                    return Err(ParseError::new(
                        "BUG! Parser got confused by indentation, please report the file that caused this bug",
                    ));
                }
            }
        }
    }

    fn with_step(&mut self, indent_level: usize) -> Result<Step<'a>, ParseError> {
        todo!()
    }

    fn in_step(&mut self, indent_level: usize) -> Result<Step<'a>, ParseError> {
        todo!()
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
                    vec![Step::Command("createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'".into())],
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
}
