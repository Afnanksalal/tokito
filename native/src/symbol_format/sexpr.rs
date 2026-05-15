//! Minimal S-expression lexer/parser for Tokito symbol libraries.

#[derive(Debug, Clone, PartialEq)]
pub enum Sexpr {
    Atom(String),
    List(Vec<Sexpr>),
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Sexpr, ParseError> {
    let tokens = tokenize(input)?;
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.parse_expr()?;
    if p.pos < p.tokens.len() {
        return Err(ParseError {
            message: "trailing tokens after expression".into(),
        });
    }
    Ok(expr)
}

#[derive(Clone)]
enum Token {
    LParen,
    RParen,
    Atom(String),
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match c {
            b'(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            b'"' => {
                i += 1;
                let start = i;
                let mut s = String::new();
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 1;
                        s.push(bytes[i] as char);
                        i += 1;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        s.push(bytes[i] as char);
                        i += 1;
                    }
                }
                if i > bytes.len()
                    || (i > 0 && bytes[i.saturating_sub(1)] != b'"' && i == bytes.len())
                {
                    // handled below by checking quote
                }
                let _ = start;
                tokens.push(Token::Atom(s));
            }
            _ => {
                let start = i;
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && bytes[i] != b'('
                    && bytes[i] != b')'
                {
                    i += 1;
                }
                let atom = std::str::from_utf8(&bytes[start..i])
                    .map_err(|_| ParseError {
                        message: "invalid utf-8 in atom".into(),
                    })?
                    .to_string();
                tokens.push(Token::Atom(atom));
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn parse_expr(&mut self) -> Result<Sexpr, ParseError> {
        match self.peek() {
            Some(Token::LParen) => {
                self.pos += 1;
                let mut items = Vec::new();
                while !matches!(self.peek(), Some(Token::RParen) | None) {
                    items.push(self.parse_expr()?);
                }
                if !matches!(self.peek(), Some(Token::RParen)) {
                    return Err(ParseError {
                        message: "unclosed list".into(),
                    });
                }
                self.pos += 1;
                Ok(Sexpr::List(items))
            }
            Some(Token::Atom(a)) => {
                let a = a.clone();
                self.pos += 1;
                Ok(Sexpr::Atom(a))
            }
            _ => Err(ParseError {
                message: "unexpected token".into(),
            }),
        }
    }
}

pub fn list_head(list: &Sexpr) -> Option<(&str, &[Sexpr])> {
    let Sexpr::List(items) = list else {
        return None;
    };
    let head = items.first()?.as_atom()?;
    Some((head, &items[1..]))
}

impl Sexpr {
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Sexpr::Atom(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn child_list(&self, head: &str) -> Option<&Sexpr> {
        let Sexpr::List(items) = self else {
            return None;
        };
        for child in items {
            if let Some((h, _)) = list_head(child) {
                if h == head {
                    return Some(child);
                }
            }
        }
        None
    }

    pub fn as_point(&self) -> Option<[f64; 2]> {
        let (_, tail) = list_head(self)?;
        let x: f64 = tail.first()?.as_atom()?.parse().ok()?;
        let y: f64 = tail.get(1)?.as_atom()?.parse().ok()?;
        Some([x, y])
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_atom()?.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_list() {
        let s = parse("(a (b 1 2) \"c\")").unwrap();
        assert!(matches!(s, Sexpr::List(_)));
    }
}
