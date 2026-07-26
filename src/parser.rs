use std::iter::Peekable;

use crate::{
    ast::{Step, Task, TaskFile},
    tokeniser::{Token, Tokeniser},
};

#[derive(Debug)]
pub struct ParseError {
    message: String,
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

        while let Some(step) = self.step(0)? {
            steps.push(step);
        }

        if steps.is_empty() {
            return Err(ParseError {
                message: format!("Task {name} should have at least one step"),
            });
        }

        Ok(Task::new(name, steps))
    }

    fn step(&mut self, prev_indent: usize) -> Result<Option<Step<'a>>, ParseError> {
        let indentation = match self.tokens.peek() {
            Some(Token::Spaces(indentation)) if *indentation > prev_indent => *indentation,
            _ => return Ok(None),
        };

        self.tokens.next();

        self.step_line(indentation).map(Some)
    }

    fn step_line(&mut self, prev_indent: usize) -> Result<Step<'a>, ParseError> {
        let mut line = String::new();

        let mut is_first_token = true;

        loop {
            match self.tokens.next() {
                Some(Token::NewLine) | None => return Ok(Step::Command(line.into())),
                Some(Token::With) => {
                    if is_first_token {
                        self.with_step(prev_indent)
                    } else {
                        line.push_str("with");
                    }
                }
                Some(Token::In) => {
                    if is_first_token {
                        self.in_step(prev_indent)
                    } else {
                        line.push_str("in");
                    }
                }
                Some(Token::Word(w)) => {
                    line.push_str(w);
                }
                Some(Token::Colon) => line.push(':'),
                Some(Token::Equals) => line.push('='),
                Some(Token::Spaces(n)) => {
                    for _ in 0..n {
                        line.push(' ');
                    }
                }
            }
        }
    }

    fn expect_next_token_to_be(&mut self, expected: Token<'a>) -> Result<Token<'a>, ParseError> {
        match self.tokens.next() {
            Some(t) if t == expected => Ok(t),
            Some(t) => Err(ParseError {
                message: format!("Expected {expected:?}, got {t:?}"),
            }),
            None => Err(ParseError {
                message: format!("Expected {expected:?} but reached end of the file"),
            }),
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
                        steps: vec![Step::Command("cargo install")],
                    }],
                ),
                // with-dir:
                //   in app/server:
                //     pnpm run build
                Task::new(
                    "with-dir",
                    vec![Step::InSubDir {
                        path: PathBuf::from("app/server"),
                        steps: vec![Step::Command("pnpm run build")],
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
                            steps: vec![Step::Command("pnpm run clean")],
                        },
                        Step::InSubDir {
                            path: PathBuf::from("app/client"),
                            steps: vec![Step::Command("pnpm run clean")],
                        },
                    ],
                ),
                // test-create-db:
                //   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
                Task::new(
                    "test-create-db",
                    vec![Step::Command(
                        "createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'",
                    )],
                ),
                // test-migrate:
                //   jute test-create-db
                //   with NODE_ENV=test:
                //     pnpm exec migrate
                Task::new(
                    "test-migrate",
                    vec![
                        Step::Command("jute test-create-db"),
                        Step::With {
                            env: vec![EnvVar::new("NODE_ENV", "test")],
                            steps: vec![Step::Command("pnpm exec migrate")],
                        },
                    ],
                ),
            ],
        };

        assert_eq!(file, expected);
    }
}
