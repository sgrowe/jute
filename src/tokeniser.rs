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

    pub fn handle_char(&mut self, c: char, index: usize) -> Token<'a> {
        match c {
            '\n' | '\r' => Token::NewLine,
            ' ' => Token::Space,
            ':' => Token::Colon,
            '=' => Token::Equals,
            _ => {
                let mut end_index: Option<usize> = self.cheeky_peek().map(|(i, _)| i);

                while self
                    .cheeky_peek()
                    .map(|(_, c)| !c.is_whitespace() && c != ':' && c != '=')
                    .unwrap_or_default()
                {
                    let Some(_) = self.chars.next() else {
                        break;
                    };

                    end_index = self.cheeky_peek().map(|(i, _)| i);
                }

                let word = match end_index {
                    Some(end) => &self.source[index..end],
                    None => &self.source[index..],
                };

                match word {
                    "with" => Token::With,
                    "in" => Token::In,
                    w => Token::Word(w),
                }
            }
        }
    }
}
