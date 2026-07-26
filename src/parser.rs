pub struct Parser<'a> {
    input: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, i: 0 }
    }
}
