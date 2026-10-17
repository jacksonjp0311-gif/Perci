//! Exact rational expression evaluator and one-variable linear solver.
//!
//! Handles `3/7 + 2/9`, `2^64`, `(1.5 + 2) * 4`, and `solve 2x + 3 = 11`
//! with checked `i128` rational arithmetic: every result is exact or an
//! error, never a rounded float.

use crate::reasoning::{Rational, ReasoningError};

type Res<T> = Result<T, ReasoningError>;

fn q(n: i128, d: i128) -> Res<Rational> {
    Rational::new(n, d)
}

fn add(a: Rational, b: Rational) -> Res<Rational> {
    let l = a
        .numerator
        .checked_mul(b.denominator)
        .ok_or(ReasoningError::Overflow)?;
    let r = b
        .numerator
        .checked_mul(a.denominator)
        .ok_or(ReasoningError::Overflow)?;
    let d = a
        .denominator
        .checked_mul(b.denominator)
        .ok_or(ReasoningError::Overflow)?;
    q(l.checked_add(r).ok_or(ReasoningError::Overflow)?, d)
}

fn neg(a: Rational) -> Res<Rational> {
    q(
        a.numerator.checked_neg().ok_or(ReasoningError::Overflow)?,
        a.denominator,
    )
}

fn mul(a: Rational, b: Rational) -> Res<Rational> {
    // Cross-reduce first to keep intermediates small.
    let g1 = gcd(a.numerator, b.denominator);
    let g2 = gcd(b.numerator, a.denominator);
    let n = (a.numerator / g1)
        .checked_mul(b.numerator / g2)
        .ok_or(ReasoningError::Overflow)?;
    let d = (a.denominator / g2)
        .checked_mul(b.denominator / g1)
        .ok_or(ReasoningError::Overflow)?;
    q(n, d)
}

fn div(a: Rational, b: Rational) -> Res<Rational> {
    if b.numerator == 0 {
        return Err(ReasoningError::DivisionByZero);
    }
    mul(a, q(b.denominator, b.numerator)?)
}

fn pow(base: Rational, exp: Rational) -> Res<Rational> {
    if exp.denominator != 1 {
        return Err(ReasoningError::Unsupported);
    }
    let e = exp.numerator;
    if e.abs() > 4096 {
        return Err(ReasoningError::Overflow);
    }
    let mut acc = q(1, 1)?;
    for _ in 0..e.abs() {
        acc = mul(acc, base)?;
    }
    if e < 0 {
        div(q(1, 1)?, acc)
    } else {
        Ok(acc)
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    (a.max(1)) as i128
}

/// `a·x + b` (a = 0 for a constant).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lin {
    a: Rational,
    b: Rational,
}

impl Lin {
    fn constant(b: Rational) -> Self {
        Self {
            a: Rational {
                numerator: 0,
                denominator: 1,
            },
            b,
        }
    }
    fn is_constant(&self) -> bool {
        self.a.numerator == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Num(Rational),
    Var,
    Op(char),
    Open,
    Close,
}

fn parse_decimal(s: &str) -> Res<Rational> {
    let bad = || ReasoningError::InvalidNumber(s.to_owned());
    match s.split_once('.') {
        None => q(s.parse().map_err(|_| bad())?, 1),
        Some((int, frac)) => {
            if frac.is_empty() || frac.len() > 18 || frac.contains('.') {
                return Err(bad());
            }
            let scale = 10i128.pow(frac.len() as u32);
            let whole: i128 = if int.is_empty() {
                0
            } else {
                int.parse().map_err(|_| bad())?
            };
            let f: i128 = frac.parse().map_err(|_| bad())?;
            let n = whole
                .checked_mul(scale)
                .and_then(|w| w.checked_add(f))
                .ok_or(ReasoningError::Overflow)?;
            q(n, scale)
        }
    }
}

fn tokenize(src: &str, var: Option<char>) -> Res<Vec<Tok>> {
    let chars: Vec<char> = src.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out: Vec<Tok> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let tok = if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            i -= 1;
            Tok::Num(parse_decimal(&s)?)
        } else if Some(c) == var {
            Tok::Var
        } else {
            match c {
                '+' | '-' | '*' | '/' | '^' => Tok::Op(c),
                '\u{00d7}' | '\u{00b7}' => Tok::Op('*'),
                '\u{00f7}' => Tok::Op('/'),
                '(' => Tok::Open,
                ')' => Tok::Close,
                _ => return Err(ReasoningError::Unsupported),
            }
        };
        // Implicit multiplication: 2x, 2(…), )(, x(, )x, )2
        if let Some(prev) = out.last() {
            let prev_value = matches!(prev, Tok::Num(_) | Tok::Var | Tok::Close);
            let next_value = matches!(tok, Tok::Num(_) | Tok::Var | Tok::Open);
            if prev_value && next_value && !matches!((prev, tok), (Tok::Num(_), Tok::Num(_))) {
                out.push(Tok::Op('*'));
            }
        }
        out.push(tok);
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.pos).copied()
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.peek();
        self.pos += 1;
        t
    }
    fn expr(&mut self) -> Res<Lin> {
        let mut acc = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek() {
            self.pos += 1;
            let rhs = self.term()?;
            let rhs = if op == '-' {
                Lin {
                    a: neg(rhs.a)?,
                    b: neg(rhs.b)?,
                }
            } else {
                rhs
            };
            acc = Lin {
                a: add(acc.a, rhs.a)?,
                b: add(acc.b, rhs.b)?,
            };
        }
        Ok(acc)
    }
    fn term(&mut self) -> Res<Lin> {
        let mut acc = self.unary()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek() {
            self.pos += 1;
            let rhs = self.unary()?;
            acc = if op == '*' {
                if acc.is_constant() {
                    Lin {
                        a: mul(acc.b, rhs.a)?,
                        b: mul(acc.b, rhs.b)?,
                    }
                } else if rhs.is_constant() {
                    Lin {
                        a: mul(acc.a, rhs.b)?,
                        b: mul(acc.b, rhs.b)?,
                    }
                } else {
                    return Err(ReasoningError::Unsupported); // x·x is not linear
                }
            } else {
                if !rhs.is_constant() {
                    return Err(ReasoningError::Unsupported);
                }
                Lin {
                    a: div(acc.a, rhs.b)?,
                    b: div(acc.b, rhs.b)?,
                }
            };
        }
        Ok(acc)
    }
    fn unary(&mut self) -> Res<Lin> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                let v = self.unary()?;
                Ok(Lin {
                    a: neg(v.a)?,
                    b: neg(v.b)?,
                })
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }
    fn power(&mut self) -> Res<Lin> {
        let base = self.primary()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.pos += 1;
            let exp = self.unary()?;
            if !base.is_constant() || !exp.is_constant() {
                return Err(ReasoningError::Unsupported);
            }
            return Ok(Lin::constant(pow(base.b, exp.b)?));
        }
        Ok(base)
    }
    fn primary(&mut self) -> Res<Lin> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Lin::constant(n)),
            Some(Tok::Var) => Ok(Lin {
                a: q(1, 1)?,
                b: q(0, 1)?,
            }),
            Some(Tok::Open) => {
                let v = self.expr()?;
                match self.next() {
                    Some(Tok::Close) => Ok(v),
                    _ => Err(ReasoningError::Unsupported),
                }
            }
            _ => Err(ReasoningError::Unsupported),
        }
    }
}

fn parse_lin(src: &str, var: Option<char>) -> Res<Lin> {
    let toks = tokenize(src, var)?;
    if toks.is_empty() {
        return Err(ReasoningError::Unsupported);
    }
    let mut p = Parser { toks, pos: 0 };
    let v = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(ReasoningError::Unsupported);
    }
    Ok(v)
}

/// True when `src` is only numbers, operators and parentheses, with at least
/// one operator between values.
pub fn looks_like_expression(src: &str) -> bool {
    let s: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    !s.is_empty()
        && s.chars().any(|c| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_digit() || "+-*/^().\u{00d7}\u{00f7}\u{00b7}".contains(c))
        && s.chars()
            .skip(1)
            .any(|c| "+-*/^\u{00d7}\u{00f7}\u{00b7}".contains(c))
}

/// Evaluate a pure numeric expression exactly.
pub fn eval(src: &str) -> Res<Rational> {
    let v = parse_lin(src, None)?;
    Ok(v.b)
}

/// Outcome of [`solve_linear`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Solution {
    One(Rational),
    All,
    None,
}

/// Solve a linear equation `lhs = rhs` in `var`.
pub fn solve_linear(equation: &str, var: char) -> Res<Solution> {
    let (l, r) = equation
        .split_once('=')
        .ok_or(ReasoningError::Unsupported)?;
    if r.contains('=') {
        return Err(ReasoningError::Unsupported);
    }
    let l = parse_lin(l, Some(var))?;
    let r = parse_lin(r, Some(var))?;
    let a = add(l.a, neg(r.a)?)?;
    let b = add(r.b, neg(l.b)?)?;
    if a.numerator == 0 {
        return Ok(if b.numerator == 0 {
            Solution::All
        } else {
            Solution::None
        });
    }
    Ok(Solution::One(div(b, a)?))
}

/// Deterministic primality for `2 ..= 10^12` by trial division; returns the
/// smallest factor for composites.
pub fn smallest_factor(n: u64) -> Option<u64> {
    if n < 4 {
        return None;
    }
    if n % 2 == 0 {
        return Some(2);
    }
    let mut d = 3u64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            return Some(d);
        }
        d += 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    #[test]
    fn evaluates_exact_expressions() {
        assert_eq!(eval("3/7+2/9").unwrap(), r(41, 63));
        assert_eq!(eval("2^64").unwrap(), r(18_446_744_073_709_551_616, 1));
        assert_eq!(eval("(1.5+2)*4").unwrap(), r(14, 1));
        assert_eq!(eval("-2^2").unwrap(), r(-4, 1));
        assert_eq!(eval("2^-2").unwrap(), r(1, 4));
        assert_eq!(eval("10-4-3").unwrap(), r(3, 1));
        assert_eq!(eval("2(3+4)").unwrap(), r(14, 1));
        assert!(matches!(eval("1/0"), Err(ReasoningError::DivisionByZero)));
        assert!(matches!(eval("2^200"), Err(ReasoningError::Overflow)));
        assert!(eval("2+").is_err());
    }

    #[test]
    fn solves_linear_equations() {
        assert_eq!(
            solve_linear("2x+3=11", 'x').unwrap(),
            Solution::One(r(4, 1))
        );
        assert_eq!(
            solve_linear("5y-2=3y+7", 'y').unwrap(),
            Solution::One(r(9, 2))
        );
        assert_eq!(solve_linear("x+1=x+1", 'x').unwrap(), Solution::All);
        assert_eq!(solve_linear("x+1=x+2", 'x').unwrap(), Solution::None);
        assert!(solve_linear("x*x=4", 'x').is_err());
    }

    #[test]
    fn expression_detection() {
        assert!(looks_like_expression("2^64"));
        assert!(looks_like_expression("3/7+2/9"));
        assert!(!looks_like_expression("-5"));
        assert!(!looks_like_expression("v2-beta"));
    }

    #[test]
    fn primality() {
        assert_eq!(smallest_factor(89), None);
        assert_eq!(smallest_factor(91), Some(7));
        assert_eq!(smallest_factor(1_000_000_007), None);
        assert_eq!(smallest_factor(2), None);
    }
}
