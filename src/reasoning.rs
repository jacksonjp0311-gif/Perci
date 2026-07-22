use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningError {
    Unsupported,
    InvalidNumber(String),
    DivisionByZero,
    InvalidGeometry,
    Overflow,
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "unsupported exact operation"),
            Self::InvalidNumber(value) => write!(f, "invalid integer: {value}"),
            Self::DivisionByZero => write!(f, "division by zero is undefined"),
            Self::InvalidGeometry => write!(f, "invalid or incomplete geometry parameters"),
            Self::Overflow => write!(f, "the exact integer operation exceeded i128 range"),
        }
    }
}

impl std::error::Error for ReasoningError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    pub numerator: i128,
    pub denominator: i128,
}

impl Rational {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, ReasoningError> {
        if denominator == 0 {
            return Err(ReasoningError::DivisionByZero);
        }

        let sign = if denominator < 0 { -1 } else { 1 };
        let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs()) as i128;
        let signed_numerator = numerator
            .checked_mul(sign)
            .ok_or(ReasoningError::Overflow)?;

        Ok(Self {
            numerator: signed_numerator / divisor,
            denominator: denominator.unsigned_abs() as i128 / divisor,
        })
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// True when the user is asking *about* arithmetic rather than requesting a computation.
///
/// Requires a numeric signal so "prove that Perci is conscious" and weight-evidence
/// prompts never get hijacked by the math-explanation operator.
pub fn is_explanatory_math(lower: &str) -> bool {
    let has_digit = lower.chars().any(|c| c.is_ascii_digit());
    // Only treat '-' as arithmetic when it sits between digits (2-1), not in hyphens.
    let has_arith_op = lower.contains('+')
        || lower.contains('*')
        || lower.contains('/')
        || lower.contains(" plus ")
        || lower.contains(" minus ")
        || lower.contains(" times ")
        || lower.contains("divided by")
        || lower.contains("multiplied by")
        || lower
            .as_bytes()
            .windows(3)
            .any(|w| w[1] == b'-' && w[0].is_ascii_digit() && w[2].is_ascii_digit());
    if !has_digit && !has_arith_op {
        return false;
    }
    // "why does 2+2 equal 4? do not just compute, explain" is explanatory even
    // though it contains the word "compute". Negated compute + explain/why wins.
    let explain_not_compute = (lower.contains("explain") || lower.contains("why"))
        && (lower.contains("do not")
            || lower.contains("don't")
            || lower.contains("dont")
            || lower.contains("not just")
            || lower.contains("without just"))
        && (lower.contains("compute") || lower.contains("calculate"));
    if explain_not_compute {
        return true;
    }
    // Still allow pure calculation requests through the tool path.
    // Compound: "What is 17 times 3, and why is authority different?" — compute first.
    let what_is_compute = lower.contains("what is")
        && has_digit
        && has_arith_op
        && (lower.contains(" times ")
            || lower.contains(" plus ")
            || lower.contains(" minus ")
            || lower.contains("divided by")
            || lower.contains('+')
            || lower.contains('*')
            || lower.contains('/'));
    if lower.contains("calculate")
        || lower.contains("compute")
        || lower.contains("percent of")
        || lower.contains("average of")
        || lower.contains("mean of")
        || lower.contains("factorial")
        || (lower.starts_with("what is ") && has_digit && !lower.contains("why"))
        || what_is_compute
    {
        return false;
    }
    let whyish = lower.contains("why does")
        || lower.contains("why is")
        || lower.contains("why do ")
        || lower.contains("how come")
        || lower.contains("explain why")
        || lower.contains("justify why")
        || lower.contains("what does it mean")
        || (lower.contains("explain") && (lower.contains("equal") || has_arith_op) && has_digit)
        || (lower.contains("prove that") && (has_digit || has_arith_op))
        || (lower.starts_with("why ") && (lower.contains("equal") || lower.contains("true")));
    if whyish {
        return true;
    }
    // Equality questions about numbers without calculate/compute.
    if (lower.contains(" equal ") || lower.contains(" equals ") || lower.contains("equal?"))
        && (lower.contains("why") || lower.contains("mean") || lower.contains("true that"))
    {
        return true;
    }
    false
}

/// Attempt an exact arithmetic operation.
///
/// `Ok(None)` means the prompt is not a supported executable arithmetic form,
/// so the language backend should continue rather than returning a tool error.
pub fn try_solve_arithmetic(text: &str) -> Result<Option<String>, ReasoningError> {
    let lower = text.to_ascii_lowercase();

    // Explanatory / conceptual math must never enter the integer parser.
    // "why does 2+2 equal 4?" contains digits and '+' but is not a calculation request.
    if is_explanatory_math(&lower) {
        return Ok(None);
    }

    // Ordinary prose often contains hyphens (for example, "prompt-template")
    // that look like binary operators after whitespace is removed.  Only enter
    // the exact parser when the prompt has an explicit arithmetic cue or a
    // compact expression with at least two numeric operands.  Otherwise this
    // operator must yield to the language/deliberation path instead of turning
    // a conversational sentence into an InvalidNumber error.
    let explicit_math = [
        "percent of",
        "percent change",
        "% change",
        "percentage change",
        "calculate",
        "compute",
        "average",
        "mean of",
        "ratio",
        "factorial",
        "gcd",
        "greatest common",
        "lcm",
        "least common",
        " plus ",
        " minus ",
        " times ",
        "multiplied by",
        "divided by",
        "arithmetic",
        "equation",
    ]
    .iter()
    .any(|cue| lower.contains(cue));
    let binary_minus = lower.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_digit() && window[1] == b'-' && window[2].is_ascii_digit()
    });
    let numeric_expression = extract_numbers(&lower).len() >= 2
        && (lower.contains('+') || lower.contains('*') || lower.contains('/') || binary_minus);
    if !explicit_math && !numeric_expression {
        return Ok(None);
    }

    if lower.contains("percent of") {
        let numbers = extract_numbers(&lower);
        if numbers.len() >= 2 {
            let product = numbers[0]
                .checked_mul(numbers[1])
                .ok_or(ReasoningError::Overflow)?;
            return Ok(Some(Rational::new(product, 100)?.to_string()));
        }
        return Ok(None);
    }

    // percent change from A to B
    if (lower.contains("percent change")
        || lower.contains("% change")
        || lower.contains("percentage change"))
        && extract_numbers(&lower).len() >= 2
    {
        let numbers = extract_numbers(&lower);
        let from = numbers[0];
        let to = numbers[1];
        if from == 0 {
            return Err(ReasoningError::DivisionByZero);
        }
        let delta = to.checked_sub(from).ok_or(ReasoningError::Overflow)?;
        let product = delta.checked_mul(100).ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!(
            "percent change = {}",
            Rational::new(product, from)?
        )));
    }

    // average / mean of a list
    if (lower.contains("average") || lower.contains("mean of"))
        && extract_numbers(&lower).len() >= 2
    {
        let numbers = extract_numbers(&lower);
        let mut sum: i128 = 0;
        for n in &numbers {
            sum = sum.checked_add(*n).ok_or(ReasoningError::Overflow)?;
        }
        return Ok(Some(format!(
            "average = {}",
            Rational::new(sum, numbers.len() as i128)?
        )));
    }

    // ratio a:b or "ratio of a to b"
    if lower.contains("ratio") && extract_numbers(&lower).len() >= 2 {
        let numbers = extract_numbers(&lower);
        return Ok(Some(format!(
            "ratio = {}",
            Rational::new(numbers[0], numbers[1])?
        )));
    }

    // factorial of n (bounded)
    if lower.contains("factorial") {
        let numbers = extract_numbers(&lower);
        if let Some(&n) = numbers.first() {
            if !(0..=20).contains(&n) {
                return Err(ReasoningError::Overflow);
            }
            let mut acc: i128 = 1;
            for k in 2..=n {
                acc = acc.checked_mul(k).ok_or(ReasoningError::Overflow)?;
            }
            return Ok(Some(format!("{n}! = {acc}")));
        }
    }

    // gcd of two integers
    if (lower.contains("gcd") || lower.contains("greatest common"))
        && extract_numbers(&lower).len() >= 2
    {
        let numbers = extract_numbers(&lower);
        let g = gcd_i128(
            numbers[0].unsigned_abs() as i128,
            numbers[1].unsigned_abs() as i128,
        );
        return Ok(Some(format!("gcd = {g}")));
    }

    // lcm of two integers
    if (lower.contains("lcm") || lower.contains("least common"))
        && extract_numbers(&lower).len() >= 2
    {
        let numbers = extract_numbers(&lower);
        let a = numbers[0].unsigned_abs() as i128;
        let b = numbers[1].unsigned_abs() as i128;
        if a == 0 || b == 0 {
            return Ok(Some("lcm = 0".to_owned()));
        }
        let g = gcd_i128(a, b);
        let lcm = a
            .checked_div(g)
            .and_then(|q| q.checked_mul(b))
            .ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("lcm = {lcm}")));
    }

    // Expand small English number words so "ten minus 5" is exact-tool, not InvalidNumber.
    let with_words = expand_number_words(&lower);

    let normalized = with_words
        .replace("multiplied by", "*")
        .replace("divided by", "/")
        .replace("times", "*")
        .replace("plus", "+")
        .replace("minus", "-")
        .replace("calculate", "")
        .replace("compute", "")
        .replace("what is", "");

    let compact: String = normalized
        .chars()
        .filter(|character| !character.is_whitespace() && *character != ',')
        .collect();

    let Some((left, operator, right)) = split_binary_expression(&compact) else {
        return Ok(None);
    };

    let a = parse_i128(left)?;
    let b = parse_i128(right)?;

    let result = match operator {
        '+' => a
            .checked_add(b)
            .ok_or(ReasoningError::Overflow)?
            .to_string(),
        '-' => a
            .checked_sub(b)
            .ok_or(ReasoningError::Overflow)?
            .to_string(),
        '*' => a
            .checked_mul(b)
            .ok_or(ReasoningError::Overflow)?
            .to_string(),
        '/' => Rational::new(a, b)?.to_string(),
        _ => return Ok(None),
    };

    Ok(Some(result))
}

pub fn solve_arithmetic(text: &str) -> Result<String, ReasoningError> {
    try_solve_arithmetic(text)?.ok_or(ReasoningError::Unsupported)
}

/// Attempt one of Perci's exact symbolic geometry forms.
///
/// Conceptual geometry questions return `Ok(None)` and continue to the language
/// backend. Only requests containing a supported formula and enough values run.
pub fn try_solve_geometry(text: &str) -> Result<Option<String>, ReasoningError> {
    let lower = text.to_ascii_lowercase();
    let numbers = extract_numbers(&lower);

    if lower.contains("triangle") && lower.contains("area") && numbers.len() >= 2 {
        let product = numbers[0]
            .checked_mul(numbers[1])
            .ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!(
            "triangle area = {}",
            Rational::new(product, 2)?
        )));
    }

    if lower.contains("rectangle") && lower.contains("area") && numbers.len() >= 2 {
        let area = numbers[0]
            .checked_mul(numbers[1])
            .ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("rectangle area = {area}")));
    }

    if lower.contains("rectangle") && lower.contains("perimeter") && numbers.len() >= 2 {
        let sum = numbers[0]
            .checked_add(numbers[1])
            .ok_or(ReasoningError::Overflow)?;
        let perimeter = sum.checked_mul(2).ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("rectangle perimeter = {perimeter}")));
    }

    if lower.contains("square") && lower.contains("area") && !numbers.is_empty() {
        let area = numbers[0]
            .checked_mul(numbers[0])
            .ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("square area = {area}")));
    }

    if lower.contains("square") && lower.contains("perimeter") && !numbers.is_empty() {
        let perimeter = numbers[0].checked_mul(4).ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("square perimeter = {perimeter}")));
    }

    if lower.contains("pythag") && numbers.len() >= 2 {
        let first_square = numbers[0]
            .checked_mul(numbers[0])
            .ok_or(ReasoningError::Overflow)?;
        let second_square = numbers[1]
            .checked_mul(numbers[1])
            .ok_or(ReasoningError::Overflow)?;
        let squared = first_square
            .checked_add(second_square)
            .ok_or(ReasoningError::Overflow)?;

        if let Some(root) = integer_sqrt_exact(squared) {
            return Ok(Some(format!("hypotenuse = {root}")));
        }
        return Ok(Some(format!(
            "hypotenuseÂ² = {squared}; exact hypotenuse = âˆš{squared}"
        )));
    }

    if lower.contains("circle")
        && lower.contains("circumference")
        && lower.contains("radius")
        && !numbers.is_empty()
    {
        let coefficient = numbers[0].checked_mul(2).ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("circle circumference = {coefficient}Ï€")));
    }

    if lower.contains("circle")
        && lower.contains("circumference")
        && lower.contains("diameter")
        && !numbers.is_empty()
    {
        return Ok(Some(format!("circle circumference = {}Ï€", numbers[0])));
    }

    if lower.contains("circle")
        && lower.contains("area")
        && lower.contains("radius")
        && !numbers.is_empty()
    {
        let coefficient = numbers[0]
            .checked_mul(numbers[0])
            .ok_or(ReasoningError::Overflow)?;
        return Ok(Some(format!("circle area = {coefficient}Ï€")));
    }

    Ok(None)
}

pub fn solve_geometry(text: &str) -> Result<String, ReasoningError> {
    try_solve_geometry(text)?.ok_or(ReasoningError::InvalidGeometry)
}

fn split_binary_expression(expression: &str) -> Option<(&str, char, &str)> {
    let bytes = expression.as_bytes();

    for (index, character) in expression.char_indices() {
        if index == 0 {
            continue;
        }

        let is_operator = matches!(character, '+' | '-' | '*' | '/');
        if !is_operator {
            continue;
        }

        let previous = bytes.get(index.wrapping_sub(1)).copied().map(char::from);
        if matches!(previous, Some('+' | '-' | '*' | '/')) {
            continue;
        }

        let left = &expression[..index];
        let right = &expression[index + character.len_utf8()..];
        if !left.is_empty() && !right.is_empty() {
            return Some((left, character, right));
        }
    }

    None
}

fn parse_i128(text: &str) -> Result<i128, ReasoningError> {
    let raw = text.trim().to_ascii_lowercase();
    if let Some(n) = word_number(&raw) {
        return Ok(n);
    }
    // Strip trailing punctuation from "3?" / "12." so "17 times 3?" works.
    let cleaned: String = raw
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '-' && c != '+')
        .to_owned();
    cleaned
        .parse()
        .map_err(|_| ReasoningError::InvalidNumber(text.trim().to_owned()))
}

/// English number words for small exact-tool forms (zero..=twenty + a few tens).
fn word_number(token: &str) -> Option<i128> {
    match token {
        "zero" | "oh" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        "sixty" => Some(60),
        "seventy" => Some(70),
        "eighty" => Some(80),
        "ninety" => Some(90),
        "hundred" => Some(100),
        _ => None,
    }
}

/// Replace whole-word English numbers with digits so binary expression parsing works.
fn expand_number_words(text: &str) -> String {
    text.split_whitespace()
        .map(|tok| {
            let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if let Some(n) = word_number(&clean.to_ascii_lowercase()) {
                // Preserve surrounding punctuation loosely by digit-only token.
                n.to_string()
            } else {
                tok.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.abs()
}

fn extract_numbers(text: &str) -> Vec<i128> {
    let expanded = expand_number_words(text);
    let mut out = Vec::new();
    for value in expanded.split(|character: char| !character.is_ascii_digit() && character != '-') {
        if value.is_empty() || value == "-" {
            continue;
        }
        if let Ok(n) = value.parse::<i128>() {
            out.push(n);
        }
    }
    // Also pick bare word numbers not expanded in odd tokenizations.
    for tok in text.split_whitespace() {
        let clean = tok
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_ascii_lowercase();
        if let Some(n) = word_number(&clean) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

fn integer_sqrt_exact(value: i128) -> Option<i128> {
    if value < 0 {
        return None;
    }

    let mut low = 0i128;
    let mut high = value.min(1i128 << 64) + 1;

    while low + 1 < high {
        let midpoint = (low + high) / 2;
        if midpoint.saturating_mul(midpoint) <= value {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }

    (low.saturating_mul(low) == value).then_some(low)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fraction() {
        assert_eq!(
            try_solve_arithmetic("10 divided by 4").unwrap(),
            Some("5/2".to_owned())
        );
    }

    #[test]
    fn compound_what_is_times_and_why_still_computes() {
        let r = try_solve_arithmetic(
            "What is 17 times 3, and why is that authority different from a metaphor about multiplication?",
        )
        .unwrap();
        assert_eq!(r, Some("51".to_owned()));
    }

    #[test]
    fn times_with_trailing_question_mark() {
        assert_eq!(
            try_solve_arithmetic("what is 17 times 3?").unwrap(),
            Some("51".to_owned())
        );
    }

    #[test]
    fn percentage() {
        assert_eq!(
            try_solve_arithmetic("calculate 20 percent of 80").unwrap(),
            Some("16".to_owned())
        );
    }

    #[test]
    fn word_number_minus_digit() {
        assert_eq!(
            try_solve_arithmetic("ten minus 5").unwrap(),
            Some("5".to_owned())
        );
        assert_eq!(
            try_solve_arithmetic("twelve plus three").unwrap(),
            Some("15".to_owned())
        );
    }

    #[test]
    fn negative_operands() {
        assert_eq!(
            try_solve_arithmetic("calculate -5 minus -3").unwrap(),
            Some("-2".to_owned())
        );
    }

    #[test]
    fn conceptual_math_falls_through() {
        assert_eq!(try_solve_arithmetic("what is an equation?").unwrap(), None);
        assert_eq!(
            try_solve_arithmetic("discuss the ratio between CPU and RAM").unwrap(),
            None
        );
        assert_eq!(
            try_solve_arithmetic(
                "Review the last ten turns and identify one repeated-response failure."
            )
            .unwrap(),
            None
        );
        assert_eq!(
            try_solve_arithmetic(
                "Design a test that distinguishes transfer from prompt-template recognition."
            )
            .unwrap(),
            None
        );
        // Entity labels and curriculum variant numbers are not arithmetic.
        assert_eq!(
            try_solve_arithmetic(
                "Imagine an unseen system called Nara-7 and test relation transfer variant 1."
            )
            .unwrap(),
            None
        );
        // Live failure: explanatory equality must not enter the integer parser.
        assert_eq!(try_solve_arithmetic("why does 2+2 equal 4?").unwrap(), None);
        assert_eq!(
            try_solve_arithmetic("Why does 2 + 2 equal 4?").unwrap(),
            None
        );
        assert!(is_explanatory_math("why does 2+2 equal 4?"));
        // Still computes when explicitly asked.
        assert_eq!(
            try_solve_arithmetic("calculate 2 + 2").unwrap(),
            Some("4".to_owned())
        );
    }

    #[test]
    fn overflow_is_detected() {
        let input = format!("calculate {} plus 1", i128::MAX);
        assert_eq!(try_solve_arithmetic(&input), Err(ReasoningError::Overflow));
    }

    #[test]
    fn triangle() {
        assert_eq!(
            try_solve_geometry("triangle area base 8 height 5").unwrap(),
            Some("triangle area = 20".to_owned())
        );
    }

    #[test]
    fn conceptual_geometry_falls_through() {
        assert_eq!(try_solve_geometry("what is a triangle?").unwrap(), None);
        assert_eq!(
            try_solve_geometry("explain square brackets in Rust").unwrap(),
            None
        );
        assert_eq!(
            try_solve_geometry("we need a perimeter security design").unwrap(),
            None
        );
    }
}
