use std::fmt;

#[derive(Debug)]
pub enum ReasoningError {
    Unsupported,
    InvalidNumber(String),
    DivisionByZero,
    InvalidGeometry,
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "I could not map that request to a supported exact operation"),
            Self::InvalidNumber(v) => write!(f, "invalid integer: {v}"),
            Self::DivisionByZero => write!(f, "division by zero is undefined"),
            Self::InvalidGeometry => write!(f, "invalid or incomplete geometry parameters"),
        }
    }
}

impl std::error::Error for ReasoningError {}

/// Exact rational value. No floating point is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational { pub numerator: i128, pub denominator: i128 }

impl Rational {
    pub fn new(n: i128, d: i128) -> Result<Self, ReasoningError> {
        if d == 0 { return Err(ReasoningError::DivisionByZero); }
        let sign = if d < 0 { -1 } else { 1 };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Ok(Self { numerator: sign * n / g, denominator: sign * d / g })
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 { write!(f, "{}", self.numerator) }
        else { write!(f, "{}/{}", self.numerator, self.denominator) }
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 { let r = a % b; a = b; b = r; }
    a.max(1)
}

/// Solves simple exact arithmetic expressions of the form `a op b`.
pub fn solve_arithmetic(text: &str) -> Result<String, ReasoningError> {
    let normalized = text
        .to_ascii_lowercase()
        .replace("calculate", "")
        .replace("what is", "")
        .replace("plus", "+")
        .replace("minus", "-")
        .replace("times", "*")
        .replace("multiplied by", "*")
        .replace("divided by", "/");

    for op in ['+', '*', '/'] {
        if let Some((left, right)) = normalized.split_once(op) {
            let a = parse_i128(left)?;
            let b = parse_i128(right)?;
            return match op {
                '+' => Ok((a + b).to_string()),
                '*' => Ok((a * b).to_string()),
                '/' => Ok(Rational::new(a, b)?.to_string()),
                _ => unreachable!(),
            };
        }
    }

    // Treat a minus sign as an operator only after the first character.
    if let Some(index) = normalized.char_indices().skip(1).find_map(|(i, c)| (c == '-').then_some(i)) {
        let a = parse_i128(&normalized[..index])?;
        let b = parse_i128(&normalized[index + 1..])?;
        return Ok((a - b).to_string());
    }
    Err(ReasoningError::Unsupported)
}

fn parse_i128(text: &str) -> Result<i128, ReasoningError> {
    let cleaned = text.trim().trim_matches(|c: char| !c.is_ascii_digit() && c != '-');
    cleaned.parse().map_err(|_| ReasoningError::InvalidNumber(cleaned.into()))
}

/// Handles a small exact geometry vocabulary using integer/rational formulas.
pub fn solve_geometry(text: &str) -> Result<String, ReasoningError> {
    let lower = text.to_ascii_lowercase();
    let nums: Vec<i128> = lower
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if lower.contains("triangle") && lower.contains("area") && nums.len() >= 2 {
        return Ok(format!("triangle area = {}", Rational::new(nums[0] * nums[1], 2)?));
    }
    if lower.contains("rectangle") && lower.contains("area") && nums.len() >= 2 {
        return Ok(format!("rectangle area = {}", nums[0] * nums[1]));
    }
    if lower.contains("pythag") && nums.len() >= 2 {
        let squared = nums[0] * nums[0] + nums[1] * nums[1];
        if let Some(root) = integer_sqrt_exact(squared) {
            return Ok(format!("hypotenuse = {root}"));
        }
        return Ok(format!("hypotenuse² = {squared}; exact hypotenuse = √{squared}"));
    }
    Err(ReasoningError::InvalidGeometry)
}

fn integer_sqrt_exact(value: i128) -> Option<i128> {
    if value < 0 { return None; }
    let mut low = 0i128;
    let mut high = value.min(1 << 64) + 1;
    while low + 1 < high {
        let mid = (low + high) / 2;
        if mid.saturating_mul(mid) <= value { low = mid; } else { high = mid; }
    }
    (low * low == value).then_some(low)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn exact_fraction() { assert_eq!(solve_arithmetic("10 divided by 4").unwrap(), "5/2"); }
    #[test] fn triangle() { assert_eq!(solve_geometry("triangle area base 8 height 5").unwrap(), "triangle area = 20"); }
}
