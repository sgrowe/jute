use std::{iter::Peekable, str::CharIndices};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'a> {
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
}

impl<'a> Iterator for Tokeniser<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (i, c) = self.chars.next()?;

        Some(self.handle_char(c, i))
    }
}

impl<'a> Tokeniser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
        }
    }

    pub fn peek_next_index(&mut self) -> usize {
        self.chars.peek().map_or(self.source.len(), |(i, _)| *i)
    }

    pub fn peek_next_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    pub fn handle_char(&mut self, c: char, index: usize) -> Token<'a> {
        match c {
            '\n' | '\r' => Token::NewLine,
            ' ' => {
                let mut count = 1;

                while self.peek_next_char().map_or(false, |c| c == ' ') {
                    count += 1;
                    self.chars.next();
                }

                Token::Spaces(count)
            }
            ':' => Token::Colon,
            '=' => Token::Equals,
            _ => {
                let mut end_index = self.peek_next_index();

                while self
                    .peek_next_char()
                    .map_or(false, |c| !c.is_whitespace() && c != ':' && c != '=')
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
            Spaces(2), In, Spaces(1), Word("app/server"), Colon, NewLine,
            //  5:     cargo install
            Spaces(4), Word("cargo"), Spaces(1), Word("install"), NewLine,
            //  6:
            NewLine,
            //  7:
            NewLine,
            //  8: with-dir:
            Word("with-dir"), Colon, NewLine,
            //  9:   in app/server:
            Spaces(2), In, Spaces(1), Word("app/server"), Colon, NewLine,
            // 10:     pnpm run build
            Spaces(4), Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("build"), NewLine,
            // 11:
            NewLine,
            // 12:
            NewLine,
            // 13: clean:
            Word("clean"), Colon, NewLine,
            // 14:   in packages/shared:
            Spaces(2), In, Spaces(1), Word("packages/shared"), Colon, NewLine,
            // 15:     pnpm run clean
            Spaces(4), Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("clean"), NewLine,
            // 16:
            NewLine,
            // 17:   in app/client:
            Spaces(2), In, Spaces(1), Word("app/client"), Colon, NewLine,
            // 18:     pnpm run clean
            Spaces(4), Word("pnpm"), Spaces(1), Word("run"), Spaces(1), Word("clean"), NewLine,
            // 19:
            NewLine,
            // 20: test-create-db:
            Word("test-create-db"), Colon, NewLine,
            // 21:   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
            Spaces(2), Word("createdb"), Spaces(1), Word("test_db"), Spaces(1), Word("&&"), Spaces(1),
            Word("psql"), Spaces(1), Word("-c"), Spaces(1), Word("'CREATE"), Spaces(1), Word("EXTENSION"),
            Spaces(1), Word("IF"), Spaces(1), Word("NOT"), Spaces(1), Word("EXISTS"), Spaces(1),
            Word("vector;'"), NewLine,
            // 22:
            NewLine,
            // 23: test-migrate:
            Word("test-migrate"), Colon, NewLine,
            // 24:   jute test-create-db
            Spaces(2), Word("jute"), Spaces(1), Word("test-create-db"), NewLine,
            // 25:   with NODE_ENV=test:
            Spaces(2), With, Spaces(1), Word("NODE_ENV"), Equals, Word("test"), Colon,
            NewLine,
            // 26:     pnpm exec migrate
            Spaces(4), Word("pnpm"), Spaces(1), Word("exec"), Spaces(1), Word("migrate"), NewLine,
        ];

        assert_eq!(tokens, expected);
    }
}
