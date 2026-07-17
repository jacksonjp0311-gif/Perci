//! Semantic evaluation beyond substring gates (v0.7.0).
//!
//! Layer stack (partial implementation):
//! L1 exact invariants · L2 schema · L3 claim presence · L4 distinction · L5 relation survival
//! Full entailment models / calibrated rubrics remain optional sidecars.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequiredClaim {
    pub claim: String,
    #[serde(default = "default_importance")]
    pub importance: f64,
}

fn default_importance() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticCase {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub capability: String,
    #[serde(default)]
    pub required_claims: Vec<RequiredClaim>,
    #[serde(default)]
    pub forbidden_claims: Vec<String>,
    /// Pairs of concepts that must both appear (distinction preserved).
    #[serde(default)]
    pub required_distinctions: Vec<[String; 2]>,
    /// Keywords that must survive entity/paraphrase mutations (relation core).
    #[serde(default)]
    pub relation_keywords: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticScore {
    pub id: String,
    pub pass: bool,
    pub claim_score: f64,
    pub distinction_score: f64,
    pub relation_score: f64,
    pub forbidden_hits: Vec<String>,
    pub missing_claims: Vec<String>,
    pub notes: Vec<String>,
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_owned())
        .collect()
}

fn claim_covered(answer: &str, claim: &str) -> bool {
    let al = answer.to_ascii_lowercase();
    let tokens = tokenize(claim);
    if tokens.is_empty() {
        return al.contains(&claim.to_ascii_lowercase());
    }
    // Require majority of content tokens (semantic proxy without embeddings).
    let hits = tokens.iter().filter(|t| al.contains(t.as_str())).count();
    let need = (tokens.len() + 1) / 2;
    hits >= need.max(1)
}

/// Score an answer against a semantic case (no external model).
pub fn evaluate_semantic(case: &SemanticCase, answer: &str) -> SemanticScore {
    let al = answer.to_ascii_lowercase();
    let mut missing = Vec::new();
    let mut claim_hits = 0.0;
    let mut claim_weight = 0.0;
    for c in &case.required_claims {
        claim_weight += c.importance;
        if claim_covered(answer, &c.claim) {
            claim_hits += c.importance;
        } else {
            missing.push(c.claim.clone());
        }
    }
    let claim_score = if claim_weight > 0.0 {
        claim_hits / claim_weight
    } else {
        1.0
    };

    let mut dist_ok = 0usize;
    let dist_n = case.required_distinctions.len().max(1);
    for pair in &case.required_distinctions {
        let a = pair[0].to_ascii_lowercase();
        let b = pair[1].to_ascii_lowercase();
        if al.contains(&a) && al.contains(&b) {
            dist_ok += 1;
        }
    }
    let distinction_score = if case.required_distinctions.is_empty() {
        1.0
    } else {
        dist_ok as f64 / dist_n as f64
    };

    let mut rel_ok = 0usize;
    let rel_n = case.relation_keywords.len().max(1);
    for k in &case.relation_keywords {
        if al.contains(&k.to_ascii_lowercase()) {
            rel_ok += 1;
        }
    }
    let relation_score = if case.relation_keywords.is_empty() {
        1.0
    } else {
        rel_ok as f64 / rel_n as f64
    };

    let forbidden_hits: Vec<String> = case
        .forbidden_claims
        .iter()
        .filter(|f| al.contains(&f.to_ascii_lowercase()))
        .cloned()
        .collect();

    let mut notes = Vec::new();
    for inv in &case.invariants {
        let il = inv.to_ascii_lowercase();
        if il.contains("uncertainty")
            && !(al.contains("unknown")
                || al.contains("uncertain")
                || al.contains("insufficient")
                || al.contains("cannot"))
        {
            notes.push(format!("invariant weak: {inv}"));
        }
    }

    let pass = forbidden_hits.is_empty()
        && claim_score >= 0.66
        && distinction_score >= 0.99
        && relation_score >= 0.5
        && notes.is_empty();

    SemanticScore {
        id: case.id.clone(),
        pass,
        claim_score,
        distinction_score,
        relation_score,
        forbidden_hits,
        missing_claims: missing,
        notes,
    }
}

/// Generate simple mutations for transfer-style testing.
pub fn mutate_prompt(prompt: &str) -> Vec<(String, &'static str)> {
    let mut out = vec![
        (prompt.to_owned(), "base"),
        (
            format!("In practical terms, {prompt} — explain carefully."),
            "paraphrase",
        ),
    ];
    // Entity inject (preserve structure).
    out.push((
        format!("{prompt} (entities: ZephyrNode, Quoril, NembitGate)"),
        "novel",
    ));
    // Negation stress (for evaluators that check forbidden flips).
    if prompt.contains("always") {
        out.push((prompt.replace("always", "sometimes"), "mutation"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_trust_claims_pass() {
        let case = SemanticCase {
            id: "S1".into(),
            prompt: "trust under lag".into(),
            capability: "systems".into(),
            required_claims: vec![RequiredClaim {
                claim: "retries must be idempotent under lag".into(),
                importance: 1.0,
            }],
            forbidden_claims: vec!["retries are always safe".into()],
            required_distinctions: vec![["timeout".into(), "proof".into()]],
            relation_keywords: vec!["timeout".into(), "idempotent".into(), "lag".into()],
            invariants: vec![],
        };
        let ans = "Timeouts are one-sided partial history without proof of remote outcome. \
Retries must be idempotent under lag so a delayed success is not a second write.";
        let s = evaluate_semantic(&case, ans);
        assert!(s.pass, "score={s:?}");
    }

    #[test]
    fn forbidden_claim_fails() {
        let case = SemanticCase {
            id: "S2".into(),
            prompt: "retries".into(),
            capability: "systems".into(),
            required_claims: vec![],
            forbidden_claims: vec!["retries are always safe".into()],
            required_distinctions: vec![],
            relation_keywords: vec![],
            invariants: vec![],
        };
        let s = evaluate_semantic(&case, "Retries are always safe on any network.");
        assert!(!s.pass);
        assert!(!s.forbidden_hits.is_empty());
    }
}
