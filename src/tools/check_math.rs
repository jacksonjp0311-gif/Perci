//! `perci.check_math`: find the arithmetic claims in a piece of text (an LLM
//! answer, a report, a commit message) and recompute each one exactly.
//!
//! A claim is `<math> = <number>` or `<math> is|equals|gives|comes to <number>`
//! inside one sentence, where `<math>` is numbers joined by operators or
//! operator words (`17% of 240`, `3/7 + 2/9`, `the square root of 144`).
//! Prose numbers with no operator ("the answer is 42") are not claims Perci
//! can check, so they are not counted. Anything ambiguous is skipped rather
//! than guessed: a wrong `fail` on a correct answer is the worst outcome.

use super::exact::{self, Computed, Match};
use super::{arg_str, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};

pub struct CheckMath;

impl Tool for CheckMath {
    fn name(&self) -> &'static str {
        "perci.check_math"
    }
    fn title(&self) -> &'static str {
        "Check the math in a text"
    }
    fn description(&self) -> &'static str {
        "Extract arithmetic claims (\"17% of 240 is 41\", \"3/7 + 2/9 = 41/63\") from text and recompute each exactly. fail = at least one claim is wrong (with the exact value), pass = every claim found is right, abstain = no checkable claim. Use on any model answer that contains numbers. Sub-millisecond."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string", "description": "The text to check (max 32 KiB)."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let text = try_arg!(arg_str(args, "text"));
        check(text)
    }
}

/// One extracted claim.
#[derive(Clone, Debug)]
pub struct Claim {
    pub span: [usize; 2],
    pub lhs: String,
    pub claimed: String,
    pub status: Status,
    pub exact: Option<String>,
    pub decimal: Option<String>,
    pub note: String,
}

const MAX_CLAIMS: usize = 64;

/// Anchor phrases, longest first so `is equal to` wins over `is`.
const ANCHORS: &[(&str, bool)] = &[
    (" is equal to ", false),
    (" comes out to ", false),
    (" works out to ", false),
    (" is roughly ", true),
    (" is about ", true),
    (" is approximately ", true),
    (" comes to ", false),
    (" equals ", false),
    (" gives ", false),
    (" yields ", false),
    (" totals ", false),
    (" makes ", false),
    (" is ", false),
    ("\u{2248}", true),
    ("=", false),
];

const MATH_WORDS: &[&str] = &[
    "plus",
    "minus",
    "times",
    "multiplied",
    "divided",
    "by",
    "over",
    "of",
    "percent",
    "to",
    "the",
    "power",
    "raised",
    "squared",
    "cubed",
    "sqrt",
    "square",
    "root",
    "x",
];

const OPERATOR_WORDS: &[&str] = &[
    "plus",
    "minus",
    "times",
    "multiplied",
    "divided",
    "over",
    "percent",
    "power",
    "squared",
    "cubed",
    "sqrt",
    "root",
];

fn is_numberish(w: &str) -> bool {
    let w = w.trim_matches(|c: char| c == ',' || c == ';' || c == ':');
    !w.is_empty()
        && w.chars().any(|c| c.is_ascii_digit())
        && w.chars().all(|c| {
            c.is_ascii_digit()
                || "+-*/^().,%$\u{00d7}\u{00f7}\u{2212}\u{20ac}\u{00a3}\u{221a}".contains(c)
        })
}

fn is_operator_token(w: &str) -> bool {
    !w.is_empty()
        && w.chars()
            .all(|c| "+-*/^()\u{00d7}\u{00f7}\u{2212}\u{00b7}".contains(c))
}

fn is_mathish(w: &str) -> bool {
    let lw = w.to_lowercase();
    let lw = lw.trim_matches(|c: char| c == ',' || c == '(' || c == ')');
    is_numberish(w) || is_operator_token(w) || MATH_WORDS.contains(&lw)
}

fn sentence_start(text: &str, upto: usize) -> usize {
    let head = &text[..upto];
    let mut start = 0;
    for (i, c) in head.char_indices() {
        let next = head[i + c.len_utf8()..].chars().next();
        let boundary = match c {
            '\n' | '!' | '?' | ';' => true,
            '.' | ':' => next.is_none_or_space(),
            _ => false,
        };
        if boundary {
            start = i + c.len_utf8();
        }
    }
    start
}

trait NoneOrSpace {
    fn is_none_or_space(&self) -> bool;
}
impl NoneOrSpace for Option<char> {
    fn is_none_or_space(&self) -> bool {
        self.map_or(true, char::is_whitespace)
    }
}

/// The math phrase that ends right before `end` (byte index), if any.
fn lhs_before(text: &str, end: usize) -> Option<(usize, String)> {
    let start = sentence_start(text, end);
    let region = &text[start..end];
    // Word boundaries with byte offsets.
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut i = 0;
    for part in region.split_whitespace() {
        let off = region[i..].find(part).map(|o| o + i).unwrap_or(i);
        words.push((start + off, part));
        i = off + part.len();
    }
    let mut first = words.len();
    while first > 0 && is_mathish(words[first - 1].1) {
        first -= 1;
    }
    let mut picked = &words[first..];
    // Drop leading connective words that are not part of an expression.
    while let Some((_, w)) = picked.first() {
        let lw = w.to_lowercase();
        if is_numberish(w)
            || is_operator_token(w)
            || ["the", "sqrt", "square", "("].contains(&lw.as_str())
            || lw.starts_with('(')
        {
            break;
        }
        picked = &picked[1..];
    }
    if picked.is_empty() {
        return None;
    }
    let phrase: Vec<&str> = picked.iter().map(|(_, w)| *w).collect();
    let joined = phrase.join(" ");
    let lower = joined.to_lowercase();
    let has_digit = joined.chars().any(|c| c.is_ascii_digit());
    let has_operator = phrase.iter().any(|w| {
        let lw = w.to_lowercase();
        OPERATOR_WORDS.contains(&lw.trim_matches(',')) || w.contains('%')
    }) || joined
        .chars()
        .skip(1)
        .any(|c| "+*/^\u{00d7}\u{00f7}\u{221a}".contains(c))
        || joined.contains(" - ")
        || joined.contains(" \u{2212} ")
        || lower.starts_with("the square root")
        || lower.starts_with("square root");
    // A single token like "2024" or "v1.2" is not an expression.
    if !has_digit || !has_operator {
        return None;
    }
    Some((picked[0].0, joined.trim_end_matches([',', ':']).to_owned()))
}

/// The claimed number right after an anchor ending at `from`.
fn rhs_after(text: &str, from: usize) -> Option<(usize, String, bool)> {
    let rest = &text[from..];
    let mut t = rest.trim_start();
    let mut approx = false;
    'strip: loop {
        let lower = t.to_lowercase();
        for p in [
            "approximately ",
            "approx. ",
            "about ",
            "around ",
            "roughly ",
            "exactly ",
            "~",
            "\u{2248}",
        ] {
            if lower.starts_with(p) && t.is_char_boundary(p.len()) {
                approx |= !p.starts_with("exactly");
                t = t[p.len()..].trim_start();
                continue 'strip;
            }
        }
        break;
    }
    // `t` is a suffix of `rest`, so its offset falls out of the lengths.
    let pos = from + (rest.len() - t.len());
    let token_len = t
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit() || ".,/%$-\u{2212}\u{20ac}\u{00a3}".contains(*c))
        .map(|(i, c)| i + c.len_utf8())
        .last()?;
    let token = t[..token_len].trim_end_matches(['.', ',', '/', '-']);
    if !token.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // "= 5 + 1" or "is 5 times …": the right side is itself an expression; skip.
    let after = t[token.len()..].trim_start();
    let next_word = after.split_whitespace().next().unwrap_or("").to_lowercase();
    if after.starts_with(['+', '*', '/', '^', '\u{00d7}', '\u{00f7}'])
        || (after.starts_with('-')
            && after[1..].starts_with(|c: char| c.is_ascii_digit() || c == ' '))
        || ["plus", "minus", "times", "divided", "multiplied", "over"].contains(&next_word.as_str())
        || after.starts_with(|c: char| c.is_ascii_alphabetic())
            && next_word.len() == 1
            && next_word != "a"
    {
        return None;
    }
    // A trailing "/" fraction like "41/63" is kept by the char filter above.
    Some((pos + token.len(), token.to_owned(), approx))
}

/// Extract and check every claim in `text`.
pub fn extract(text: &str) -> Vec<Claim> {
    let lower = text.to_lowercase();
    // Lowercasing can change byte lengths for some scripts; only use the
    // lowered copy for searching when the lengths agree.
    let hay = if lower.len() == text.len() {
        lower.as_str()
    } else {
        text
    };
    let mut taken: Vec<[usize; 2]> = Vec::new();
    let mut claims = Vec::new();
    for (anchor, approx_anchor) in ANCHORS {
        let mut from = 0;
        while let Some(off) = hay[from..].find(anchor) {
            let a_start = from + off;
            let a_end = a_start + anchor.len();
            from = a_end;
            if taken.iter().any(|r| a_start < r[1] && a_end > r[0]) {
                continue;
            }
            if *anchor == "=" {
                let prev = text[..a_start].chars().last();
                let next = text[a_end..].chars().next();
                if matches!(prev, Some('=' | '<' | '>' | '!')) || next == Some('=') {
                    continue;
                }
            }
            let Some((l_start, lhs)) = lhs_before(text, a_start) else {
                continue;
            };
            let Some((r_end, claimed, approx_word)) = rhs_after(text, a_end) else {
                continue;
            };
            taken.push([a_start, a_end]);
            claims.push(judge(
                [l_start, r_end],
                &lhs,
                &claimed,
                *approx_anchor || approx_word,
            ));
            if claims.len() >= MAX_CLAIMS {
                break;
            }
        }
    }
    claims.sort_by_key(|c| c.span[0]);
    claims
}

fn judge(span: [usize; 2], lhs: &str, claimed_text: &str, approx: bool) -> Claim {
    let mut claim = Claim {
        span,
        lhs: lhs.to_owned(),
        claimed: claimed_text.to_owned(),
        status: Status::Abstain,
        exact: None,
        decimal: None,
        note: String::new(),
    };
    let computed = exact::compute(lhs);
    let Some(value) = exact::exact_value(&computed) else {
        claim.note = match computed {
            Computed::Undefined { .. } => "left side divides by zero".into(),
            Computed::Irrational { .. } => "irrational value; not compared".into(),
            Computed::TooLarge => "exceeds exact range".into(),
            Computed::Ambiguous { why, .. } => format!("ambiguous: {why}"),
            _ => "left side not understood".into(),
        };
        return claim;
    };
    let Some(mut claimed) = exact::parse_claimed(claimed_text) else {
        claim.note = "claimed value not understood".into();
        return claim;
    };
    claimed.approximate |= approx;
    let (dec, _) = exact::decimal(value, 12);
    claim.exact = Some(value.to_string());
    claim.decimal = Some(dec.clone());
    match exact::compare(value, &claimed) {
        Match::Exact => claim.status = Status::Pass,
        Match::Rounded => {
            claim.status = Status::Pass;
            claim.note = "correctly rounded".into();
        }
        Match::Wrong => {
            claim.status = Status::Fail;
            claim.note = format!("claimed {claimed_text}, exact {value} ({dec})");
        }
        Match::Unknown => claim.note = "comparison exceeds exact range".into(),
    }
    claim
}

/// Run the checker and build the outcome.
pub fn check(text: &str) -> Outcome {
    let claims = extract(text);
    let failed: Vec<&Claim> = claims.iter().filter(|c| c.status == Status::Fail).collect();
    let passed = claims.iter().filter(|c| c.status == Status::Pass).count();
    let abstained = claims.len() - failed.len() - passed;
    let status = if !failed.is_empty() {
        Status::Fail
    } else if passed > 0 {
        Status::Pass
    } else {
        Status::Abstain
    };
    let summary = match status {
        Status::Fail => format!(
            "{} of {} math claim(s) wrong: {}",
            failed.len(),
            claims.len(),
            failed
                .iter()
                .map(|c| format!(
                    "{} → {} (not {})",
                    c.lhs,
                    c.exact.as_deref().unwrap_or("?"),
                    c.claimed
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Status::Pass => format!("{passed} math claim(s) checked, all correct"),
        _ => "no checkable math claim found".to_owned(),
    };
    let result = with_summary(
        json!({
            "claims": claims.iter().map(|c| json!({
                "span": c.span,
                "expr": c.lhs,
                "claimed": c.claimed,
                "exact": c.exact,
                "decimal": c.decimal,
                "verdict": c.status.as_str(),
                "note": c.note,
            })).collect::<Vec<_>>(),
            "checked": passed + failed.len(),
            "failed": failed.len(),
            "abstained": abstained,
        }),
        summary,
    );
    let mut outcome = Outcome::new(status, result).confidence(Confidence::Exact);
    for c in &claims {
        if c.status == Status::Fail {
            outcome = outcome.reason(
                Reason::new("MATH_MISMATCH", format!("{}: {}", c.lhs, c.note))
                    .at(c.span[0], c.span[1]),
            );
        }
    }
    if claims.is_empty() {
        outcome = outcome.reason(Reason::new(
            "NO_CLAIMS",
            "no arithmetic claim with an operator was found",
        ));
    }
    outcome.evidence(
        Evidence::new("computation", "perci.check_math/rational-i128")
            .hashed(crate::doctor::sha256_hex(text.as_bytes())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdicts(text: &str) -> Vec<(String, String, &'static str)> {
        extract(text)
            .into_iter()
            .map(|c| (c.lhs, c.claimed, c.status.as_str()))
            .collect()
    }

    #[test]
    fn catches_a_wrong_percentage() {
        let v = verdicts("Sure! 17% of 240 is 41, so the tip is $41.");
        assert_eq!(v, vec![("17% of 240".into(), "41".into(), "fail")]);
        assert_eq!(check("17% of 240 is 41.").status, Status::Fail);
        assert_eq!(check("17% of 240 is 40.8.").status, Status::Pass);
    }

    #[test]
    fn handles_common_shapes() {
        let text = "First, 3/7 + 2/9 = 41/63. Then 2^10 = 1024. The square root of 144 is 12. \
                    12 times 12 equals 144. 1/3 is about 0.33. And 100 / 7 = 14.29.";
        let v = verdicts(text);
        assert_eq!(v.len(), 6, "{v:?}");
        assert!(v.iter().all(|c| c.2 == "pass"), "{v:?}");
    }

    #[test]
    fn multiple_claims_one_wrong() {
        let text = "The subtotal is 120 + 45 = 165. With 8% tax: 8% of 165 is 13.2. Total 165 + 13.2 = 178.5.";
        let v = verdicts(text);
        assert_eq!(v.iter().filter(|c| c.2 == "fail").count(), 1, "{v:?}");
        assert_eq!(v.iter().find(|c| c.2 == "fail").unwrap().0, "165 + 13.2");
    }

    #[test]
    fn prose_numbers_are_not_claims() {
        for t in [
            "The answer is 42.",
            "In 2024 the rate is 5%.",
            "Version 1.2 is 3 times faster.",
            "There were 3 apples and 4 pears.",
            "x = 5 when y = 2",
            "if a == 3 then b <= 4",
            "Use port 8080 = default.",
        ] {
            let c = extract(t);
            assert!(c.iter().all(|c| c.status != Status::Fail), "{t}: {c:?}");
        }
        assert_eq!(check("The answer is 42.").status, Status::Abstain);
    }

    #[test]
    fn chained_and_rhs_expressions_are_skipped_not_failed() {
        assert!(extract("2 + 3 = 5 + 0")
            .iter()
            .all(|c| c.status != Status::Fail));
        let v = verdicts("so 2 + 2 = 4 = 2 * 2");
        assert!(v.iter().all(|c| c.2 != "fail"), "{v:?}");
    }

    #[test]
    fn spans_point_at_the_claim() {
        let text = "Note: 6 * 7 = 43 here.";
        let c = &extract(text)[0];
        assert_eq!(&text[c.span[0]..c.span[1]], "6 * 7 = 43");
        assert_eq!(c.status, Status::Fail);
    }
}
