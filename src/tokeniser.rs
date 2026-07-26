use std::{iter::Peekable, str::CharIndices};

use crate::tokeniser::Token::{Dedent, Indent};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'a> {
    Indent,
    Dedent,
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
        if self.just_had_new_line && self.peek_next_char().map_or(true, |c| !c.is_whitespace()) {
            if self.indent_stack.pop().is_some() {
                return Some(Dedent);
            }
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
            return Some(Indent);
        } else {
            self.indent_stack.pop();
            return Some(Dedent);
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
            _ => {
                let mut end_index = self.peek_next_index();

                while self
                    .peek_next_char()
                    .map_or(false, |c| !['\n', '\r', ':', '='].contains(&c))
                {
                    self.chars.next();

                    end_index = self.peek_next_index();

                    match &self.source[index..end_index] {
                        "with " => return Token::With,
                        "in " => return Token::In,
                        _ => {}
                    }
                }

                Token::Word(&self.source[index..end_index])
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
            Indent, In, Word("app/server"), Colon, NewLine,
            //  5:     cargo install
            Indent, Word("cargo install"), NewLine,
            //  6:
            NewLine,
            //  7:
            NewLine,
            //  8: with-dir:
            Dedent, Dedent, Word("with-dir"), Colon, NewLine,
            //  9:   in app/server:
            Indent, In, Word("app/server"), Colon, NewLine,
            // 10:     pnpm run build
            Indent, Word("pnpm run build"), NewLine,
            // 11:
            NewLine,
            // 12:
            NewLine,
            // 13: clean:
            Dedent, Dedent, Word("clean"), Colon, NewLine,
            // 14:   in packages/shared:
            Indent, In, Word("packages/shared"), Colon, NewLine,
            // 15:     pnpm run clean
            Indent, Word("pnpm run clean"), NewLine,
            // 16:
            NewLine,
            // 17:   in app/client:
            Dedent, In, Word("app/client"), Colon, NewLine,
            // 18:     pnpm run clean
            Indent, Word("pnpm run clean"), NewLine,
            // 19:
            NewLine,
            // 20: test-create-db:
            Dedent, Dedent, Word("test-create-db"), Colon, NewLine,
            // 21:   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
            Indent, Word("createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'"), NewLine,
            // 22:
            NewLine,
            // 23: test-migrate:
            Dedent, Word("test-migrate"), Colon, NewLine,
            // 24:   jute test-create-db
            Indent, Word("jute test-create-db"), NewLine,
            // 25:   with NODE_ENV=test:
            With, Word("NODE_ENV"), Equals, Word("test"), Colon, NewLine,
            // 26:     pnpm exec migrate
            Indent, Word("pnpm exec migrate"), NewLine,
            // end of file: close the two blocks still open
            Dedent, Dedent,
        ];

        assert_eq!(tokens, expected);
    }
}
