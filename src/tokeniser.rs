use std::{iter::Peekable, str::CharIndices};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'a> {
    Space,
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

    pub fn cheeky_peek(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    pub fn peek_next_index(&mut self) -> usize {
        self.cheeky_peek().map_or(self.source.len(), |(i, _)| i)
    }

    pub fn handle_char(&mut self, c: char, index: usize) -> Token<'a> {
        match c {
            '\n' | '\r' => Token::NewLine,
            ' ' => Token::Space,
            ':' => Token::Colon,
            '=' => Token::Equals,
            _ => {
                let mut end_index = self.peek_next_index();

                while self
                    .cheeky_peek()
                    .map(|(_, c)| !c.is_whitespace() && c != ':' && c != '=')
                    .unwrap_or_default()
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
            Space, Space, In, Space, Word("app/server"), Colon, NewLine,
            //  5:     cargo install
            Space, Space, Space, Space, Word("cargo"), Space, Word("install"), NewLine,
            //  6:
            NewLine,
            //  7:
            NewLine,
            //  8: with-dir:
            Word("with-dir"), Colon, NewLine,
            //  9:   in app/server:
            Space, Space, In, Space, Word("app/server"), Colon, NewLine,
            // 10:     pnpm run build
            Space, Space, Space, Space, Word("pnpm"), Space, Word("run"), Space,
            Word("build"), NewLine,
            // 11:
            NewLine,
            // 12:
            NewLine,
            // 13: clean:
            Word("clean"), Colon, NewLine,
            // 14:   in packages/shared:
            Space, Space, In, Space, Word("packages/shared"), Colon, NewLine,
            // 15:     pnpm run clean
            Space, Space, Space, Space, Word("pnpm"), Space, Word("run"), Space,
            Word("clean"), NewLine,
            // 16:
            NewLine,
            // 17:   in app/client:
            Space, Space, In, Space, Word("app/client"), Colon, NewLine,
            // 18:     pnpm run clean
            Space, Space, Space, Space, Word("pnpm"), Space, Word("run"), Space,
            Word("clean"), NewLine,
            // 19:
            NewLine,
            // 20: test-create-db:
            Word("test-create-db"), Colon, NewLine,
            // 21:   createdb test_db && psql -c 'CREATE EXTENSION IF NOT EXISTS vector;'
            Space, Space, Word("createdb"), Space, Word("test_db"), Space, Word("&&"), Space,
            Word("psql"), Space, Word("-c"), Space, Word("'CREATE"), Space, Word("EXTENSION"), Space,
            Word("IF"), Space, Word("NOT"), Space, Word("EXISTS"), Space, Word("vector;'"), NewLine,
            // 22:
            NewLine,
            // 23: test-migrate:
            Word("test-migrate"), Colon, NewLine,
            // 24:   jute test-create-db
            Space, Space, Word("jute"), Space, Word("test-create-db"), NewLine,
            // 25:   with NODE_ENV=test:
            Space, Space, With, Space, Word("NODE_ENV"), Equals, Word("test"), Colon,
            NewLine,
            // 26:     pnpm exec migrate
            Space, Space, Space, Space, Word("pnpm"), Space, Word("exec"), Space,
            Word("migrate"), NewLine,
        ];

        assert_eq!(tokens, expected);
    }
}
