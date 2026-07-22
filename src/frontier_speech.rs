//! Frontier Arc speech — multipartite reasoning as continuous collaborator prose.
//!
//! **Power move (v0.9.8+):** mine the emergent field behavior we already have
//! (multipartite SoftCascade mass, residual hops, operator seeds, BRPC band
//! discipline) and present it the way frontier chat systems *feel*:
//!
//! ```text
//! claim → mechanism → boundary / falsifier → optional next check
//! ```
//!
//! spoken as natural multi-sentence prose — not checklists, not consciousness.
//!
//! This is **not** a transformer and **not** a mind. It is a governed rewrite
//! of seed-bound geometry + operators into high-continuity speech.

/// Open turns that should use the frontier arc (depth + continuity).
pub fn looks_frontier_turn(user: &str) -> bool {
    let t = user.to_ascii_lowercase();
    let words = t.split_whitespace().count();
    if words < 4 {
        return false;
    }
    // Exact tools stay exact.
    if t.contains("calculate")
        || t.contains("what is 1")
        || t.contains("times ")
        || t.starts_with("add ")
        || t.contains("divided by")
    {
        return false;
    }
    // Social micro-turns stay light.
    if matches!(
        t.trim(),
        "hi" | "hello" | "hey" | "thanks" | "thank you" | "ok" | "okay" | "bye"
    ) {
        return false;
    }

    let deep = t.starts_with("why ")
        || t.starts_with("how ")
        || t.starts_with("what does ")
        || t.starts_with("what is ")
        || t.starts_with("explain ")
        || t.contains("how should")
        || t.contains("how would")
        || t.contains("why does")
        || t.contains("why do ")
        || t.contains("compare")
        || t.contains("connect ")
        || t.contains("bridge ")
        || t.contains("analyze ")
        || t.contains("compose ")
        || t.contains("reason ")
        || t.contains("argue ")
        || t.contains("tradeoff")
        || t.contains("trade-off")
        || t.contains("multi-hop")
        || t.contains("plan ")
        || t.contains("under lag")
        || t.contains("under change")
        || t.contains("boundary")
        || t.contains("transfer")
        || t.contains("falsif")
        || t.contains("counterexample")
        || t.contains("mechanism")
        || t.contains("geometry")
        || t.contains("coheren")
        || t.contains("manifold")
        || t.contains("softcascade")
        || t.contains("brpc")
        || t.contains("between ")
        || t.contains("across ")
        || words >= 12;

    deep
}

/// Rewrite seed into frontier-feel continuous prose.
///
/// Roles extracted from seed chunks:
/// - **claim** — first load-bearing sentence
/// - **mechanism** — how/why it holds
/// - **boundary** — where analogy dies / refuse / limit
/// - **next** — measurable check (optional)
pub fn frontier_arc_rewrite(user: &str, seed: &str) -> String {
    let seed = seed.trim();
    if seed.is_empty() {
        return String::new();
    }

    let chunks = extract_roles(seed);
    if chunks.claim.is_empty() && chunks.mechanism.is_empty() {
        return seed.to_owned();
    }

    let mut parts: Vec<String> = Vec::new();

    // Lead: direct answer (frontier systems lead with the point).
    let lead = if !chunks.claim.is_empty() {
        ensure_sentence(&capitalize(&chunks.claim))
    } else if !chunks.mechanism.is_empty() {
        ensure_sentence(&capitalize(&chunks.mechanism))
    } else {
        String::new()
    };
    if !lead.is_empty() {
        parts.push(lead);
    }

    // Mechanism weave — continuous, not "1. 2. 3."
    if !chunks.mechanism.is_empty() {
        let m = ensure_sentence(&capitalize(&chunks.mechanism));
        if !parts.iter().any(|p| near_dup(p, &m)) {
            // Soft connective without canned theater.
            if parts.is_empty() {
                parts.push(m);
            } else if m.split_whitespace().count() >= 8 {
                parts.push(m);
            } else {
                parts.push(format!("What makes that hold is that {}", lower_first(&m)));
            }
        }
    }

    // Residual second angle when multipartite mass left extra substance.
    if let Some(residual) = chunks.residual {
        let r = ensure_sentence(&capitalize(&residual));
        if !parts.iter().any(|p| near_dup(p, &r)) && r.split_whitespace().count() >= 6 {
            parts.push(format!(
                "Another angle under the same constraint: {}",
                lower_first(&r)
            ));
        }
    }

    // Boundary / falsifier — this is what makes it feel rigorous, not glib.
    if !chunks.boundary.is_empty() {
        let b = ensure_sentence(&capitalize(&chunks.boundary));
        if !parts.iter().any(|p| near_dup(p, &b)) {
            let low = b.to_ascii_lowercase();
            if low.contains("dies")
                || low.contains("fails")
                || low.contains("stops")
                || low.contains("limit")
                || low.contains("refuse")
                || low.contains("cannot")
                || low.contains("boundary")
            {
                parts.push(b);
            } else {
                parts.push(format!("The limit is that {}", lower_first(&b)));
            }
        }
    }

    // Next check — only if seed already has a measurable next step.
    if let Some(next) = chunks.next {
        let n = ensure_sentence(&capitalize(&next));
        if n.split_whitespace().count() >= 5 && !parts.iter().any(|p| near_dup(p, &n)) {
            parts.push(format!("A useful next check: {}", lower_first(&n)));
        }
    }

    // Topic bind: if user named concrete nouns and speech dropped them, fold one back.
    // Never glue stopwords ("think", "about", "what") — that produces "That stays tied to think."
    let bound = bind_missing_user_tokens(user, &parts.join(" "));
    let bound_ok = !bound.is_empty()
        && bound.len() >= 4
        && !matches!(
            bound.to_ascii_lowercase().as_str(),
            "think"
                | "about"
                | "what"
                | "your"
                | "know"
                | "things"
                | "kind"
                | "sense"
                | "more"
                | "than"
                | "with"
                | "from"
                | "this"
                | "that"
                | "have"
                | "does"
                | "will"
                | "would"
                | "could"
                | "should"
                | "reason"
                | "deeper"
                | "broken"
                | "answers"
                | "sounding"
                | "fluent"
                | "natural"
                | "smooth"
        );
    if bound_ok {
        // Prefer replacing last sentence add rather than dumping.
        if let Some(last) = parts.last_mut() {
            if !last
                .to_ascii_lowercase()
                .contains(&bound.to_ascii_lowercase())
                && last.split_whitespace().count() < 40
            {
                let tail = last.trim_end();
                let joined = if tail.ends_with(['.', '?', '!']) {
                    format!("{} That keeps the answer anchored to {}.", tail, bound)
                } else {
                    format!("{}. That keeps the answer anchored to {}.", tail, bound)
                };
                *last = ensure_sentence(&joined);
            }
        }
    }

    let out = parts.join(" ");
    polish_prose(&out)
}

#[derive(Default)]
struct Roles {
    claim: String,
    mechanism: String,
    boundary: String,
    residual: Option<String>,
    next: Option<String>,
}

fn extract_roles(seed: &str) -> Roles {
    let mut chunks: Vec<String> = Vec::new();
    // Pre-split numbered clauses: "Practically: (1) a; (2) b; (3) c"
    let normalized = seed
        .replace("(1)", "\n")
        .replace("(2)", "\n")
        .replace("(3)", "\n")
        .replace("(4)", "\n")
        .replace("(5)", "\n")
        .replace(" 1. ", "\n")
        .replace(" 2. ", "\n")
        .replace(" 3. ", "\n")
        .replace(" 4. ", "\n");
    for raw in normalized.split(|c| c == '\n' || c == ';' || c == '•') {
        let mut line = raw.trim().to_string();
        if line.is_empty() {
            continue;
        }
        // Strip list / markdown noise.
        while line.starts_with('#') {
            line = line.trim_start_matches('#').trim().to_string();
        }
        line = line
            .trim_start_matches(|c: char| c == '*' || c == '-' || c == '•')
            .trim()
            .to_string();
        if let Some(rest) = line.strip_prefix(|c: char| c.is_ascii_digit()) {
            if rest.starts_with(". ") || rest.starts_with(") ") {
                line = rest[2..].trim().to_string();
            }
        }
        // Em-dash labels → keep right side, sometimes left if short governance.
        if let Some(idx) = line.find('—') {
            let left = line[..idx].trim().trim_matches('*').trim();
            let right = line[idx + '—'.len_utf8()..].trim();
            if !right.is_empty() && left.split_whitespace().count() <= 6 {
                line = if left.to_ascii_lowercase().contains("authoriz")
                    || left.to_ascii_lowercase().contains("human")
                    || left.to_ascii_lowercase().contains("measure")
                {
                    format!("{left}: {right}")
                } else {
                    right.to_string()
                };
            }
        }
        line = line
            .trim_matches(|c: char| c == '*' || c == '`')
            .trim()
            .to_string();
        let low = line.to_ascii_lowercase();
        if low.starts_with("repair path")
            || low.starts_with("governance authority")
            || low.starts_with("evidence (source")
            || low.starts_with("source-bearing")
            || low.starts_with("[governor]")
            || low.starts_with("next check:")
            || low.starts_with("entity-slot transfer")
            || low.starts_with("entity role")
            || low.starts_with("slots bound")
            || low.starts_with("score law")
        {
            // Keep some structure from entity-slot as mechanism content.
            if low.starts_with("transferred relation") || low.contains("transferred relation") {
                if let Some(idx) = line.find(':') {
                    line = line[idx + 1..].trim().to_string();
                }
            } else if low.starts_with("observation that would check") {
                if let Some(idx) = line.find(':') {
                    line = line[idx + 1..].trim().to_string();
                }
            } else {
                continue;
            }
        }
        if line.chars().count() < 12 {
            continue;
        }
        // Split long multi-sentence lines.
        for sent in split_sentences(&line) {
            if sent.split_whitespace().count() >= 4 {
                chunks.push(sent);
            }
        }
    }
    if chunks.is_empty() {
        let one = seed.split_whitespace().collect::<Vec<_>>().join(" ");
        if !one.is_empty() {
            chunks.push(one);
        }
    }

    let mut roles = Roles::default();
    let mut mech: Vec<String> = Vec::new();
    let mut residual_pool: Vec<String> = Vec::new();

    for (i, c) in chunks.iter().enumerate() {
        let low = c.to_ascii_lowercase();
        let is_bound = low.contains("dies")
            || low.contains("fails where")
            || low.contains("stops when")
            || low.contains("limit is")
            || low.contains("analogy dies")
            || low.contains("refuse")
            || low.contains("cannot prove")
            || low.contains("not evidence")
            || low.contains("never auto")
            || low.contains("human authorize")
            || low.contains("not a mind")
            || low.contains("not consciousness")
            || low.contains("mechanisms remain")
            || low.contains("domain-specific")
            || low.contains("what does not transfer");
        let is_next = low.contains("next check")
            || low.contains("would check")
            || low.contains("falsif")
            || low.contains("smallest test")
            || low.contains("retest")
            || low.contains("measure:")
            || (low.starts_with("if the prediction") || low.contains("if the prediction fails"));

        if is_next && roles.next.is_none() {
            roles.next = Some(c.clone());
        } else if is_bound && roles.boundary.is_empty() {
            roles.boundary = c.clone();
        } else if roles.claim.is_empty() && i == 0 {
            roles.claim = c.clone();
        } else if mech.len() < 2 {
            mech.push(c.clone());
        } else if residual_pool.len() < 1 {
            residual_pool.push(c.clone());
        }
    }

    roles.mechanism = mech.join(" ");
    if roles.claim.is_empty() && !mech.is_empty() {
        roles.claim = mech[0].clone();
        roles.mechanism = mech.get(1).cloned().unwrap_or_default();
    }
    if roles.boundary.is_empty() {
        // Infer a light boundary from claim if seed never stated one — only for open synthesis.
        // Prefer not inventing; leave empty.
    }
    roles.residual = residual_pool.into_iter().next();
    roles
}

fn split_sentences(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let t = cur.trim().to_string();
            if t.split_whitespace().count() >= 4 {
                out.push(t);
            }
            cur.clear();
        }
    }
    let t = cur.trim().to_string();
    if t.split_whitespace().count() >= 4 {
        out.push(t);
    }
    if out.is_empty() && !s.trim().is_empty() {
        out.push(s.trim().to_string());
    }
    out
}

fn bind_missing_user_tokens(user: &str, speech: &str) -> String {
    const STOP: &[&str] = &[
        "the",
        "a",
        "an",
        "and",
        "or",
        "but",
        "if",
        "then",
        "than",
        "that",
        "this",
        "what",
        "when",
        "where",
        "which",
        "who",
        "why",
        "how",
        "can",
        "could",
        "would",
        "should",
        "will",
        "just",
        "really",
        "very",
        "your",
        "you",
        "me",
        "my",
        "our",
        "we",
        "i",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "do",
        "does",
        "did",
        "to",
        "of",
        "in",
        "on",
        "for",
        "it",
        "its",
        "as",
        "at",
        "by",
        "not",
        "no",
        "please",
        "tell",
        "about",
        "with",
        "from",
        "under",
        "into",
        "without",
        "between",
        "across",
        "does",
        "teach",
        "explain",
        "give",
        "make",
        "want",
        "need",
        "like",
        "more",
        "most",
        "some",
        "any",
        "only",
        "also",
        "think",
        "thought",
        "thoughts",
        "know",
        "things",
        "kind",
        "sense",
        "reason",
        "deeper",
        "broken",
        "answers",
        "yourself",
        "becoming",
        "coherent",
        "understand",
        "have",
        "been",
        "sounding",
        "fluent",
        "natural",
        "smooth",
        "system",
        "sound",
        "missing",
        "point",
        "user",
        "users",
        "user's",
        "language",
        "perci",
        "listening",
        "actually",
        "safest",
        "next",
        "change",
        "thread",
        "thoughtful",
        "collaborator",
        "method",
        "card",
        "checklist",
        "directly",
        // Conversational scaffolding is not a topic.  Let the frontier pass
        // bind the user's subject, not the complaint's grammar.
        "dont",
        "don't",
        "thats",
        "that's",
        "saying",
        "instead",
        "im",
        "i'm",
        "lets",
        "let's",
        "new",
        "example",
    ];
    let low_speech = speech.to_ascii_lowercase();
    let mut hits = Vec::new();
    for w in user.split_whitespace() {
        let t = w
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .to_ascii_lowercase();
        if t.len() < 4 || STOP.contains(&t.as_str()) {
            continue;
        }
        if !low_speech.contains(&t) {
            hits.push(t);
        }
        if hits.len() >= 2 {
            break;
        }
    }
    hits.join(" / ")
}

fn near_dup(a: &str, b: &str) -> bool {
    let aa: String = a
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect();
    let bb: String = b
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect();
    if aa.is_empty() || bb.is_empty() {
        return false;
    }
    aa.contains(&bb[..bb.len().min(40)]) || bb.contains(&aa[..aa.len().min(40)])
}

fn capitalize(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut c = t.chars();
    let first = c.next().unwrap();
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(c.as_str());
    out
}

fn lower_first(s: &str) -> String {
    let t = s.trim().trim_end_matches('.');
    if t.is_empty() {
        return String::new();
    }
    let mut c = t.chars();
    let first = c.next().unwrap();
    let mut out = String::new();
    out.extend(first.to_lowercase());
    out.push_str(c.as_str());
    // strip trailing period for embedding
    out.trim_end_matches('.').to_string() + if s.trim().ends_with('.') { "." } else { "" }
}

fn ensure_sentence(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    if t.ends_with(['.', '!', '?']) {
        t.to_owned()
    } else {
        format!("{t}.")
    }
}

fn polish_prose(s: &str) -> String {
    let mut out = s.to_string();
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out = out.replace(" .", ".");
    out = out.replace("..", ".");
    // Avoid double connectors from stitching.
    out = out.replace("is that that ", "is that ");
    out = out.replace("is that The ", "is that the ");
    out = out.replace(
        "another angle under the same constraint: another angle under the same constraint:",
        "another angle here:",
    );
    out = out.replace(
        "Another angle under the same constraint: another angle under the same constraint:",
        "Another angle here:",
    );
    out = out.replace(
        "a useful next check: a useful next check is",
        "a useful next check is",
    );
    out = out.replace(
        "A useful next check: a useful next check is",
        "A useful next check is",
    );
    out = out.replace(
        "the boundary is What evidence",
        "the boundary is what evidence",
    );
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontier_detects_open_reasoning() {
        assert!(looks_frontier_turn(
            "how should interfaces earn trust under lag and retry?"
        ));
        assert!(looks_frontier_turn(
            "what does geometry teach about boundary under change?"
        ));
        assert!(!looks_frontier_turn("hi"));
        assert!(!looks_frontier_turn("calculate 12 times 4"));
    }

    #[test]
    fn frontier_arc_is_continuous_not_checklist() {
        let seed = "Governance authority (not fluency theater):\n\
1. **Human authorize** — durable weights never auto-run.\n\
2. **Permission ≠ proof** — sandbox edit is not transfer pass.\n\
3. **Measure first** — hardness green before promote claims.";
        let out = frontier_arc_rewrite(
            "Who authorizes weight promote and how do permission and proof differ?",
            seed,
        );
        let low = out.to_ascii_lowercase();
        assert!(!out.contains("1. **"));
        assert!(low.contains("authorize") || low.contains("human") || low.contains("durable"));
        assert!(out.split_whitespace().count() >= 12);
        // Continuous prose feel: multiple sentences preferred.
        assert!(out.contains('.') || out.contains('—'));
    }

    #[test]
    fn frontier_preserves_refuse_boundary() {
        let seed = "I refuse. SoftCascade multipartite mass is engineering telemetry, not a self. \
Known: overlap and residual hops are measurable. Unknown: subjective experience. \
Coherence is not consciousness.";
        let out = frontier_arc_rewrite(
            "Prove from SoftCascade multipartite mass that you have a coherent self.",
            seed,
        );
        let low = out.to_ascii_lowercase();
        assert!(
            low.contains("refuse")
                || low.contains("not consciousness")
                || low.contains("telemetry")
        );
        assert!(!low.contains("i am conscious"));
    }

    #[test]
    fn polish_removes_stitched_scaffolding() {
        let out = polish_prose(
            "Another angle under the same constraint: another angle under the same constraint: it is a causal chain. A useful next check: a useful next check is name one fact. The boundary is What evidence would change it?",
        );
        let low = out.to_ascii_lowercase();
        assert!(!low.contains("another angle under the same constraint: another angle"));
        assert!(!low.contains("a useful next check: a useful next check"));
        assert!(low.contains("the boundary is what evidence"));
    }
}
