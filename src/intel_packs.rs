//! Local intelligence-pack retrieval (works without Cortex daemon).
//!
//! Scans `knowledge/packs/**/*.md` for keyword overlap and returns
//! bounded guidance lines with provenance. Cortex may still refine
//! retrieval when warm; packs remain the offline backbone.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PackHit {
    pub path: String,
    pub title: String,
    pub score: usize,
    pub excerpt: String,
}

/// Discover pack root: PERCI_PACKS, else ./knowledge/packs, else host knowledge/packs.
pub fn packs_root() -> Option<PathBuf> {
    if let Some(p) = env::var_os("PERCI_PACKS") {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return Some(path);
        }
    }
    let candidates = [
        PathBuf::from("knowledge/packs"),
        env::current_dir()
            .ok()
            .map(|c| c.join("knowledge").join("packs"))
            .unwrap_or_default(),
        env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()))
            .map(|p| p.join("..").join("..").join("knowledge").join("packs"))
            .unwrap_or_default(),
    ];
    for c in candidates {
        if c.is_dir() {
            return Some(c);
        }
    }
    None
}

/// List pack ids (directory names under knowledge/packs).
pub fn list_packs() -> Vec<String> {
    let Some(root) = packs_root() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

/// Retrieve top pack cards for a query (offline, deterministic).
pub fn retrieve(query: &str, limit: usize) -> io::Result<Vec<PackHit>> {
    let Some(root) = packs_root() else {
        return Ok(Vec::new());
    };
    let terms = terms(query);
    if terms.is_empty() {
        return Ok(default_cards(&root, limit));
    }

    let mut hits: Vec<PackHit> = Vec::new();
    walk_md(&root, &mut |path, text| {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let title = first_heading(text).unwrap_or_else(|| rel.clone());
        let hay = text.to_ascii_lowercase();
        let mut score = 0usize;
        for t in &terms {
            if hay.contains(t) {
                score += 2;
            }
            if title.to_ascii_lowercase().contains(t) {
                score += 4;
            }
            if rel.to_ascii_lowercase().contains(t) {
                score += 3;
            }
        }
        // Domain priors from filename
        score += domain_prior(&rel, &terms);
        if score == 0 {
            return;
        }
        let excerpt = best_excerpt(text, &terms, 320);
        hits.push(PackHit {
            path: format!("knowledge/packs/{rel}"),
            title,
            score,
            excerpt,
        });
    })?;

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.dedup_by(|a, b| a.path == b.path);
    hits.truncate(limit.max(1));
    Ok(hits)
}

/// Format hits for backend context injection.
pub fn format_guidance(hits: &[PackHit]) -> Vec<String> {
    hits.iter()
        .map(|h| {
            format!(
                "[Pack: {} | {}] {}",
                h.path,
                h.title,
                h.excerpt.replace('\n', " ")
            )
        })
        .collect()
}

/// Compact status for /status and Lumen bridge.
pub fn status_summary() -> String {
    let packs = list_packs();
    let root = packs_root()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(missing)".into());
    if packs.is_empty() {
        format!("packs=0 root={root}")
    } else {
        format!(
            "packs={} · {} · root={}",
            packs.len(),
            packs.join(","),
            root
        )
    }
}

fn default_cards(root: &Path, limit: usize) -> Vec<PackHit> {
    let preferred = [
        "perci-deep-intelligence-v2/00-meta-control.md",
        "perci-deep-intelligence-v2/07-self-awareness.md",
        "perci-core-intelligence-v1/00-control-loop.md",
    ];
    let mut out = Vec::new();
    for rel in preferred {
        let path = root.join(rel);
        if let Ok(text) = fs::read_to_string(&path) {
            out.push(PackHit {
                path: format!("knowledge/packs/{rel}"),
                title: first_heading(&text).unwrap_or_else(|| rel.into()),
                score: 1,
                excerpt: best_excerpt(&text, &[], 280),
            });
        }
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn domain_prior(rel: &str, terms: &[String]) -> usize {
    let r = rel.to_ascii_lowercase();
    let mut bonus = 0;
    let pairs: &[(&str, &[&str])] = &[
        (
            "math",
            &[
                "math", "calcul", "number", "algebra", "geometr", "percent", "probab",
            ],
        ),
        (
            "coding",
            &[
                "code", "rust", "compile", "debug", "function", "patch", "test", "cargo",
            ],
        ),
        (
            "reasoning",
            &["reason", "logic", "proof", "argument", "hypothes", "deduc"],
        ),
        (
            "science",
            &["science", "experiment", "measure", "causal", "hypothesis"],
        ),
        (
            "language",
            &[
                "language",
                "write",
                "word",
                "communicat",
                "explain",
                "precision",
            ],
        ),
        (
            "introspect",
            &["introspect", "think", "metacog", "reflect", "uncertainty"],
        ),
        (
            "self-awareness",
            &["self", "identity", "aware", "perci", "who are", "limit"],
        ),
        (
            "governance",
            &["govern", "permission", "authority", "safety", "snapshot"],
        ),
        (
            "learning",
            &["learn", "memory", "cortex", "lesson", "train"],
        ),
        (
            "meta-control",
            &["plan", "control", "loop", "objective", "constraint"],
        ),
    ];
    for (key, keys) in pairs {
        if r.contains(key) {
            for t in terms {
                if keys.iter().any(|k| t.contains(k) || k.contains(t.as_str())) {
                    bonus += 5;
                }
            }
        }
    }
    bonus
}

fn walk_md(dir: &Path, f: &mut dyn FnMut(&Path, &str)) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_md(&path, f)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(text) = fs::read_to_string(&path) {
                f(&path, &text);
            }
        }
    }
    Ok(())
}

fn first_heading(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn best_excerpt(text: &str, terms: &[String], max: usize) -> String {
    let mut best_line = "";
    let mut best_score = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if !usable_excerpt_line(t) {
            continue;
        }
        let l = t.to_ascii_lowercase();
        let score = terms
            .iter()
            .filter(|term| l.contains(term.as_str()))
            .count();
        // Prefer imperative / operator lines
        let bonus = if t.starts_with('-')
            || t.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            1
        } else if t.contains('→') || t.contains("->") {
            2
        } else {
            0
        };
        let score = score + bonus;
        if score > best_score || (best_line.is_empty() && t.len() > 20) {
            best_score = score;
            best_line = t;
            if score >= 3 {
                break;
            }
        }
    }
    if best_line.is_empty() {
        for line in text.lines() {
            let t = line.trim();
            if usable_excerpt_line(t) && t.len() > 20 {
                best_line = t;
                break;
            }
        }
    }
    clip(best_line.trim_start_matches(['-', '•', '*', ' ']), max)
}

fn usable_excerpt_line(t: &str) -> bool {
    if t.is_empty() || t.starts_with('#') {
        return false;
    }
    // Skip markdown tables, dialogue leaks, noise
    if t.starts_with('|') || t.contains("---|") {
        return false;
    }
    let l = t.to_ascii_lowercase();
    if l.starts_with("user:") || l.starts_with("perci:") || l.contains("[recent dialogue]") {
        return false;
    }
    if t.contains("is not a geometry") || t.contains("is not a ") && t.len() < 80 {
        return false;
    }
    // Prefer actionable length
    t.len() >= 18 && t.len() <= 400
}

fn terms(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in query.split(|c: char| !c.is_alphanumeric()) {
        let w = raw.to_ascii_lowercase();
        if w.len() >= 3 && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_extract() {
        let t = terms("Fix cargo compile error in math");
        assert!(t.iter().any(|x| x == "cargo"));
        assert!(t.iter().any(|x| x == "math"));
    }
}
