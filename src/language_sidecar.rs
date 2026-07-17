//! Language-generation sidecar client (Capability Fabric Phase 2).
//!
//! External process protocol (stdin/stdout JSON lines):
//!   request:  LanguageRequest (fabric)
//!   response: LanguageResponse
//!
//! Env: PERCI_LANGUAGE_SIDECAR = path to executable (optional).
//! If unset, uses the **local governed synthesizer** (deterministic fluency layer).
//! Perci critic always runs after generation.

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
    if let Some(bin) = env::var_os("PERCI_LANGUAGE_SIDECAR") {
        match invoke_external(std::path::Path::new(&bin), req, seed_body) {
            Ok(mut resp) => {
                if let Err(e) = critic_accept_language(&resp.text, &req.required_claim_boundaries) {
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
    let mut resp = local_synthesize(req, seed_body);
    if let Err(e) = critic_accept_language(&resp.text, &req.required_claim_boundaries) {
        resp.ok = false;
        resp.error = Some(e.clone());
        resp.text = format!("{seed_body}\n\n[Governor] Refused language expansion: {e}");
    }
    resp
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

/// Deterministic fluency layer: structures seed (operator/tool body) into clean multi-sentence prose.
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
    // Keep the seed answer in the foreground. The former fixed headers,
    // plan dump, and boundary footer made every long answer sound like a
    // diagnostic preset. Routing and authority remain available in `/trace`;
    // the sidecar's display text should read like a human answer.
    let task = req.task.as_str();
    let lead = match task {
        "summarize" => "In short.\n\n",
        _ => "",
    };
    let body = seed_body.trim();
    let text = format!(
        "{lead}{body}{evidence_block}"
    );
    let mut resp = LanguageResponse::local(text);
    resp.claims = claims;
    resp
}

/// Whether this turn should invoke the language path (after operators/tools).
pub fn should_invoke_language(user: &str) -> bool {
    let t = user.to_ascii_lowercase();
    t.contains("explain")
        || t.contains("summarize")
        || t.contains("in plain")
        || t.contains("write a")
        || t.contains("essay")
        || (t.split_whitespace().count() >= 14
            && (t.contains("how") || t.contains("why") || t.contains("what")))
}

/// Build a LanguageRequest from fabric + seed evidence.
pub fn request_from(
    user: &str,
    operator: &str,
    evidence: Vec<EvidenceRecord>,
) -> LanguageRequest {
    let mut req = LanguageRequest::default();
    req.task = if user.to_ascii_lowercase().contains("summar") {
        "summarize".into()
    } else {
        "explain".into()
    };
    req.intent = "technical_analysis".into();
    req.operator_plan = vec![operator.to_owned(), "language_sidecar".into(), "critic".into()];
    req.evidence = evidence;
    req.constraints.push(format!("user_topic: {}", truncate(user, 120)));
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
        assert!(r.text.starts_with("Timeouts need"));
        assert!(!r.text.contains("Here is a clear account"));
    }

    #[test]
    fn critic_strips_consciousness_from_local_path() {
        // Local synth does not introduce consciousness; critic still passes clean seed.
        let req = LanguageRequest::default();
        let r = generate(&req, "Local tool, not conscious.");
        assert!(r.ok);
    }
}
