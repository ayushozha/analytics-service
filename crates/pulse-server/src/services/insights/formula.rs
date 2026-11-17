//! Series formulas such as `A / B * 100` (PostHog `trendsFilter.formula`).
//!
//! Grammar: expr := term (('+'|'-') term)* ; term := factor (('*'|'/') factor)* ;
//! factor := number | letter | '(' expr ')' | '-' factor.
//! Letters A–Z refer to series by position. Division by zero yields 0.

use super::bad_request;
use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Series(usize),
    Neg(Box<Expr>),
    Bin(Box<Expr>, char, Box<Expr>),
}

const MAX_FORMULA_LEN: usize = 500;

pub fn parse(src: &str, series_count: usize) -> AppResult<Expr> {
    if src.len() > MAX_FORMULA_LEN {
        return Err(bad_request("formula is too long"));
    }
    let tokens: Vec<char> = src.chars().filter(|c| !c.is_whitespace()).collect();
    let mut p = Parser {
        tokens,
        pos: 0,
        series_count,
        depth: 0,
    };
    let e = p.expr()?;
    if p.pos != p.tokens.len() {
        return Err(bad_request(format!(
            "unexpected '{}' in formula",
            p.tokens[p.pos]
        )));
    }
    Ok(e)
}

struct Parser {
    tokens: Vec<char>,
    pos: usize,
    series_count: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.tokens.get(self.pos).copied()
    }

    fn expr(&mut self) -> AppResult<Expr> {
        let mut lhs = self.term()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.pos += 1;
            lhs = Expr::Bin(Box::new(lhs), op, Box::new(self.term()?));
        }
        Ok(lhs)
    }

    fn term(&mut self) -> AppResult<Expr> {
        let mut lhs = self.factor()?;
        while let Some(op @ ('*' | '/')) = self.peek() {
            self.pos += 1;
            lhs = Expr::Bin(Box::new(lhs), op, Box::new(self.factor()?));
        }
        Ok(lhs)
    }

    fn factor(&mut self) -> AppResult<Expr> {
        self.depth += 1;
        if self.depth > 100 {
            return Err(bad_request("formula is nested too deeply"));
        }
        let result = match self.peek() {
            Some('-') => {
                self.pos += 1;
                Ok(Expr::Neg(Box::new(self.factor()?)))
            }
            Some('(') => {
                self.pos += 1;
                let e = self.expr()?;
                if self.peek() != Some(')') {
                    return Err(bad_request("missing ')' in formula"));
                }
                self.pos += 1;
                Ok(e)
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.pos += 1;
                let idx = (c.to_ascii_uppercase() as u8 - b'A') as usize;
                if idx >= self.series_count {
                    return Err(bad_request(format!(
                        "formula references series {c}, but only {} series exist",
                        self.series_count
                    )));
                }
                Ok(Expr::Series(idx))
            }
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let start = self.pos;
                while matches!(self.peek(), Some(d) if d.is_ascii_digit() || d == '.') {
                    self.pos += 1;
                }
                let s: String = self.tokens[start..self.pos].iter().collect();
                s.parse()
                    .map(Expr::Num)
                    .map_err(|_| bad_request(format!("invalid number '{s}' in formula")))
            }
            Some(c) => Err(bad_request(format!("unexpected '{c}' in formula"))),
            None => Err(bad_request("formula ended unexpectedly")),
        };
        self.depth -= 1;
        result
    }
}

/// Evaluate for one bucket, given each series' value at that bucket.
pub fn eval(e: &Expr, values: &[f64]) -> f64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Series(i) => values.get(*i).copied().unwrap_or(0.0),
        Expr::Neg(x) => -eval(x, values),
        Expr::Bin(a, op, b) => {
            let (a, b) = (eval(a, values), eval(b, values));
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                _ => {
                    if b == 0.0 {
                        0.0
                    } else {
                        a / b
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_parens_and_unary() {
        let e = parse("A / B * 100", 2).unwrap();
        assert_eq!(eval(&e, &[1.0, 4.0]), 25.0);
        let e = parse("(a + b) * -2", 2).unwrap();
        assert_eq!(eval(&e, &[1.0, 2.0]), -6.0);
        let e = parse("A - B - 1", 2).unwrap();
        assert_eq!(eval(&e, &[10.0, 3.0]), 6.0);
    }

    #[test]
    fn divide_by_zero_is_zero() {
        assert_eq!(eval(&parse("A/B", 2).unwrap(), &[5.0, 0.0]), 0.0);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("A + C", 2).is_err());
        assert!(parse("A +", 2).is_err());
        assert!(parse("(A", 1).is_err());
        assert!(parse("A $ B", 2).is_err());
        assert!(parse(&"(".repeat(200), 1).is_err());
    }
}
