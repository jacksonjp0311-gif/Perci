//! `perci.exact`: compute an exact answer, or check a claimed one.
//!
//! Accepts plain expressions (`3/7 + 2/9`, `2^64`, `(1.5+2)*4`, `1e3 * 2`),
//! everyday phrasing (`15% of 80 plus 3`, `2 to the power of 64`,
//! `sqrt of 144`, `is 97 prime`, `solve 2x + 3 = 11`) and the phrase-based
//! forms in `reasoning.rs` (gcd, lcm, factorial, average, ratio, percent
//! change, circle/rectangle/triangle geometry). Everything is checked `i128`
//! rational arithmetic. Anything it cannot parse is an `abstain`, never a guess.

use super::{arg_str, opt_str, try_arg, with_summary, Ctx, Tool};
use crate::exact_expr::{self, Solution};
use crate::reasoning::{self, Rational, ReasoningError};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};

pub struct Exact;

impl Tool for Exact {
    fn name(&self) -> &'static str {
        "perci.exact"
    }
    fn title(&self) -> &'static str {
        "Exact math"
    }
    fn description(&self) -> &'static str {
        "Compute an exact answer with checked rational arithmetic (fractions, %, powers, roots, primality, one-variable linear equations, gcd/lcm/factorial, basic geometry). Pass `claimed` to check a value: fail if it is wrong. Abstains on anything it cannot parse instead of guessing. ~microseconds."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["expr"],
            "properties": {
                "expr": {"type": "string", "description": "Expression or short math question, e.g. \"17% of 240\", \"3/7 + 2/9\", \"solve 2x+3=11\"."},
                "claimed": {"type": "string", "description": "Optional value to check, e.g. \"41\" or \"40.8\" or \"204/5\"."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let expr = try_arg!(arg_str(args, "expr"));
        let claimed = try_arg!(opt_str(args, "claimed"));
        let computed = compute(expr);
        match claimed {
            None => computed.into_outcome(expr),
            Some(c) => check_claim(expr, &computed, c),
        }
    }
}

/// What `compute` found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Computed {
    /// An exact rational value.
    Value {
        value: Rational,
        normalized: String,
        method: &'static str,
    },
    /// Primality answer.
    Prime {
        n: u64,
        smallest_factor: Option<u64>,
    },
    /// sqrt of a non-square: irrational, decimal is an approximation.
    Irrational { normalized: String, approx: String },
    /// Linear equation outcome.
    Solved {
        var: char,
        solution: Solution,
        normalized: String,
    },
    /// Phrase-based answer from `reasoning.rs` (text, still exact).
    Phrase { answer: String },
    /// Mathematically undefined (division by zero).
    Undefined { normalized: String },
    /// Overflow of the i128 range.
    TooLarge,
    /// Notation with two readings, e.g. `3/4 / 1/2`: abstain, don't pick one.
    Ambiguous {
        normalized: String,
        why: &'static str,
    },
    /// Not understood.
    Unparsed,
}

impl Computed {
    fn into_outcome(self, expr: &str) -> Outcome {
        let ev = Evidence::new("computation", "perci.exact/rational-i128")
            .hashed(crate::doctor::sha256_hex(expr.trim().as_bytes()));
        match self {
            Computed::Value {
                value,
                normalized,
                method,
            } => {
                let (decimal, terminating) = decimal(value, 12);
                Outcome::pass(with_summary(
                    json!({
                        "kind": "value",
                        "value": value.to_string(),
                        "decimal": decimal,
                        "decimal_exact": terminating,
                        "normalized": normalized,
                        "method": method,
                    }),
                    if value.denominator == 1 || !terminating {
                        format!(
                            "{normalized} = {value}{}",
                            if terminating {
                                String::new()
                            } else {
                                format!(" (\u{2248} {decimal})")
                            }
                        )
                    } else {
                        format!("{normalized} = {value} = {decimal}")
                    },
                ))
                .evidence(ev)
            }
            Computed::Prime { n, smallest_factor } => Outcome::pass(with_summary(
                json!({
                    "kind": "primality",
                    "n": n.to_string(),
                    "prime": smallest_factor.is_none() && n >= 2,
                    "smallest_factor": smallest_factor.map(|f| f.to_string()),
                }),
                match smallest_factor {
                    _ if n < 2 => format!("{n} is not prime"),
                    None => format!("{n} is prime"),
                    Some(f) => format!("{n} is not prime ({n} = {f} \u{00d7} {})", n / f),
                },
            ))
            .evidence(ev),
            Computed::Irrational { normalized, approx } => Outcome::pass(with_summary(
                json!({
                    "kind": "irrational",
                    "value": Value::Null,
                    "decimal": approx,
                    "decimal_exact": false,
                    "normalized": normalized,
                }),
                format!("{normalized} is irrational (\u{2248} {approx})"),
            ))
            .evidence(ev),
            Computed::Solved {
                var,
                solution,
                normalized,
            } => {
                let (value, summary) = match &solution {
                    Solution::One(v) => (json!(v.to_string()), format!("{var} = {v}")),
                    Solution::All => (Value::Null, format!("every {var} satisfies {normalized}")),
                    Solution::None => (Value::Null, format!("no {var} satisfies {normalized}")),
                };
                Outcome::pass(with_summary(
                    json!({
                        "kind": "equation",
                        "variable": var.to_string(),
                        "value": value,
                        "solutions": match solution { Solution::One(_) => "one", Solution::All => "all", Solution::None => "none" },
                        "normalized": normalized,
                    }),
                    summary,
                ))
                .evidence(ev)
            }
            Computed::Phrase { answer } => Outcome::pass(with_summary(
                json!({"kind": "phrase", "answer": answer}),
                answer.clone(),
            ))
            .evidence(ev),
            Computed::Undefined { normalized } => Outcome::fail(with_summary(
                json!({"kind": "undefined", "normalized": normalized}),
                format!("{normalized} is undefined"),
            ))
            .reason(Reason::new(
                "DIVISION_BY_ZERO",
                "the expression divides by zero",
            ))
            .evidence(ev),
            Computed::Ambiguous { normalized, why } => {
                Outcome::abstain("AMBIGUOUS", format!("{normalized}: {why}"))
            }
            Computed::TooLarge => Outcome::abstain(
                "OVERFLOW",
                "result exceeds the exact 128-bit range; refusing to round",
            ),
            Computed::Unparsed => Outcome::abstain(
                "UNPARSED",
                format!(
                    "not an expression Perci can evaluate exactly: {:?}",
                    crate::util::text::truncate(expr.trim(), 80)
                ),
            )
            .confidence(Confidence::Exact),
        }
    }
}

/// Lowercase and turn everyday phrasing into an operator expression.
pub fn normalize(expr: &str) -> String {
    let mut s = expr
        .trim()
        .to_lowercase()
        .replace(['\u{00d7}', '\u{00b7}', '\u{2217}'], "*")
        .replace('\u{00f7}', "/")
        .replace(['\u{2212}', '\u{2013}'], "-")
        .replace("**", "^");
    s = s.trim_end_matches(['?', '.', '!', ' ']).to_owned();
    for prefix in [
        "what is the value of ",
        "what's ",
        "whats ",
        "what is ",
        "how much is ",
        "calculate ",
        "compute ",
        "evaluate ",
        "simplify ",
        "work out ",
    ] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_owned();
            break;
        }
    }
    let replacements: &[(&str, &str)] = &[
        (" raised to the power of ", "^"),
        (" to the power of ", "^"),
        (" raised to ", "^"),
        (" to the ", "^"),
        (" squared", "^2"),
        (" cubed", "^3"),
        (" multiplied by ", "*"),
        (" times ", "*"),
        (" divided by ", "/"),
        (" over ", "/"),
        (" plus ", "+"),
        (" minus ", "-"),
        (" percent of ", "/100*"),
        ("% of ", "/100*"),
        (" percent", "/100"),
        ("%", "/100"),
    ];
    for (from, to) in replacements {
        s = s.replace(from, to);
    }
    // "3 x 4" → "3*4" (only between digits).
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        let prev = chars[..i].iter().rev().find(|c| !c.is_whitespace());
        let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
        match c {
            'x' if prev.is_some_and(|p| p.is_ascii_digit() || *p == ')')
                && next.is_some_and(|n| n.is_ascii_digit() || *n == '(') =>
            {
                out.push('*')
            }
            // Scientific notation 1e3 / 2.5e-4 → *10^
            'e' if i > 0
                && chars[i - 1].is_ascii_digit()
                && (chars.get(i + 1).is_some_and(|n| n.is_ascii_digit())
                    || (chars.get(i + 1).is_some_and(|n| *n == '-' || *n == '+')
                        && chars.get(i + 2).is_some_and(|n| n.is_ascii_digit()))) =>
            {
                out.push_str("*10^")
            }
            ',' if prev.is_some_and(|p| p.is_ascii_digit())
                && next.is_some_and(|n| n.is_ascii_digit())
                && chars[i + 1..]
                    .iter()
                    .take_while(|c| c.is_ascii_digit())
                    .count()
                    == 3 =>
            {
                // thousands separator
            }
            '$' | '\u{20ac}' | '\u{00a3}' => {}
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn rational_sqrt(v: Rational) -> Option<Rational> {
    if v.numerator < 0 {
        return None;
    }
    let n = isqrt(v.numerator as u128)?;
    let d = isqrt(v.denominator as u128)?;
    Rational::new(n as i128, d as i128).ok()
}

fn isqrt(n: u128) -> Option<u128> {
    if n < 2 {
        return Some(n);
    }
    let mut x = (n as f64).sqrt() as u128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    (x * x == n).then_some(x)
}

fn map_err(e: ReasoningError, normalized: &str) -> Computed {
    match e {
        ReasoningError::DivisionByZero => Computed::Undefined {
            normalized: normalized.to_owned(),
        },
        ReasoningError::Overflow => Computed::TooLarge,
        _ => Computed::Unparsed,
    }
}

/// Compute `expr` exactly.
pub fn compute(expr: &str) -> Computed {
    let n = normalize(expr);
    let compact: String = n.chars().filter(|c| !c.is_whitespace()).collect();

    // Primality: "is 97 prime", "is 1001 a prime number", "97 prime?"
    if n.contains("prime") {
        let words: Vec<&str> = n.split_whitespace().collect();
        let nums: Vec<u64> = words
            .iter()
            .filter_map(|w| w.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
            .collect();
        let only_number_words = words.iter().all(|w| {
            w.chars().all(|c| c.is_ascii_digit())
                || ["is", "a", "prime", "number", "check", "if", "whether"].contains(w)
        });
        if nums.len() == 1 && only_number_words {
            let v = nums[0];
            if v > 1_000_000_000_000 {
                return Computed::TooLarge;
            }
            return Computed::Prime {
                n: v,
                smallest_factor: if v < 2 {
                    None
                } else {
                    exact_expr::smallest_factor(v)
                },
            };
        }
        return Computed::Unparsed;
    }

    // sqrt: "sqrt of 144", "square root of 9/4", "sqrt(2)", "√2"
    for head in [
        "the square root of ",
        "square root of ",
        "sqrt of ",
        "sqrt",
        "\u{221a}",
    ] {
        if let Some(rest) = n.strip_prefix(head) {
            let inner = rest.trim().trim_start_matches('(').trim_end_matches(')');
            if !exact_expr_ok(inner) {
                return Computed::Unparsed;
            }
            return match exact_expr::eval(&inner.replace(' ', "")) {
                Ok(v) if v.numerator < 0 => Computed::Unparsed,
                Ok(v) => match rational_sqrt(v) {
                    Some(root) => Computed::Value {
                        value: root,
                        normalized: format!("sqrt({v})"),
                        method: "isqrt",
                    },
                    None => Computed::Irrational {
                        normalized: format!("sqrt({v})"),
                        approx: format!(
                            "{:.12}",
                            (v.numerator as f64 / v.denominator as f64).sqrt()
                        )
                        .trim_end_matches('0')
                        .to_owned(),
                    },
                },
                Err(e) => map_err(e, &n),
            };
        }
    }

    // One-variable linear equation.
    if compact.contains('=') {
        let body = compact.strip_prefix("solve").unwrap_or(&compact);
        let body = body.split_once("for").map(|(l, _)| l).unwrap_or(body);
        let mut letters: Vec<char> = body.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        letters.sort_unstable();
        letters.dedup();
        if letters.len() == 1 {
            let var = letters[0];
            return match exact_expr::solve_linear(body, var) {
                Ok(solution) => Computed::Solved {
                    var,
                    solution,
                    normalized: body.to_owned(),
                },
                Err(e) => map_err(e, body),
            };
        }
        if letters.is_empty() {
            // "2+2=4" given to exact: evaluate the left side.
            if let Some((l, _)) = body.split_once('=') {
                return compute(l);
            }
        }
        return Computed::Unparsed;
    }

    // Plain expression (including a lone number).
    if exact_expr_ok(&compact) {
        if chained_division(&compact) {
            return Computed::Ambiguous {
                normalized: compact,
                why: "two divisions in one product (a/b/c or a/b / c/d) read differently by people and by precedence rules; add parentheses",
            };
        }
        return match exact_expr::eval(&compact) {
            Ok(value) => Computed::Value {
                value,
                normalized: compact,
                method: "expression",
            },
            Err(e) => map_err(e, &compact),
        };
    }

    // Phrase forms from reasoning.rs (gcd, lcm, factorial, average, geometry, …).
    match reasoning::try_solve_arithmetic(expr) {
        Ok(Some(answer)) => return Computed::Phrase { answer },
        Err(ReasoningError::DivisionByZero) => return Computed::Undefined { normalized: n },
        Err(ReasoningError::Overflow) => return Computed::TooLarge,
        _ => {}
    }
    match reasoning::try_solve_geometry(expr) {
        Ok(Some(answer)) => Computed::Phrase { answer },
        _ => Computed::Unparsed,
    }
}

/// True when two `/` operators follow each other in one product chain
/// (same parenthesis depth, no `+`, `-` or `*` between them): `a/b/c`,
/// `a/b / c/d`. `a/b * c/d` reads the same under every convention.
pub fn chained_division(s: &str) -> bool {
    let mut stack = vec![0u32];
    let mut prev: Option<char> = None;
    for c in s.chars() {
        match c {
            '(' => stack.push(0),
            ')' => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            '+' | '-' if prev.is_some_and(|p| p.is_ascii_digit() || p == ')' || p == '.') => {
                if let Some(top) = stack.last_mut() {
                    *top = 0;
                }
            }
            '*' => {
                if let Some(top) = stack.last_mut() {
                    *top = 0;
                }
            }
            '/' => {
                if let Some(top) = stack.last_mut() {
                    *top += 1;
                    if *top >= 2 {
                        return true;
                    }
                }
            }
            _ => {}
        }
        prev = Some(c);
    }
    false
}

fn exact_expr_ok(s: &str) -> bool {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    !s.is_empty()
        && s.chars().any(|c| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_digit() || "+-*/^().".contains(c))
}

/// Exact decimal for terminating values; otherwise rounded to `places`.
/// Returns `(text, terminating)`.
pub fn decimal(v: Rational, places: usize) -> (String, bool) {
    let neg = v.numerator < 0;
    let num = v.numerator.unsigned_abs();
    let den = v.denominator.unsigned_abs();
    let int = num / den;
    let mut rem = num % den;
    let mut digits = Vec::new();
    while rem != 0 && digits.len() <= places {
        rem *= 10;
        digits.push((rem / den) as u8);
        rem %= den;
    }
    let terminating = rem == 0 && digits.len() <= places;
    let mut int_part = int;
    if !terminating {
        // Round half up on the extra digit.
        let extra = digits.pop().unwrap_or(0);
        if extra >= 5 {
            let mut i = digits.len();
            loop {
                if i == 0 {
                    int_part += 1;
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
        while digits.last() == Some(&0) {
            digits.pop();
        }
    }
    let mut s = String::new();
    if neg && (int_part != 0 || !digits.is_empty()) {
        s.push('-');
    }
    s.push_str(&int_part.to_string());
    if !digits.is_empty() {
        s.push('.');
        s.extend(digits.iter().map(|d| char::from(b'0' + d)));
    }
    (s, terminating)
}

/// A number someone claimed, as text. Handles `41`, `40.8`, `204/5`,
/// `$1,234.50`, `-3`, `1e3`, `≈ 0.33`.
#[derive(Clone, Debug)]
pub struct Claimed {
    pub value: Rational,
    /// Digits after the decimal point as written (0 for integers/fractions).
    pub places: u32,
    pub approximate: bool,
    pub text: String,
}

pub fn parse_claimed(text: &str) -> Option<Claimed> {
    let raw = text.trim();
    let lower = raw.to_lowercase();
    let mut approximate = false;
    let mut s = lower.as_str();
    for p in [
        "approximately ",
        "approx. ",
        "approx ",
        "about ",
        "around ",
        "roughly ",
        "~",
        "\u{2248}",
        "exactly ",
    ] {
        if let Some(rest) = s.strip_prefix(p) {
            approximate = !p.starts_with("exactly");
            s = rest.trim_start();
        }
    }
    let cleaned: String = normalize(s)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let cleaned = cleaned.trim_end_matches(['.', ',', ';', ':']);
    if cleaned.is_empty() || !exact_expr_ok(cleaned) {
        return None;
    }
    let places = cleaned
        .split_once('.')
        .map(|(_, f)| f.chars().take_while(|c| c.is_ascii_digit()).count() as u32)
        .unwrap_or(0);
    let value = exact_expr::eval(cleaned).ok()?;
    Some(Claimed {
        value,
        places,
        approximate,
        text: raw.to_owned(),
    })
}

/// How a claimed number compares to the exact value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Exact,
    /// Correctly rounded to the digits written.
    Rounded,
    Wrong,
    /// Comparison overflowed; can't tell.
    Unknown,
}

/// Rounding is accepted only when the writer clearly rounded: at least two
/// decimals for a terminating value, at least one for a repeating one, or
/// an explicit "about/≈". `41` for an exact `40.8` is wrong, not rounded.
pub fn compare(exact: Rational, claimed: &Claimed) -> Match {
    if exact == claimed.value {
        return Match::Exact;
    }
    let (_, terminating) = decimal(exact, 30);
    let allow = claimed.approximate
        || (terminating && claimed.places >= 2)
        || (!terminating && claimed.places >= 1);
    if !allow {
        return Match::Wrong;
    }
    // |exact - claimed| * 2 * 10^places <= 1  ⇔  within half a unit of the last digit
    let Some(scale) = 10i128.checked_pow(claimed.places) else {
        return Match::Unknown;
    };
    let diff = (|| {
        let l = exact.numerator.checked_mul(claimed.value.denominator)?;
        let r = claimed.value.numerator.checked_mul(exact.denominator)?;
        let n = l.checked_sub(r)?.checked_abs()?;
        let d = exact.denominator.checked_mul(claimed.value.denominator)?;
        Some((n.checked_mul(2)?.checked_mul(scale)?, d))
    })();
    match diff {
        Some((n, d)) if n <= d => Match::Rounded,
        Some(_) => Match::Wrong,
        None => Match::Unknown,
    }
}

/// The exact value `compute` produced, if it is a single number.
pub fn exact_value(c: &Computed) -> Option<Rational> {
    match c {
        Computed::Value { value, .. } => Some(*value),
        Computed::Solved {
            solution: Solution::One(v),
            ..
        } => Some(*v),
        Computed::Phrase { answer } => {
            // "gcd = 6", "12! = 479001600", "average = 7/2", plain "204/5"
            let tail = answer.rsplit('=').next()?.trim();
            parse_claimed(tail)
                .filter(|_| !tail.contains(' '))
                .map(|c| c.value)
        }
        _ => None,
    }
}

fn check_claim(expr: &str, computed: &Computed, claimed_text: &str) -> Outcome {
    if let Computed::Prime { n, smallest_factor } = computed {
        let said = claimed_text.trim().to_lowercase();
        let truth = smallest_factor.is_none() && *n >= 2;
        let said_bool = match said.as_str() {
            "yes" | "true" | "prime" => Some(true),
            "no" | "false" | "composite" | "not prime" => Some(false),
            _ => None,
        };
        let base = computed.clone().into_outcome(expr);
        return match said_bool {
            None => Outcome::abstain(
                "CLAIM_UNPARSED",
                format!("claimed {claimed_text:?} is not yes/no"),
            ),
            Some(b) if b == truth => base,
            Some(_) => Outcome {
                status: Status::Fail,
                ..base
            }
            .reason(Reason::new(
                "PRIMALITY_MISMATCH",
                format!(
                    "claimed {claimed_text}, but {n} is {}prime",
                    if truth { "" } else { "not " }
                ),
            )),
        };
    }
    let Some(exact) = exact_value(computed) else {
        let mut o = computed.clone().into_outcome(expr);
        if o.status == Status::Pass {
            o = Outcome {
                status: Status::Abstain,
                ..o
            }
            .reason(Reason::new(
                "NOT_A_SINGLE_VALUE",
                "result is not one exact number, so the claim can't be compared",
            ));
        }
        return o;
    };
    let Some(claimed) = parse_claimed(claimed_text) else {
        return Outcome::abstain(
            "CLAIM_UNPARSED",
            format!("claimed {claimed_text:?} is not a number"),
        );
    };
    let base = computed.clone().into_outcome(expr);
    let (dec, _) = decimal(exact, 12);
    let mut result = base.result.clone();
    if let Some(obj) = result.as_object_mut() {
        obj.insert("claimed".into(), json!(claimed.text));
    }
    match compare(exact, &claimed) {
        Match::Exact => Outcome { result, ..base },
        Match::Rounded => Outcome { result, ..base }.reason(Reason::new(
            "ROUNDED",
            format!(
                "claimed {} is {exact} ({dec}) correctly rounded",
                claimed.text
            ),
        )),
        Match::Wrong => Outcome {
            status: Status::Fail,
            result,
            ..base
        }
        .reason(Reason::new(
            "EXACT_MISMATCH",
            format!("claimed {}, exact {exact} ({dec})", claimed.text),
        )),
        Match::Unknown => Outcome::abstain("OVERFLOW", "comparison exceeds the exact range"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(expr: &str) -> String {
        match compute(expr) {
            Computed::Value { value, .. } => value.to_string(),
            Computed::Solved {
                solution: Solution::One(x),
                ..
            } => x.to_string(),
            Computed::Prime {
                smallest_factor, ..
            } => format!("prime={}", smallest_factor.is_none()),
            Computed::Phrase { answer } => answer,
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn plan_probes_now_compute() {
        assert_eq!(v("what is 3/7 plus 2/9"), "41/63");
        assert_eq!(v("2 to the power of 64"), "18446744073709551616");
        assert_eq!(v("15% of 80 plus 3"), "15");
        assert_eq!(v("17% of 240"), "204/5");
        assert_eq!(v("calculate 17 percent of 240"), "204/5");
        assert_eq!(v("is 97 prime"), "prime=true");
        assert_eq!(v("is 91 a prime number?"), "prime=false");
        assert_eq!(v("solve 2x + 3 = 11"), "4");
        assert_eq!(v("sqrt of 144"), "12");
        assert_eq!(v("square root of 9/4"), "3/2");
        assert_eq!(v("1e3 times 2"), "2000");
        assert_eq!(v("$1,250 * 3"), "3750");
        assert_eq!(v("3 x 4"), "12");
        assert_eq!(v("12 factorial"), "12! = 479001600");
        assert_eq!(v("gcd of 12 and 18"), "gcd = 6");
    }

    #[test]
    fn irrational_undefined_and_unparsed() {
        assert!(matches!(compute("sqrt 2"), Computed::Irrational { .. }));
        assert!(matches!(compute("1/0"), Computed::Undefined { .. }));
        assert!(matches!(compute("2^200"), Computed::TooLarge));
        assert!(matches!(
            compute("explain rust lifetimes"),
            Computed::Unparsed
        ));
        assert!(matches!(compute("what is love"), Computed::Unparsed));
        // Two readings: abstain instead of guessing (held-out seed 20261017 found these).
        assert!(matches!(
            compute("15/24 / 16/13"),
            Computed::Ambiguous { .. }
        ));
        assert!(matches!(compute("1/2/3"), Computed::Ambiguous { .. }));
        assert!(!chained_division("19/28*30/17"));
        assert!(!chained_division("2^3/4"));
        assert_eq!(v("(15/24) / (16/13)"), "65/128");
        assert_eq!(v("3/7 + 2/9 - 1/3"), "20/63");
        // B2E-PERCI-014: word operator between fractions.
        assert_eq!(v("3/7 plus 2/9"), "41/63");
    }

    #[test]
    fn decimals() {
        let r = |n, d| Rational::new(n, d).unwrap();
        assert_eq!(decimal(r(204, 5), 12), ("40.8".into(), true));
        assert_eq!(decimal(r(1, 3), 4), ("0.3333".into(), false));
        assert_eq!(decimal(r(2, 3), 4), ("0.6667".into(), false));
        assert_eq!(decimal(r(-1, 8), 12), ("-0.125".into(), true));
        assert_eq!(decimal(r(999_999, 1_000_000), 3), ("1".into(), false));
    }

    #[test]
    fn claim_comparison_is_strict_about_unrounded_answers() {
        let exact = Rational::new(204, 5).unwrap();
        let c = |s| parse_claimed(s).unwrap();
        assert_eq!(compare(exact, &c("40.8")), Match::Exact);
        assert_eq!(compare(exact, &c("204/5")), Match::Exact);
        assert_eq!(compare(exact, &c("41")), Match::Wrong);
        assert_eq!(compare(exact, &c("about 41")), Match::Rounded);
        let third = Rational::new(1, 3).unwrap();
        assert_eq!(compare(third, &c("0.33")), Match::Rounded);
        assert_eq!(compare(third, &c("0.34")), Match::Wrong);
        assert_eq!(
            compare(Rational::new(1, 16).unwrap(), &c("0.06")),
            Match::Rounded
        );
        assert_eq!(
            compare(Rational::new(1, 16).unwrap(), &c("0.1")),
            Match::Wrong
        );
        assert_eq!(
            compare(Rational::new(3750, 1).unwrap(), &c("$3,750")),
            Match::Exact
        );
    }

    #[test]
    fn tool_verdicts() {
        let ctx = Ctx::new();
        let run = |a: Value| super::super::run(&ctx, &Exact, &a);
        let ok = run(json!({"expr": "17% of 240"}));
        assert_eq!(ok.status, Status::Pass);
        assert_eq!(ok.result["value"], "204/5");
        assert_eq!(ok.result["decimal"], "40.8");
        let bad = run(json!({"expr": "17% of 240", "claimed": "41"}));
        assert_eq!(bad.status, Status::Fail);
        assert_eq!(bad.reasons[0].code, "EXACT_MISMATCH");
        assert!(bad.reasons[0].detail.contains("204/5"));
        assert_eq!(
            run(json!({"expr": "is 97 prime", "claimed": "no"})).status,
            Status::Fail
        );
        assert_eq!(
            run(json!({"expr": "tell me a story"})).status,
            Status::Abstain
        );
        assert_eq!(run(json!({})).status, Status::Error);
    }
}
