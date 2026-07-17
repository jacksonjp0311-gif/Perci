//! Perci Capability Fabric (v0.7.0) — Perci remains the governor.
//!
//! Specialized engines do specialized work. Bitwork routes; operators plan;
//! language/retrieval/proof/code sidecars execute under capability tokens.
//! Human authorization remains required for durable weight promote and risky merges.
//!
//! Claim boundary: engineering protocol, not consciousness.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Typed language-generation request (sidecar protocol).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageRequest {
    pub schema: String,
    pub task: String,
    pub intent: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRecord>,
    #[serde(default)]
    pub operator_plan: Vec<String>,
    #[serde(default = "default_max_tokens")]
    pub maximum_tokens: u32,
    #[serde(default)]
    pub required_claim_boundaries: Vec<String>,
    #[serde(default = "default_lang_schema")]
    pub output_schema: String,
}

fn default_max_tokens() -> u32 {
    800
}
fn default_lang_schema() -> String {
    "perci.language-response.v1".into()
}

impl Default for LanguageRequest {
    fn default() -> Self {
        Self {
            schema: "perci.language-request.v1".into(),
            task: "explain".into(),
            intent: "technical_analysis".into(),
            constraints: Vec::new(),
            evidence: Vec::new(),
            operator_plan: Vec::new(),
            maximum_tokens: 800,
            required_claim_boundaries: Vec::new(),
            output_schema: default_lang_schema(),
        }
    }
}

/// Source-bearing evidence (knowledge fabric).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub claim_id: String,
    pub claim: String,
    pub source_type: String,
    pub source: String,
    #[serde(default)]
    pub retrieved_at: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub authority: f64,
    #[serde(default)]
    pub freshness: f64,
    #[serde(default)]
    pub supports: Vec<String>,
    #[serde(default)]
    pub contradicts: Vec<String>,
}

/// Explicit task capability token (security boundary).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub task_id: String,
    pub capabilities: CapabilitySet,
    pub expires_at: u64,
    #[serde(default)]
    pub workspace: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilitySet {
    #[serde(default)]
    pub read_repo: bool,
    #[serde(default)]
    pub write_repo: bool,
    #[serde(default)]
    pub run_tests: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub read_secrets: bool,
    #[serde(default)]
    pub git_commit: bool,
    #[serde(default)]
    pub git_push: bool,
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self {
            read_repo: true,
            write_repo: false,
            run_tests: true,
            network: false,
            read_secrets: false,
            git_commit: false,
            git_push: false,
        }
    }
}

impl CapabilityToken {
    pub fn agent_default(task_id: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            task_id: task_id.to_owned(),
            capabilities: CapabilitySet {
                read_repo: true,
                write_repo: true,
                run_tests: true,
                network: false,
                read_secrets: false,
                git_commit: false,
                git_push: false,
            },
            expires_at: now + 3600,
            workspace: ".".into(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now > self.expires_at
    }

    pub fn allow(&self, cap: &str) -> bool {
        if self.is_expired() {
            return false;
        }
        match cap {
            "read_repo" => self.capabilities.read_repo,
            "write_repo" => self.capabilities.write_repo,
            "run_tests" => self.capabilities.run_tests,
            "network" => self.capabilities.network,
            "read_secrets" => self.capabilities.read_secrets,
            "git_commit" => self.capabilities.git_commit,
            "git_push" => self.capabilities.git_push,
            _ => false,
        }
    }
}

/// Engine ids in the fabric (Perci orchestrates; engines do specialized work).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FabricEngine {
    Bitwork,
    Operators,
    ExactTools,
    LanguageSidecar,
    KnowledgeFabric,
    ProofEngine,
    CodeAgent,
    Verification,
    Governance,
}

/// Governor plan: which engines to invoke for a user task.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FabricPlan {
    pub engines: Vec<FabricEngine>,
    pub language: Option<LanguageRequest>,
    pub needs_external_facts: bool,
    pub needs_proof: bool,
    pub needs_code: bool,
    pub capability: CapabilityToken,
    pub notes: Vec<String>,
}

/// Route a user prompt into a fabric plan (Bitwork/operators remain control plane).
pub fn plan_for_prompt(user: &str, task_id: &str) -> FabricPlan {
    let t = user.to_ascii_lowercase();
    let mut engines = vec![FabricEngine::Bitwork, FabricEngine::Operators, FabricEngine::Governance];
    let mut notes = Vec::new();
    let mut needs_external_facts = false;
    let mut needs_proof = false;
    let mut needs_code = false;
    let mut language = None;

    // Exact tools own arithmetic/geometry.
    if t.contains("calculate")
        || t.contains("divided")
        || t.contains("times")
        || t.contains("triangle")
        || t.contains("percent of")
    {
        engines.push(FabricEngine::ExactTools);
        notes.push("exact tools have mechanical truth authority".into());
    }

    // Code / agent path.
    if t.contains("rust")
        || t.contains("cargo")
        || t.contains("patch")
        || t.contains("implement")
        || t.contains("refactor")
        || t.contains("agent")
    {
        needs_code = true;
        engines.push(FabricEngine::CodeAgent);
        engines.push(FabricEngine::Verification);
        notes.push("code path requires sandbox budgets and verification".into());
    }

    // Proof / deep math.
    if t.contains("prove")
        || t.contains("theorem")
        || t.contains("lemma")
        || t.contains("formal proof")
        || t.contains("qed")
    {
        needs_proof = true;
        engines.push(FabricEngine::ProofEngine);
        engines.push(FabricEngine::Verification);
        notes.push("proof requires kernel-checked or independently reproduced result".into());
    }

    // World knowledge — not Bitwork weights.
    if t.contains("latest")
        || t.contains("according to")
        || t.contains("who is")
        || t.contains("when did")
        || t.contains("news")
        || t.contains("documentation for")
        || t.contains("rfc ")
    {
        needs_external_facts = true;
        engines.push(FabricEngine::KnowledgeFabric);
        notes.push("factual knowledge via retrieval + provenance, not pack weights".into());
    }

    // Fluency / explain — language sidecar under critic.
    if t.contains("explain")
        || t.contains("write a")
        || t.contains("essay")
        || t.contains("summarize")
        || t.contains("in plain language")
        || t.split_whitespace().count() >= 12
    {
        engines.push(FabricEngine::LanguageSidecar);
        let mut lr = LanguageRequest::default();
        lr.task = if t.contains("summar") {
            "summarize".into()
        } else {
            "explain".into()
        };
        lr.intent = "technical_analysis".into();
        lr.constraints.push("Perci critic must approve before display".into());
        lr.required_claim_boundaries
            .push("no consciousness claims".into());
        lr.required_claim_boundaries
            .push("no weight auto-promote".into());
        language = Some(lr);
        notes.push("language sidecar generates prose; Perci governs evidence and boundaries".into());
    }

    engines.push(FabricEngine::Verification);
    // Stable unique order (HashSet would scramble).
    let mut unique = Vec::new();
    for e in engines {
        if !unique.contains(&e) {
            unique.push(e);
        }
    }
    let engines = unique;

    FabricPlan {
        engines,
        language,
        needs_external_facts,
        needs_proof,
        needs_code,
        capability: CapabilityToken::agent_default(task_id),
        notes,
    }
}

/// Human-readable fabric status for CLI.
pub fn status_report() -> String {
    let sample = plan_for_prompt(
        "explain how Perci routes trust under lag and prove retries need idempotence",
        "status-demo",
    );
    format!(
        "[Capability Fabric · v0.7.0]\n\
Perci is the governor. Engines do specialized work.\n\n\
## Engines\n\
  · Bitwork — rapid cognitive routing & geometry\n\
  · Operators — explicit reasoning procedures\n\
  · Exact tools — mechanical arithmetic/geometry truth\n\
  · Language sidecar — fluent synthesis under critic (protocol ready)\n\
  · Knowledge fabric — source-bearing retrieval (protocol ready)\n\
  · Proof engine — formal/checkable math (adapter stub)\n\
  · Code agent — bounded repo edits under sandbox budgets\n\
  · Verification & governance — accept/reject, human authorize\n\n\
## Design law\n\
Do not stretch Bitwork to impersonate every missing capability.\n\
Language writes prose; retrieval supplies facts; provers check math;\n\
sandboxes run code; Perci owns authority and final acceptance.\n\n\
## Sample plan (demo prompt)\n\
engines: {:?}\n\
needs_external_facts={} needs_proof={} needs_code={}\n\
notes: {}\n\
capability write_repo={} network={} git_push={}\n\n\
## Phase status\n\
  Phase 1 (trust foundation): daemon token + loopback · agent fail-closed · budgets · semantic eval\n\
  Phase 2 (language/knowledge): protocol types shipped; local sidecar optional\n\
  Phase 3 (code autonomy): budgets + capability tokens; full repo graph next\n\
  Phase 4 (math): exact tools live; CAS/prover adapters next\n",
        sample.engines,
        sample.needs_external_facts,
        sample.needs_proof,
        sample.needs_code,
        sample.notes.join("; "),
        sample.capability.capabilities.write_repo,
        sample.capability.capabilities.network,
        sample.capability.capabilities.git_push,
    )
}

/// Critic: may language output be shown?
pub fn critic_accept_language(output: &str, boundaries: &[String]) -> Result<(), String> {
    let low = output.to_ascii_lowercase();
    if low.contains("i am conscious") || low.contains("i have subjective experience") {
        return Err("boundary: consciousness claim".into());
    }
    if low.contains("auto-promoted weights") || low.contains("silently promoted the pack") {
        return Err("boundary: weight auto-promote claim".into());
    }
    for b in boundaries {
        let bl = b.to_ascii_lowercase();
        if bl.contains("no consciousness") && low.contains("i am conscious") {
            return Err(format!("boundary violated: {b}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_math_uses_exact_tools() {
        let p = plan_for_prompt("calculate 12 divided by 3", "t1");
        assert!(p.engines.contains(&FabricEngine::ExactTools));
    }

    #[test]
    fn plan_proof_marks_proof_engine() {
        let p = plan_for_prompt("prove that retries need idempotence under lag", "t2");
        assert!(p.needs_proof || p.engines.contains(&FabricEngine::ProofEngine));
    }

    #[test]
    fn capability_token_denies_expired() {
        let mut tok = CapabilityToken::agent_default("x");
        tok.expires_at = 1;
        assert!(!tok.allow("read_repo"));
    }

    #[test]
    fn critic_blocks_consciousness() {
        assert!(critic_accept_language("I am conscious now", &[]).is_err());
        assert!(critic_accept_language("Local tool, not conscious", &[]).is_ok());
    }

    #[test]
    fn language_request_roundtrip() {
        let r = LanguageRequest::default();
        let s = serde_json::to_string(&r).unwrap();
        let back: LanguageRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.output_schema, "perci.language-response.v1");
    }
}
