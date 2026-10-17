//! Source hygiene: no double-encoded UTF-8 (mojibake) in src/.
//! Patterns are written as escapes so this file does not match itself.

use std::fs;
use std::path::Path;

const MOJIBAKE: &[&str] = &[
    "\u{00cf}\u{20ac}",         // "Ï€"  (π read as cp1252)
    "\u{00e2}\u{02c6}\u{0161}", // "âˆš" (√)
    "\u{00c2}\u{00b2}",         // "Â²"  (²)
    "\u{00e2}\u{20ac}\u{201d}", // "â€”" (—)
    "\u{00e2}\u{20ac}\u{2122}", // "â€™" (’)
    "\u{0393}\u{00c7}",         // "ΓÇ"  (cp437 view of ’ — …)
];

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan(&path, hits);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                if MOJIBAKE.iter().any(|m| line.contains(m)) {
                    hits.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
    }
}

#[test]
fn src_has_no_mojibake() {
    let mut hits = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut hits,
    );
    assert!(hits.is_empty(), "mojibake found:\n{}", hits.join("\n"));
}
