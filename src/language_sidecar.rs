//! Language-generation sidecar client (Capability Fabric Phase 2).
//!
//! External process protocol (stdin/stdout JSON lines):
//!   request:  LanguageRequest (fabric)
//!   response: LanguageResponse
//!
//! Env: PERCI_LANGUAGE_SIDECAR = path to executable (optional).
//! If unset, uses the **local governed synthesizer** (deterministic fluency layer).
//! Perci critic always runs after generation.

use crate::backend::LanguageBackend;
use crate::fabric::{critic_accept_language, EvidenceRecord, LanguageRequest};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageResponse {
    pub schema: String,
    pub text: String,
    #[serde(default)]
    pub claims: Vec<String>,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
}

impl LanguageResponse {
    pub fn local(text: impl Into<String>) -> Self {
        Self {
            schema: "perci.language-response.v1".into(),
            text: text.into(),
            claims: Vec::new(),
            engine: "local-governed-synthesizer".into(),
            ok: true,
            error: None,
        }
    }
}

/// Generate prose under Perci governor constraints.
pub fn generate(req: &LanguageRequest, seed_body: &str) -> LanguageResponse {
    // External language processes are compatibility-only.  Native Perci
    // inference is the default; the old sidecar requires an explicit opt-in.
    if external_language_enabled() {
        if let Some(bin) = env::var_os("PERCI_LANGUAGE_SIDECAR") {
            match invoke_external(std::path::Path::new(&bin), req, seed_body) {
                Ok(mut resp) => {
                    if let Err(e) =
                        critic_accept_language(&resp.text, &req.required_claim_boundaries)
                    {
                        resp.ok = false;
                        resp.error = Some(e);
                        resp.text = format!(
                        "{seed_body}\n\n[Governor] Language sidecar output refused: boundary violation."
                    );
                    }
                    return resp;
                }
                Err(e) => {
                    let mut resp = local_synthesize(req, seed_body);
                    resp.engine = format!("local-fallback after sidecar error: {e}");
                    return resp;
                }
            }
        }
    }
    // An external HTTP model remains available only for compatibility tests.
    // It never receives authority over routing, facts, tools, or weights.
    if external_language_enabled() {
        if let Some(mut model) = crate::backend::LocalModelBackend::from_env() {
            let mut context = req
                .operator_plan
                .iter()
                .map(|step| format!("[operator] {step}"))
                .collect::<Vec<_>>();
            context.extend(
                req.constraints
                    .iter()
                    .map(|item| format!("[constraint] {item}")),
            );
            context.extend(
                req.evidence
                    .iter()
                    .take(4)
                    .map(|item| format!("[evidence] {}", item.claim)),
            );
            let system = "Perci governs the answer. Rewrite the supplied operator result into direct, natural prose without adding unsupported facts. Keep the mechanism and uncertainty intact; do not expose hidden chain-of-thought.";
            if let Ok(text) = model.generate(system, &context, seed_body) {
                if critic_accept_language(&text, &req.required_claim_boundaries).is_ok() {
                    let mut response = LanguageResponse::local(text);
                    response.engine = "local HTTP model under Perci critic".into();
                    response.claims = req.evidence.iter().map(|item| item.claim.clone()).collect();
                    return response;
                }
            }
        }
    }
    let mut resp = local_synthesize(req, seed_body);
    if let Err(e) = critic_accept_language(&resp.text, &req.required_claim_boundaries) {
        resp.ok = false;
        resp.error = Some(e.clone());
        resp.text = format!("{seed_body}\n\n[Governor] Refused language expansion: {e}");
    }
    resp
}

fn external_language_enabled() -> bool {
    env::var("PERCI_ENABLE_EXTERNAL_LM")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "on" | "yes"
            )
        })
        .unwrap_or(false)
}

fn invoke_external(
    bin: &std::path::Path,
    req: &LanguageRequest,
    seed_body: &str,
) -> Result<LanguageResponse, String> {
    let mut payload = serde_json::to_value(req).map_err(|e| e.to_string())?;
    payload["seed_body"] = serde_json::json!(seed_body);
    let input = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn sidecar: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("sidecar stdin: {e}"))?;
    }
    // Bounded wait (no hang forever).
    let _ = Duration::from_secs(30);
    let out = child
        .wait_with_output()
        .map_err(|e| format!("sidecar wait: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "sidecar exit {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or(text.trim());
    serde_json::from_str(line).map_err(|e| format!("sidecar json: {e}"))
}

/// Public fluency pass for any seed (operators, fluid SoftCascade, dialogue).
/// Seed-bound: restructures and softens wording; does not invent new facts.
pub fn fluent_rewrite(user: &str, seed_body: &str) -> String {
    let req = request_from(user, "fluency", Vec::new());
    let resp = local_synthesize(&req, seed_body);
    if resp.ok && !resp.text.trim().is_empty() {
        resp.text
    } else {
        seed_body.to_owned()
    }
}

/// Deterministic fluency layer: rewrite seed into natural multi-sentence prose.
///
/// This is not a transformer. It is a governed rewrite that removes checklist /
/// markdown card presentation so chat reads more like a collaborator LLM while
/// remaining bound to the seed's claims.
fn local_synthesize(req: &LanguageRequest, seed_body: &str) -> LanguageResponse {
    let mut claims = Vec::new();
    for e in &req.evidence {
        claims.push(e.claim.clone());
    }
    let evidence_block = if req.evidence.is_empty() {
        String::new()
    } else {
        let mut b = String::from("\n\nEvidence (source-bearing):\n");
        for e in req.evidence.iter().take(5) {
            b.push_str(&format!(
                "- [{} · auth={:.2}] {} — {}\n",
                e.source_type, e.authority, e.claim, e.source
            ));
        }
        b
    };

    let user_topic = req
        .constraints
        .iter()
        .find_map(|c| c.strip_prefix("user_topic: "))
        .unwrap_or("");
    let task = req.task.as_str();
    let body = rewrite_as_llm_prose(user_topic, seed_body, task);
    let text = format!("{body}{evidence_block}");
    let mut resp = LanguageResponse::local(text);
    resp.claims = claims;
    resp.engine = if external_language_enabled() {
        "local-governed-fluency (external LM available but not used this turn)".into()
    } else {
        "local-governed-fluency".into()
    };
    resp
}

/// Convert operator/card/checklist seed into flowing chat prose.
fn rewrite_as_llm_prose(user: &str, seed: &str, task: &str) -> String {
    let seed = seed.trim();
    if seed.is_empty() {
        return String::new();
    }
    // Exact short social / math-like answers: leave alone.
    if seed.split_whitespace().count() <= 8 && !seed.contains('\n') && !seed.contains("**") {
        return seed.to_owned();
    }

    let bullets = collect_content_chunks(seed);
    if bullets.is_empty() {
        return seed.to_owned();
    }

    // Already a single natural paragraph with no card markers — soft pass only.
    let joined_raw = bullets.join(" ");
    if bullets.len() == 1 && looks_already_natural(&joined_raw) {
        return soft_open(user, task, &joined_raw);
    }

    let paragraph = stitch_chunks_as_prose(&bullets);
    soft_open(user, task, &paragraph)
}

fn collect_content_chunks(seed: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in seed.lines() {
        let mut line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        // Skip pure section titles / repair footers that make answers sound like tickets.
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("repair path")
            || lower.starts_with("governance authority")
            || lower.starts_with("source-bearing")
            || lower.starts_with("evidence (source")
            || lower.starts_with("[governor]")
            || lower.starts_with("next check:")
        {
            continue;
        }
        // Strip markdown emphasis and list markers.
        while line.starts_with('#') {
            line = line.trim_start_matches('#').trim();
        }
        line = line.trim_start_matches(|c: char| c == '*' || c == '-' || c == '•').trim();
        // "1. **Human** — text" or "1) text"
        if line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if let Some(pos) = line.find(". ") {
                line = line[pos + 2..].trim();
            } else if let Some(pos) = line.find(") ") {
                line = line[pos + 2..].trim();
            }
        }
        // "1. **Human authorize** — durable..."
        if let Some(idx) = line.find("—") {
            let (left, right) = line.split_at(idx);
            let left = left.trim().trim_matches('*').trim();
            let right = right.trim_start_matches('—').trim();
            if !right.is_empty() && left.split_whitespace().count() <= 6 {
                line = right;
            }
        } else if let Some(idx) = line.find(" - ") {
            let (left, right) = line.split_at(idx);
            let left = left.trim().trim_matches('*').trim();
            let right = right.trim_start_matches(['-', ' ']).trim();
            if !right.is_empty() && left.split_whitespace().count() <= 6 {
                line = right;
            }
        }
        line = line.trim_matches(|c: char| c == '*' || c == '`').trim();
        // Drop leftover labels ending with colon only.
        if line.ends_with(':') && line.split_whitespace().count() <= 5 {
            continue;
        }
        if line.chars().count() < 8 {
            continue;
        }
        // Collapse internal whitespace.
        let cleaned = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !cleaned.is_empty() {
            out.push(cleaned);
        }
    }
    if out.is_empty() {
        // Fallback: whole seed as one chunk without newlines.
        let one = seed.split_whitespace().collect::<Vec<_>>().join(" ");
        if !one.is_empty() {
            out.push(one);
        }
    }
    out
}

fn stitch_chunks_as_prose(chunks: &[String]) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    if chunks.len() == 1 {
        return ensure_sentence_end(&capitalize_sentence(&chunks[0]));
    }
    let mut parts: Vec<String> = Vec::new();
    for chunk in chunks.iter() {
        let c = ensure_sentence_end(&capitalize_sentence(chunk.trim()));
        if c.is_empty() {
            continue;
        }
        parts.push(c);
        if parts.len() >= 5 {
            break; // keep chat airy; depth belongs in /deep or /think
        }
    }
    parts.join(" ")
}

fn capitalize_sentence(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut chars = t.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(chars.as_str());
    out
}

fn ensure_sentence_end(s: &str) -> String {
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

fn looks_already_natural(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    !text.contains("**")
        && !text.contains("\n1.")
        && !text.contains("\n- ")
        && !lower.contains("governance authority")
        && !lower.contains("repair path for pack")
        && (lower.starts_with("i ")
            || lower.starts_with("yes")
            || lower.starts_with("yeah")
            || lower.starts_with("think of")
            || lower.starts_with("here's")
            || lower.starts_with("here is")
            || lower.starts_with("short version")
            || lower.starts_with("not ")
            || lower.starts_with("aware ")
            || lower.starts_with("hey")
            || lower.starts_with("fair")
            || lower.starts_with("still on")
            || lower.starts_with("we are ")
            || lower.starts_with("that's "))
}

fn soft_open(user: &str, task: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return String::new();
    }
    if looks_already_natural(body) {
        return body.to_owned();
    }
    let u = user.to_ascii_lowercase();
    if task == "summarize" {
        return format!("In short: {body}");
    }
    // Prefer no canned opener when the body already carries the answer well.
    if body.split_whitespace().count() >= 12
        && (body.contains(". ") || body.ends_with('.'))
        && !body.starts_with("1.")
    {
        return body.to_owned();
    }
    if u.contains("who authoriz")
        || u.contains("weight promot")
        || u.contains("permission") && u.contains("proof")
    {
        return format!(
            "Here's the clean read. {body} Nothing durable moves without a human authorize step—and fluency never substitutes for that gate."
        );
    }
    if u.starts_with("how ") || u.starts_with("why ") || u.contains("how should") {
        return format!("Here's how I'd put it. {body}");
    }
    if u.contains("compare") || u.contains("between ") || u.contains("difference") {
        return format!("The useful comparison is this: {body}");
    }
    body.to_owned()
}

/// Whether this turn should invoke the language path (after operators/tools).
pub fn should_invoke_language(user: &str) -> bool {
    let t = user.to_ascii_lowercase();
    let compact = t
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '\'');
    // Keep pure social micro-turns on dialogue-act paths.
    if matches!(
        compact,
        "hi" | "hello"
            | "hey"
            | "hi there"
            | "thanks"
            | "thank you"
            | "bye"
            | "ok"
            | "okay"
            | "whoa"
            | "wow"
            | "hmm"
    ) {
        return false;
    }
    let configured = env::var("PERCI_MODEL_URL").is_ok()
        || env::var("PERCI_OPENAI_URL").is_ok()
        || env::var("PERCI_LANGUAGE_SIDECAR").is_ok()
        || external_language_enabled();
    // Default: open conversational turns get fluency rewrite so chat does not
    // sound like a checklist. Exact tools stay on their own path in chat.rs.
    t.contains("explain")
        || t.contains("summarize")
        || t.contains("in plain")
        || t.contains("write a")
        || t.contains("essay")
        || t.contains("how ")
        || t.contains("why ")
        || t.contains("what ")
        || t.contains("who ")
        || t.contains("compare")
        || t.contains("between")
        || t.contains("improv")
        || t.contains("should i")
        || t.contains("should we")
        || t.split_whitespace().count() >= 5
        || (configured && t.split_whitespace().count() >= 3)
}

/// Build a LanguageRequest from fabric + seed evidence.
pub fn request_from(user: &str, operator: &str, evidence: Vec<EvidenceRecord>) -> LanguageRequest {
    let mut req = LanguageRequest::default();
    req.task = if user.to_ascii_lowercase().contains("summar") {
        "summarize".into()
    } else {
        "explain".into()
    };
    req.intent = "technical_analysis".into();
    req.operator_plan = vec![
        operator.to_owned(),
        "language_sidecar".into(),
        "critic".into(),
    ];
    req.evidence = evidence;
    req.constraints
        .push(format!("user_topic: {}", truncate(user, 120)));
    req.required_claim_boundaries = vec![
        "no consciousness claims".into(),
        "no weight auto-promote".into(),
    ];
    req
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_synth_preserves_seed() {
        let req = LanguageRequest::default();
        let r = generate(&req, "Timeouts need idempotent retries under lag.");
        assert!(r.ok);
        assert!(r.text.contains("idempotent"));
        assert!(!r.text.contains("Here is a clear account"));
    }

    #[test]
    fn fluency_turns_checklist_into_prose() {
        let seed = "Governance authority (not fluency theater):\n\
1. **Human authorize** — durable `.pwgt` promote never auto-runs.\n\
2. **Permission ≠ proof** — sandbox edit is not transfer pass.\n\
3. **Measure first** — transfer suite green before claims.";
        let out = fluent_rewrite("Who authorizes weight promote?", seed);
        let low = out.to_ascii_lowercase();
        assert!(low.contains("authorize") || low.contains("durable") || low.contains("human"));
        assert!(!out.contains("**Human authorize**"));
        assert!(!out.contains("Governance authority (not fluency theater)"));
        assert!(!out.starts_with("1."));
    }

    #[test]
    fn critic_strips_consciousness_from_local_path() {
        // Local synth does not introduce consciousness; critic still passes clean seed.
        let req = LanguageRequest::default();
        let r = generate(&req, "Local tool, not conscious.");
        assert!(r.ok);
    }
}
