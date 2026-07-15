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

/// Attempt an exact arithmetic operation.
///
/// `Ok(None)` means the prompt is not a supported executable arithmetic form,
/// so the language backend should continue rather than returning a tool error.
pub fn try_solve_arithmetic(text: &str) -> Result<Option<String>, ReasoningError> {
    let lower = text.to_ascii_lowercase();

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

    let normalized = lower
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
    text.trim()
        .parse()
        .map_err(|_| ReasoningError::InvalidNumber(text.trim().to_owned()))
}

fn extract_numbers(text: &str) -> Vec<i128> {
    text.split(|character: char| !character.is_ascii_digit() && character != '-')
        .filter(|value| !value.is_empty() && *value != "-")
        .filter_map(|value| value.parse().ok())
        .collect()
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
    fn percentage() {
        assert_eq!(
            try_solve_arithmetic("calculate 20 percent of 80").unwrap(),
            Some("16".to_owned())
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
