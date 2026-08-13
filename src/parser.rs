use std::{borrow::Cow, fmt, iter::Peekable, path::PathBuf};

use crate::{
    ast::{EnvVar, Step, Task, TaskFile},
    tokeniser::{Token, Tokeniser},
};

pub fn parse_tasks_file(raw: &str) -> Result<TaskFile<'_>, ParseError> {
    Parser::new(Tokeniser::new(raw)).parse()
}

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
                Token::NewLine | Token::Spaces(_) => {}
                Token::Indent | Token::Dedent => return Err(ParseError::indentation()),
                t => match Self::token_text(&t) {
                    Some(name) => file.insert_task(self.task(name)?)?,
                    None => {
                        return Err(ParseError::new(format!("Expected a task name, got {t:?}")));
                    }
                },
            }
        }

        Ok(file)
    }

    fn task(&mut self, name: &'a str) -> Result<Task<'a>, ParseError> {
        self.skip_spaces();

        // A task is invoked as `jute <name>`, so a name holding a space would
        // be ambiguous on the command line.
        if self.peek_word().is_some() {
            return Err(ParseError::new(format!(
                "Task names cannot contain spaces, so `{name}` should be one word"
            )));
        }

        self.expect_next_token_to_be(Token::Colon)?;
        self.skip_spaces();
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
        let program = Cow::Borrowed(first_word);

        let mut args = Vec::new();
        let mut cur_arg: Option<Cow<'a, str>> = None;

        loop {
            let token = self.tokens.next();

            match token {
                Some(Token::NewLine) | None => {
                    args.extend(cur_arg);
                    return Ok(Step::Command(program, args));
                }
                Some(Token::Spaces(_)) => args.extend(cur_arg.take()),
                Some(Token::With) => Self::extend_arg(&mut cur_arg, "with"),
                Some(Token::In) => Self::extend_arg(&mut cur_arg, "in"),
                Some(Token::Word(w)) => Self::extend_arg(&mut cur_arg, w),
                Some(Token::Colon) => Self::extend_arg(&mut cur_arg, ":"),
                Some(Token::Equals) => Self::extend_arg(&mut cur_arg, "="),
                Some(Token::DoubleQuotedString(s) | Token::SingleQuotedString(s)) => {
                    Self::extend_arg(&mut cur_arg, s)
                }
                Some(Token::UnterminatedString(quote)) => {
                    return Err(ParseError::new(format!(
                        "Unterminated string: missing closing `{quote}`"
                    )));
                }
                Some(Token::Indent) | Some(Token::Dedent) => {
                    return Err(ParseError::indentation());
                }
            }
        }
    }

    /// Appends `text` to the argument being built, starting a new (borrowed
    /// where possible) one if none is in progress yet. Only allocates when a
    /// token boundary (`:`, `=`, a keyword, an escape inside a quoted
    /// string) falls inside a single argument.
    fn extend_arg<S: Into<Cow<'a, str>>>(arg: &mut Option<Cow<'a, str>>, text: S) {
        let text = text.into();

        match arg {
            Some(a) => a.to_mut().push_str(&text),
            None => *arg = Some(text),
        }
    }

    /// `with NAME=value [NAME=value ...]:` followed by a block.
    ///
    /// A space always separates one variable from the next, as it does in
    /// `env A=1 B=2 cmd`, so a value cannot itself contain one.
    fn with_step(&mut self) -> Result<Step<'a>, ParseError> {
        let mut env = Vec::new();

        loop {
            self.skip_spaces();

            let name = self.next_word("Expected an environment variable name after `with`")?;

            self.expect_next_token_to_be(Token::Equals)?;

            let value = self.next_word(&format!("Expected a value for `{name}`"))?;

            env.push(EnvVar::new(name, value));

            self.skip_spaces();

            // Two words are never adjacent, so a word here was preceded by a
            // space and starts the next variable. Anything else — a `:`, most
            // often — ends the list, which makes the spaces just skipped
            // trailing whitespace.
            if self.peek_word().is_none() {
                break;
            }
        }

        self.expect_next_token_to_be(Token::Colon)?;
        self.skip_spaces();
        self.expect_next_token_to_be(Token::NewLine)?;

        let steps = self.block("A `with` block")?;

        Ok(Step::With { env, steps })
    }

    /// `in <path>:` followed by a block. Everything up to the `:` is the path,
    /// so a directory whose name holds a space still works.
    fn in_step(&mut self) -> Result<Step<'a>, ParseError> {
        self.skip_spaces();

        let mut path = String::from(self.next_word("Expected a directory after `in`")?);

        while let Some(Token::Spaces(n)) = self.tokens.next_if(Self::is_spaces) {
            let Some(word) = self.peek_word() else {
                // trailing whitespace before the `:`
                break;
            };

            self.tokens.next();
            path.extend(std::iter::repeat_n(' ', n));
            path.push_str(word);
        }

        self.expect_next_token_to_be(Token::Colon)?;
        self.skip_spaces();
        self.expect_next_token_to_be(Token::NewLine)?;

        let steps = self.block(&format!("`in {path}`"))?;

        Ok(Step::InSubDir {
            path: PathBuf::from(path),
            steps,
        })
    }

    /// The text of a token that can stand in for a word. `with` and `in` are
    /// keywords wherever they appear, so they have to be turned back into text
    /// in the places the grammar wanted a name, a path or a value.
    fn token_text(token: &Token<'a>) -> Option<&'a str> {
        match token {
            Token::Word(w) => Some(*w),
            Token::With => Some("with"),
            Token::In => Some("in"),
            _ => None,
        }
    }

    /// The text of the next token, without consuming it, if it can stand in
    /// for a word.
    fn peek_word(&mut self) -> Option<&'a str> {
        self.tokens.peek().and_then(Self::token_text)
    }

    fn next_word(&mut self, expectation: &str) -> Result<&'a str, ParseError> {
        let token = self.tokens.next();

        match token.as_ref().and_then(Self::token_text) {
            Some(word) => Ok(word),
            None => Err(ParseError::new(format!("{expectation}, got {token:?}"))),
        }
    }

    fn is_spaces(token: &Token<'a>) -> bool {
        matches!(token, Token::Spaces(_))
    }

    fn skip_new_lines(&mut self) {
        while self.tokens.next_if_eq(&Token::NewLine).is_some() {}
    }

    /// Spaces carry no meaning of their own outside a command, but they are
    /// still tokens, so every position that ignores them says so.
    fn skip_spaces(&mut self) {
        while self.tokens.next_if(Self::is_spaces).is_some() {}
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

    /// The `TaskFile` a test expects to see, built through the same public API
    /// the parser itself uses.
    fn task_file<'a>(tasks: impl IntoIterator<Item = Task<'a>>) -> TaskFile<'a> {
        let mut file = TaskFile::default();

        for task in tasks {
            file.insert_task(task).expect("task names should be unique");
        }

        file
    }

    #[test]
    fn parses_example_001() {
        let source = read_example_file("001.jute");

        let file = parse_tasks_file(&source).expect("001.jute should parse");

        let expected = task_file([
            // simple:
            //   in app/server:
            //     cargo install
            Task::new(
                "simple",
                vec![Step::InSubDir {
                    path: PathBuf::from("app/server"),
                    steps: vec![Step::Command("cargo".into(), vec!["install".into()])],
                }],
            ),
            // with-dir:
            //   in app/server:
            //     pnpm run build
            Task::new(
                "with-dir",
                vec![Step::InSubDir {
                    path: PathBuf::from("app/server"),
                    steps: vec![Step::Command(
                        "pnpm".into(),
                        vec!["run".into(), "build".into()],
                    )],
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
                        steps: vec![Step::Command(
                            "pnpm".into(),
                            vec!["run".into(), "clean".into()],
                        )],
                    },
                    Step::InSubDir {
                        path: PathBuf::from("app/client"),
                        steps: vec![Step::Command(
                            "pnpm".into(),
                            vec!["run".into(), "clean".into()],
                        )],
                    },
                ],
            ),
            // test-create-db:
            //   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
            Task::new(
                "test-create-db",
                vec![Step::Command(
                    "createdb".into(),
                    vec![
                        "test_db".into(),
                        "&&".into(),
                        "psql".into(),
                        "-c".into(),
                        "CREATE EXTENSION IF NOT EXISTS vector;".into(),
                    ],
                )],
            ),
            // test-migrate:
            //   jute test-create-db
            //   with NODE_ENV=test:
            //     pnpm exec migrate
            Task::new(
                "test-migrate",
                vec![
                    Step::Command("jute".into(), vec!["test-create-db".into()]),
                    Step::With {
                        env: vec![EnvVar::new("NODE_ENV", "test")],
                        steps: vec![Step::Command(
                            "pnpm".into(),
                            vec!["exec".into(), "migrate".into()],
                        )],
                    },
                ],
            ),
        ]);

        assert_eq!(file, expected);
    }

    #[test]
    fn parses_several_env_vars_in_one_with() {
        let file = parse_tasks_file(
            "\
build:
  with NODE_ENV=production DEBUG=0:
    pnpm run build
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "build",
                vec![Step::With {
                    env: vec![
                        EnvVar::new("NODE_ENV", "production"),
                        EnvVar::new("DEBUG", "0"),
                    ],
                    steps: vec![Step::Command(
                        "pnpm".into(),
                        vec!["run".into(), "build".into()]
                    )],
                }],
            )])
        );
    }

    #[test]
    fn parses_a_with_block_nested_in_an_in_block() {
        let file = parse_tasks_file(
            "\
migrate:
  in app/server:
    with NODE_ENV=test:
      pnpm exec migrate
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "migrate",
                vec![Step::InSubDir {
                    path: PathBuf::from("app/server"),
                    steps: vec![Step::With {
                        env: vec![EnvVar::new("NODE_ENV", "test")],
                        steps: vec![Step::Command(
                            "pnpm".into(),
                            vec!["exec".into(), "migrate".into()]
                        )],
                    }],
                }],
            )])
        );
    }

    /// The tokeniser splits commands on `:` and `=`, so the parser has to put
    /// them back — an owned string is the price of a command containing one.
    #[test]
    fn rebuilds_commands_containing_colons_and_equals() {
        let file = parse_tasks_file(
            "\
serve:
  docker run -p 8080:80 -e FOO=bar image
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "serve",
                vec![Step::Command(
                    "docker".into(),
                    vec![
                        "run".into(),
                        "-p".into(),
                        "8080:80".into(),
                        "-e".into(),
                        "FOO=bar".into(),
                        "image".into(),
                    ],
                )],
            )])
        );

        let Step::Command(_, args) = &file.get("serve").expect("serve should exist").steps[0]
        else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(args[2], Cow::Owned(_)));
    }

    /// Words are split on spaces, so only a genuinely one-word command reaches
    /// the parser as a single `Word` and avoids being rebuilt.
    #[test]
    fn borrows_commands_that_need_no_rebuilding() {
        let file = parse_tasks_file(
            "\
build:
  ./build.sh
",
        )
        .expect("source should parse");

        let step = &file.get("build").expect("build should exist").steps[0];

        let Step::Command(program, args) = step else {
            panic!("expected a command, got {step:?}")
        };

        assert_eq!(program, "./build.sh");
        assert_eq!(args, &Vec::<Cow<str>>::new());
        assert!(
            matches!(program, Cow::Borrowed(_)),
            "a command that is a single word should borrow from the source"
        );
    }

    /// A run of spaces is still just one separator between arguments, so it
    /// doesn't produce empty/phantom args.
    #[test]
    fn a_run_of_spaces_is_a_single_argument_separator() {
        let file = parse_tasks_file(
            "\
fix:
  sed s/a   b/c/ file
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "fix",
                vec![Step::Command(
                    "sed".into(),
                    vec!["s/a".into(), "b/c/".into(), "file".into()],
                )],
            )])
        );
    }

    /// `in` and `with` are keywords wherever they appear, so a command using
    /// one as an ordinary word has to be put back together.
    #[test]
    fn rebuilds_commands_containing_keywords() {
        let file = parse_tasks_file(
            "\
list:
  for f in *.txt; do echo $f; done
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "list",
                vec![Step::Command(
                    "for".into(),
                    vec![
                        "f".into(),
                        "in".into(),
                        "*.txt;".into(),
                        "do".into(),
                        "echo".into(),
                        "$f;".into(),
                        "done".into(),
                    ],
                )],
            )])
        );
    }

    #[test]
    fn parses_a_directory_whose_name_contains_spaces() {
        let file = parse_tasks_file(
            "\
build:
  in My Project/server:
    cargo build
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "build",
                vec![Step::InSubDir {
                    path: PathBuf::from("My Project/server"),
                    steps: vec![Step::Command("cargo".into(), vec!["build".into()])],
                }],
            )])
        );
    }

    /// Spaces around the structural parts of a line carry no meaning.
    #[test]
    fn ignores_spaces_around_colons() {
        let file = parse_tasks_file(
            "\
build :
  with A=1 :
    in src :
      cargo build
",
        )
        .expect("spaces before a `:` should be ignored");

        assert_eq!(
            file,
            task_file([Task::new(
                "build",
                vec![Step::With {
                    env: vec![EnvVar::new("A", "1")],
                    steps: vec![Step::InSubDir {
                        path: PathBuf::from("src"),
                        steps: vec![Step::Command("cargo".into(), vec!["build".into()])],
                    }],
                }],
            )])
        );
    }

    #[test]
    fn a_task_name_containing_a_space_is_an_error() {
        let error = parse_tasks_file(
            "\
my task:
  echo hello
",
        )
        .expect_err("a task name with a space should not parse");

        assert_eq!(
            error.to_string(),
            "Task names cannot contain spaces, so `my` should be one word"
        );
    }

    /// A space always separates one variable from the next, so there is no way
    /// for a value to hold one.
    #[test]
    fn an_env_var_value_containing_a_space_is_an_error() {
        let error = parse_tasks_file(
            "\
greet:
  with MSG=hello world:
    ./greet.sh
",
        )
        .expect_err("an env var value with a space should not parse");

        assert_eq!(error.to_string(), "Expected Equals, got Colon");
    }

    /// `with` and `in` are keywords, but the parser turns them back into text
    /// where it wanted a name or a value.
    #[test]
    fn keywords_can_be_used_as_env_var_names_and_values() {
        let file = parse_tasks_file(
            "\
run:
  with in=with:
    ./run.sh
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "run",
                vec![Step::With {
                    env: vec![EnvVar::new("in", "with")],
                    steps: vec![Step::Command("./run.sh".into(), vec![])],
                }],
            )])
        );
    }

    #[test]
    fn a_task_without_steps_is_an_error() {
        let error = parse_tasks_file(
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
        let error = parse_tasks_file(
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

    /// The motivating case: a quoted argument holding a space stays one
    /// argument, and with no escapes to decode it borrows from the source.
    #[test]
    fn a_double_quoted_argument_with_a_space_is_one_borrowed_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo \"hello world\"
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["hello world".into()])],
            )])
        );

        let Step::Command(_, args) = &file.get("greet").expect("greet should exist").steps[0]
        else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(args[0], Cow::Borrowed(_)));
    }

    /// `\"` and `\\` are decoded, which forces the argument to be rebuilt.
    #[test]
    fn a_double_quoted_argument_with_escapes_is_decoded_and_owned() {
        let file = parse_tasks_file(
            "\
greet:
  echo \"say \\\"hi\\\" or \\\\bye\"
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command(
                    "echo".into(),
                    vec!["say \"hi\" or \\bye".into()],
                )],
            )])
        );

        let Step::Command(_, args) = &file.get("greet").expect("greet should exist").steps[0]
        else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(args[0], Cow::Owned(_)));
    }

    /// No space separates a word from an adjacent quoted string, so they're
    /// the same argument — the same rebuilding `extend_arg` already does for
    /// `:`/`=`/keywords.
    #[test]
    fn a_word_glued_to_a_double_quoted_string_is_one_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo foo\"bar baz\"
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["foobar baz".into()])],
            )])
        );
    }

    #[test]
    fn an_empty_double_quoted_string_is_an_empty_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo \"\"
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["".into()])],
            )])
        );
    }

    #[test]
    fn an_unterminated_double_quoted_string_is_an_error() {
        let error = parse_tasks_file(
            "\
greet:
  echo \"oops
",
        )
        .expect_err("an unterminated string should not parse");

        assert_eq!(
            error.to_string(),
            "Unterminated string: missing closing `\"`"
        );
    }

    /// Quoted strings are single-line only, so a raw newline inside one ends
    /// it rather than being included in the argument.
    #[test]
    fn a_double_quoted_string_cannot_span_a_newline() {
        let error = parse_tasks_file(
            "\
greet:
  echo \"hello
  world\"
",
        )
        .expect_err("a string spanning a newline should not parse");

        assert_eq!(
            error.to_string(),
            "Unterminated string: missing closing `\"`"
        );
    }

    /// Single quotes group an argument holding a space just as double quotes
    /// do, and with no escapes to decode the argument borrows from the source.
    #[test]
    fn a_single_quoted_argument_with_a_space_is_one_borrowed_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo 'hello world'
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["hello world".into()])],
            )])
        );

        let Step::Command(_, args) = &file.get("greet").expect("greet should exist").steps[0]
        else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(args[0], Cow::Borrowed(_)));
    }

    /// `\'` and `\\` are decoded, which forces the argument to be rebuilt.
    #[test]
    fn a_single_quoted_argument_with_escapes_is_decoded_and_owned() {
        let file = parse_tasks_file(
            "\
greet:
  echo 'say \\'hi\\' or \\\\bye'
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command(
                    "echo".into(),
                    vec!["say 'hi' or \\bye".into()]
                )],
            )])
        );

        let Step::Command(_, args) = &file.get("greet").expect("greet should exist").steps[0]
        else {
            unreachable!("asserted to be a command above")
        };

        assert!(matches!(args[0], Cow::Owned(_)));
    }

    /// Spaces are an argument separator outside a quoted string but ordinary
    /// characters inside one, so a run of them survives intact.
    #[test]
    fn a_run_of_spaces_inside_a_quoted_string_is_kept_verbatim() {
        let file = parse_tasks_file(
            "\
fix:
  sed 's/a   b/c/' file
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "fix",
                vec![Step::Command(
                    "sed".into(),
                    vec!["s/a   b/c/".into(), "file".into()],
                )],
            )])
        );
    }

    #[test]
    fn a_word_glued_to_a_single_quoted_string_is_one_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo foo'bar baz'
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["foobar baz".into()])],
            )])
        );
    }

    #[test]
    fn an_empty_single_quoted_string_is_an_empty_argument() {
        let file = parse_tasks_file(
            "\
greet:
  echo ''
",
        )
        .expect("source should parse");

        assert_eq!(
            file,
            task_file([Task::new(
                "greet",
                vec![Step::Command("echo".into(), vec!["".into()])],
            )])
        );
    }

    /// The error names the quote that opened the string, not whichever one the
    /// tokeniser happens to scan first.
    #[test]
    fn an_unterminated_single_quoted_string_is_an_error() {
        let error = parse_tasks_file(
            "\
greet:
  echo 'oops
",
        )
        .expect_err("an unterminated string should not parse");

        assert_eq!(
            error.to_string(),
            "Unterminated string: missing closing `'`"
        );
    }

    #[test]
    fn a_single_quoted_string_cannot_span_a_newline() {
        let error = parse_tasks_file(
            "\
greet:
  echo 'hello
  world'
",
        )
        .expect_err("a string spanning a newline should not parse");

        assert_eq!(
            error.to_string(),
            "Unterminated string: missing closing `'`"
        );
    }
}
