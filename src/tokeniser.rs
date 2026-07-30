use std::{iter::Peekable, str::CharIndices};

use crate::tokeniser::Token::{Dedent, Indent};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'a> {
    Indent,
    Dedent,
    /// A run of spaces within a line, carrying its width so a command can be
    /// rebuilt exactly. Spaces at the start of a line are indentation, and
    /// become `Indent`/`Dedent` instead.
    Spaces(usize),
    NewLine,
    Colon,
    With,
    In,
    Equals,
    Word(&'a str),
}

#[derive(Debug)]
pub struct Tokeniser<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    just_had_new_line: bool,
    indent_stack: Vec<usize>,
}

impl<'a> Iterator for Tokeniser<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.just_had_new_line
            && self.peek_next_char().is_none_or(|c| !c.is_whitespace())
            && self.indent_stack.pop().is_some()
        {
            return Some(Dedent);
        }

        let (i, c) = self.chars.next()?;

        if self.just_had_new_line && c == ' ' {
            self.just_had_new_line = false;

            let mut indent = 1;
            while self.peek_next_char() == Some(' ') {
                self.chars.next();
                indent += 1;
            }

            if let Some('\n' | '\r') = self.peek_next_char() {
                // ignore blank lines
                return self.next();
            }

            if let Some(change) = self.indentation_changed(indent) {
                return Some(change);
            }

            return self.next();
        }

        Some(self.handle_char(c, i))
    }
}

impl<'a> Tokeniser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            just_had_new_line: true,
            indent_stack: Vec::new(),
        }
    }

    pub fn indentation_changed(&mut self, new_indent: usize) -> Option<Token<'static>> {
        let prev = self.indent_stack.last().copied().unwrap_or_default();

        if new_indent == prev {
            return None;
        }

        if new_indent > prev {
            self.indent_stack.push(new_indent);
            Some(Indent)
        } else {
            self.indent_stack.pop();
            Some(Dedent)
        }
    }

    pub fn peek_next_index(&mut self) -> usize {
        self.chars.peek().map_or(self.source.len(), |(i, _)| *i)
    }

    pub fn peek_next_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    pub fn handle_char(&mut self, c: char, index: usize) -> Token<'a> {
        self.just_had_new_line = false;

        match c {
            '\n' | '\r' => {
                self.just_had_new_line = true;
                Token::NewLine
            }
            ':' => Token::Colon,
            '=' => Token::Equals,
            ' ' => {
                let mut count = 1;

                while self.peek_next_char() == Some(' ') {
                    self.chars.next();
                    count += 1;
                }

                Token::Spaces(count)
            }
            _ => {
                let mut end_index = self.peek_next_index();

                while self
                    .peek_next_char()
                    .is_some_and(|c| !['\n', '\r', ':', '=', ' '].contains(&c))
                {
                    self.chars.next();

                    end_index = self.peek_next_index();
                }

                match &self.source[index..end_index] {
                    "with" => Token::With,
                    "in" => Token::In,
                    word => Token::Word(word),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token::*;
    use super::Tokeniser;
    use std::{fs, path::Path};

    fn read_example_file(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(name);

        fs::read_to_string(path).unwrap()
    }

    /// Tokenises a source string, for cases too small to be worth an example file.
    fn tokenise(source: &str) -> Vec<super::Token<'_>> {
        Tokeniser::new(source).collect()
    }

    #[test]
    fn tokenises_example_001() {
        let source = read_example_file("001.jute");

        let tokens: Vec<_> = Tokeniser::new(&source).collect();

        #[rustfmt::skip]
        let expected = vec![
            //  1:
            NewLine,
            //  2:
            NewLine,
            //  3: simple:
            Word("simple"), Colon, NewLine,
            //  4:   in app/server:
            Indent, In, Spaces(1), Word("app/server"), Colon, NewLine,
            //  5:     cargo install
            Indent, Word("cargo"), Spaces(1), Word("install"), NewLine,
            //  6:
            NewLine,
            //  7:
            NewLine,
            //  8: with-dir:
            Dedent, Dedent, Word("with-dir"), Colon, NewLine,
            //  9:   in app/server:
            Indent, In, Spaces(1), Word("app/server"), Colon, NewLine,
            // 10:     pnpm run build
            Indent, Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("build"), NewLine,
            // 11:
            NewLine,
            // 12:
            NewLine,
            // 13: clean:
            Dedent, Dedent, Word("clean"), Colon, NewLine,
            // 14:   in packages/shared:
            Indent, In, Spaces(1), Word("packages/shared"), Colon, NewLine,
            // 15:     pnpm run clean
            Indent, Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("clean"), NewLine,
            // 16:
            NewLine,
            // 17:   in app/client:
            Dedent, In, Spaces(1), Word("app/client"), Colon, NewLine,
            // 18:     pnpm run clean
            Indent, Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("clean"), NewLine,
            // 19:
            NewLine,
            // 20: test-create-db:
            Dedent, Dedent, Word("test-create-db"), Colon, NewLine,
            // 21:   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
            Indent, Word("createdb"), Spaces(1), Word("test_db"), Spaces(1), Word("&&"), Spaces(1),
            Word("psql"), Spaces(1), Word("-c"), Spaces(1), Word("'CREATE"), Spaces(1),
            Word("EXTENSION"), Spaces(1), Word("IF"), Spaces(1), Word("NOT"), Spaces(1),
            Word("EXISTS"), Spaces(1), Word("vector;'"), NewLine,
            // 22:
            NewLine,
            // 23: test-migrate:
            Dedent, Word("test-migrate"), Colon, NewLine,
            // 24:   jute test-create-db
            Indent, Word("jute"), Spaces(1), Word("test-create-db"), NewLine,
            // 25:   with NODE_ENV=test:
            With, Spaces(1), Word("NODE_ENV"), Equals, Word("test"), Colon, NewLine,
            // 26:     pnpm exec migrate
            Indent, Word("pnpm"), Spaces(1), Word("exec"), Spaces(1), Word("migrate"), NewLine,
            // end of file: close the two blocks still open
            Dedent, Dedent,
        ];

        assert_eq!(tokens, expected);
    }

    /// The reason `Spaces` exists: `A=1 B=2` used to tokenise as `A`, `=`,
    /// `1 B`, `=`, `2`, which left the parser to work out where one variable
    /// ended and the next began.
    #[test]
    fn a_space_between_env_vars_is_its_own_token() {
        #[rustfmt::skip]
        let expected = vec![
            Word("t"), Colon, NewLine,
            Indent, With, Spaces(1), Word("A"), Equals, Word("1"),
            Spaces(1), Word("B"), Equals, Word("2"), Colon, NewLine,
            Indent, Word("echo"), Spaces(1), Word("hi"), NewLine,
            Dedent, Dedent,
        ];

        assert_eq!(tokenise("t:\n  with A=1 B=2:\n    echo hi\n"), expected);
    }

    /// A run of spaces is one token carrying its width, so a command can be
    /// put back together exactly as it was written.
    #[test]
    fn runs_of_spaces_are_batched_and_keep_their_width() {
        #[rustfmt::skip]
        let expected = vec![
            Word("t"), Colon, NewLine,
            Indent, Word("sed"), Spaces(1), Word("'s/a"), Spaces(3), Word("b/c/'"), NewLine,
            Dedent,
        ];

        assert_eq!(tokenise("t:\n  sed 's/a   b/c/'\n"), expected);
    }

    /// `with` and `in` are matched as whole words, so `with-dir` is a name and
    /// a bare `in` is a keyword wherever it appears. The parser turns the
    /// keywords back into text where it wanted a word.
    #[test]
    fn keywords_are_matched_as_whole_words() {
        #[rustfmt::skip]
        let expected = vec![
            Word("with-dir"), Colon, NewLine,
            Indent, Word("for"), Spaces(1), Word("f"), Spaces(1), In, Spaces(1),
            Word("*.txt"), NewLine,
            Dedent,
        ];

        assert_eq!(tokenise("with-dir:\n  for f in *.txt\n"), expected);
    }
}
