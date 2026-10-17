//! Char-safe text helpers shared across modules.

/// Trim, then cut to at most `max` characters (not bytes), ending with `…`
/// when shortened. Never panics on multi-byte UTF-8.
pub fn truncate(s: &str, max: usize) -> String {
    truncate_untrimmed(s.trim(), max)
}

/// Like [`truncate`] but keeps leading/trailing whitespace.
pub fn truncate_untrimmed(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Map typographic quotes and apostrophes to ASCII so phrase matching sees
/// `I’m` and `I'm` as the same text.
pub fn normalize_quotes(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains([
        '\u{2018}', '\u{2019}', '\u{201a}', '\u{201b}', '\u{2032}', '\u{201c}', '\u{201d}',
        '\u{201e}', '\u{2033}', '\u{ff07}',
    ]) {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(
        s.chars()
            .map(|c| match c {
                '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}' | '\u{2032}' | '\u{ff07}' => '\'',
                '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => '"',
                other => other,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_never_splits_a_multibyte_char() {
        // B2E-PERCI-002: agent.rs byte-sliced &t[..max] and panicked here.
        let s = "build log: tests passed … café résumé naïve ✓✓✓";
        for max in 0..s.len() + 2 {
            let out = truncate(s, max);
            assert!(out.chars().count() <= max.max(1));
        }
        assert_eq!(truncate("  é  ", 5), "é");
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate_untrimmed(" ab ", 10), " ab ");
    }

    #[test]
    fn curly_quotes_become_ascii() {
        assert_eq!(
            normalize_quotes("I\u{2019}m \u{201c}here\u{201d}"),
            "I'm \"here\""
        );
        assert!(matches!(
            normalize_quotes("plain"),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}
