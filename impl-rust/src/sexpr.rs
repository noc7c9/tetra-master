#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Token<'a> {
    ListStart,
    ListEnd,
    Atom(&'a str),
}

#[derive(Debug, Clone)]
pub(crate) struct Sexpr<'a> {
    input: &'a str,
}

impl<'a> Sexpr<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Sexpr { input }
    }

    pub(crate) fn list_start(&mut self) {
        let next = self.next();
        if !matches!(next, Some(Token::ListStart)) {
            panic!("Expected ListStart, got: {next:?}")
        }
    }

    pub(crate) fn list_end(&mut self) {
        let next = self.next();
        if !matches!(next, Some(Token::ListEnd)) {
            panic!("Expected ListEnd, got: {next:?}")
        }
    }

    pub(crate) fn atom(&mut self) -> &str {
        let next = self.next();
        if let Some(Token::Atom(atom)) = next {
            atom
        } else {
            panic!("Expected Atom, got: {next:?}")
        }
    }
}

impl<'a> Iterator for Sexpr<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.input.is_empty() {
                return None;
            }

            // note: we don't need to support anything but ascii, so indexing like this is fine
            match &self.input[0..1] {
                "(" => {
                    self.input = &self.input[1..];
                    return Some(Token::ListStart);
                }
                ")" => {
                    self.input = &self.input[1..];
                    return Some(Token::ListEnd);
                }
                " " => {
                    self.input = &self.input[1..];
                }
                _ => {
                    let mut end = 1;
                    while end < self.input.len()
                        && ![" ", "(", ")"].contains(&&self.input[end..end + 1])
                    {
                        end += 1;
                    }
                    let token = Token::Atom(&self.input[..end]);
                    self.input = &self.input[end..];
                    return Some(token);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token::*;
    use super::*;

    // helper to compare with a Vec<Token> without collecting to avoid possible infinite loops
    fn assert_eq_tokens(input: &str, expected: Vec<Token<'_>>) {
        let mut lexer = Sexpr::new(input);
        for expected in expected {
            assert_eq!(lexer.next(), Some(expected));
        }
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn parse_empty_input() {
        assert_eq_tokens("", vec![])
    }

    #[test]
    fn parse_empty_list() {
        assert_eq_tokens("()", vec![ListStart, ListEnd]);
    }

    #[test]
    fn parse_bare_atoms() {
        assert_eq_tokens("abc", vec![Atom("abc")]);
        assert_eq_tokens("123", vec![Atom("123")]);
        assert_eq_tokens("a-2-c", vec![Atom("a-2-c")]);
        assert_eq_tokens("--abc", vec![Atom("--abc")]);
        assert_eq_tokens("abc--", vec![Atom("abc--")]);
    }

    #[test]
    fn parse_list_of_atoms() {
        assert_eq_tokens(
            "(abc 123 a-b-c)",
            vec![ListStart, Atom("abc"), Atom("123"), Atom("a-b-c"), ListEnd],
        );
    }

    #[test]
    fn parse_nested_atoms() {
        assert_eq_tokens(
            "((abc) ((123) a-b-c))",
            vec![
                ListStart,
                ListStart,
                Atom("abc"),
                ListEnd,
                ListStart,
                ListStart,
                Atom("123"),
                ListEnd,
                Atom("a-b-c"),
                ListEnd,
                ListEnd,
            ],
        );
    }
}
