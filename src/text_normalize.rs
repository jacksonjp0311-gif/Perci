//! Bounded, dependency-free input normalization for Perci's language surface.
//!
//! Perci keeps the original turn for evidence and session memory, but routes a
//! repaired view through its operators and native binary fields.  This is not
//! a spell checker or a generative language model: it only applies an explicit
//! alias table plus a conservative one-edit match against a small vocabulary.
//! Unknown names and invented words therefore remain untouched.

const ALIASES: &[(&str, &str)] = &[
    ("intellegence", "intelligence"),
    ("inteligence", "intelligence"),
    ("intellgent", "intelligent"),
    ("converstaion", "conversation"),
    ("conversaton", "conversation"),
    ("langauge", "language"),
    ("genration", "generation"),
    ("natrual", "natural"),
    ("reasning", "reasoning"),
    ("speeling", "spelling"),
    ("speling", "spelling"),
    ("recusive", "recursive"),
    ("mathmatics", "mathematics"),
    ("understnd", "understand"),
    ("emergnce", "emergence"),
    ("adpative", "adaptive"),
    ("adpatve", "adaptive"),
    ("refelctive", "reflective"),
    ("tranformer", "transformer"),
    ("transformr", "transformer"),
    ("knowlege", "knowledge"),
    ("responce", "response"),
    ("throught", "thought"),
    ("atention", "attention"),
    ("exlpain", "explain"),
    ("calcualte", "calculate"),
    ("evoluton", "evolution"),
    ("magnitute", "magnitude"),
    ("differnce", "difference"),
    ("natual", "natural"),
    ("intrepret", "interpret"),
    ("interpet", "interpret"),
    ("compositionality", "compositionality"),
    ("goverened", "governed"),
    ("provenancee", "provenance"),
    ("consequece", "consequence"),
    ("preservee", "preserve"),
];

/// Canonical terms used for cautious one-edit correction.  The list is small
/// on purpose: broad fuzzy correction can silently damage names or domain
/// vocabulary, while these terms cover Perci's routing and dialogue surfaces.
const VOCABULARY: &[&str] = &[
    "adaptive",
    "attention",
    "binary",
    "boundary",
    "calculate",
    "capability",
    "conversation",
    "correction",
    "creative",
    "cognition",
    "difference",
    "emergence",
    "explain",
    "evolution",
    "geometry",
    "generation",
    "governed",
    "intelligence",
    "interpret",
    "language",
    "learning",
    "life",
    "magnitude",
    "mathematics",
    "memory",
    "natural",
    "reasoning",
    "reflective",
    "response",
    "retrieve",
    "routing",
    "sacred",
    "spelling",
    "system",
    "thought",
    "transformer",
    "understand",
];

/// Repair known Perci-domain misspellings while preserving punctuation and
/// whitespace.  The original casing of a token is retained where practical.
pub fn repair_typos(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut token = String::new();

    for character in input.chars() {
        if character.is_ascii_alphabetic() || (character == '\'' && !token.is_empty()) {
            token.push(character);
            continue;
        }
        flush_token(&mut output, &mut token);
        output.push(character);
    }
    flush_token(&mut output, &mut token);
    output
}

/// Lowercase, repaired routing key.  Operators use this form for matching;
/// response generation still receives the repaired, human-readable string.
pub fn normalize_for_routing(input: &str) -> String {
    repair_typos(input)
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn flush_token(output: &mut String, token: &mut String) {
    if token.is_empty() {
        return;
    }
    let repaired = repair_word(token);
    output.push_str(&repaired);
    token.clear();
}

fn repair_word(original: &str) -> String {
    let lower = original.to_ascii_lowercase();
    let canonical = ALIASES
        .iter()
        .find(|(broken, _)| *broken == lower)
        .map(|(_, repaired)| *repaired)
        .or_else(|| unique_one_edit_match(&lower));
    let Some(canonical) = canonical else {
        return original.to_owned();
    };
    preserve_case(original, canonical)
}

fn unique_one_edit_match(word: &str) -> Option<&'static str> {
    if word.len() < 5 || !word.bytes().all(|byte| byte.is_ascii_lowercase()) {
        return None;
    }
    let mut found = None;
    for candidate in VOCABULARY {
        if *candidate == word || damerau_levenshtein_at_most_one(word, candidate) != 1 {
            continue;
        }
        if found.is_some() {
            // Ambiguous fuzzy matches are left alone rather than guessed.
            return None;
        }
        found = Some(*candidate);
    }
    found
}

/// Damerau-Levenshtein distance specialized to the only distance we accept:
/// one insertion, deletion, substitution, or adjacent transposition.
fn damerau_levenshtein_at_most_one(left: &str, right: &str) -> u8 {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left == right {
        return 0;
    }
    if left.len().abs_diff(right.len()) > 1 {
        return 2;
    }
    if left.len() == right.len() {
        let differences = left
            .iter()
            .zip(right.iter())
            .enumerate()
            .filter_map(|(index, (a, b))| (a != b).then_some(index))
            .collect::<Vec<_>>();
        if differences.len() == 1 {
            return 1;
        }
        if differences.len() == 2 {
            let first = differences[0];
            let second = differences[1];
            if second == first + 1 && left[first] == right[second] && left[second] == right[first] {
                return 1;
            }
        }
        return 2;
    }

    let (shorter, longer) = if left.len() < right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let mut short_index = 0;
    let mut long_index = 0;
    let mut edits = 0;
    while short_index < shorter.len() && long_index < longer.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
        } else {
            edits += 1;
            if edits > 1 {
                return 2;
            }
            long_index += 1;
        }
    }
    1
}

fn preserve_case(original: &str, canonical: &str) -> String {
    if original
        .chars()
        .all(|character| !character.is_ascii_alphabetic() || character.is_ascii_uppercase())
    {
        return canonical.to_ascii_uppercase();
    }
    if original
        .chars()
        .next()
        .map(|character| character.is_ascii_uppercase())
        .unwrap_or(false)
    {
        let mut chars = canonical.chars();
        return chars
            .next()
            .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
            .unwrap_or_default();
    }
    canonical.to_owned()
}

#[cfg(test)]
mod tests {
    use super::{normalize_for_routing, repair_typos};

    #[test]
    fn repairs_domain_typos_without_touching_punctuation() {
        assert_eq!(
            repair_typos("Can you exlpain natral langauge, Perci?"),
            "Can you explain natural language, Perci?"
        );
    }

    #[test]
    fn repairs_transposition_and_one_edit_words() {
        assert_eq!(
            repair_typos("intellegence and atention"),
            "intelligence and attention"
        );
        assert_eq!(
            repair_typos("What is the differnce?"),
            "What is the difference?"
        );
    }

    #[test]
    fn leaves_unknown_names_and_spacing_alone() {
        assert_eq!(
            repair_typos("  Nembit  klaz-vexor!  "),
            "  Nembit  klaz-vexor!  "
        );
    }

    #[test]
    fn routing_key_is_repaired_and_stable() {
        assert_eq!(
            normalize_for_routing("  WHY is natral   language hard? "),
            "why is natural language hard?"
        );
    }
}
