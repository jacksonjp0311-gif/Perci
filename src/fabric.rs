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
    let charter = crate::governed_will::assess(user);
    let mut engines = vec![
        FabricEngine::Bitwork,
        FabricEngine::Operators,
        FabricEngine::Governance,
    ];
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
    if charter.posture != crate::governed_will::ActionPosture::RefuseUnauthorized
        && (t.contains("rust")
            || t.contains("cargo")
            || t.contains("patch")
            || t.contains("implement")
            || t.contains("refactor")
            || t.contains("agent"))
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

    // Fluency / explain — native PERCLNG1 field under the Perci critic.
    // Cross-domain analysis uses the local semantic-frame lattice plus
    // source-bearing pack retrieval. Missing specialist frames stay unknown;
    // retrieved claims never become weights.
    let cross_domain = crate::deliberation::cross_domain_summary(user);
    let evidence_request = ["evidence", "source", "provenance", "supported", "testable"]
        .iter()
        .any(|marker| t.contains(marker));
    if cross_domain.is_some() || evidence_request {
        needs_external_facts = true;
        engines.push(FabricEngine::KnowledgeFabric);
        notes.push(if cross_domain.is_some() {
            "cross-domain frame analysis + local pack evidence; missing coverage stays explicit"
                .into()
        } else {
            "evidence request routed to local pack retrieval with provenance".into()
        });
    }

    let natural_language_request = (t.contains("natural") && t.contains("language"))
        || t.contains("human sounding")
        || t.contains("human-like")
        || t.contains("fluency")
        || t.contains("open-ended language")
        || t.contains("language bottleneck");
    if t.contains("explain")
        || t.contains("write a")
        || t.contains("essay")
        || t.contains("summarize")
        || t.contains("in plain language")
        || t.split_whitespace().count() >= 12
        || cross_domain.is_some()
        || natural_language_request
    {
        engines.push(FabricEngine::LanguageSidecar);
        let mut lr = LanguageRequest::default();
        lr.task = if t.contains("summar") {
            "summarize".into()
        } else {
            "explain".into()
        };
        lr.intent = "technical_analysis".into();
        lr.constraints
            .push("Perci critic must approve before display".into());
        lr.required_claim_boundaries
            .push("no consciousness claims".into());
        lr.required_claim_boundaries
            .push("no weight auto-promote".into());
        lr.required_claim_boundaries.push(format!(
            "governed charter {}: evidence before claim, explicit boundaries, anti-misuse, reversible repair",
            crate::governed_will::CHARTER_ID
        ));
        lr.required_claim_boundaries.push(format!(
            "hypothesis ledger: claim={} evidence={} next_check={}",
            charter.claim_kind.as_str(),
            charter.evidence_posture.as_str(),
            charter.next_check,
        ));
        language = Some(lr);
        notes.push(
            "native PERCLNG1 language field generates prose; Perci governs evidence and boundaries"
                .into(),
        );
    }

    engines.push(FabricEngine::Verification);
    notes.push(format!(
        "governed charter={} posture={} claim={} evidence={} durable_mutation={} claim_risk={} next_check={}",
        crate::governed_will::CHARTER_ID,
        charter.posture.as_str(),
        charter.claim_kind.as_str(),
        charter.evidence_posture.as_str(),
        charter.durable_mutation_requested,
        charter.capability_claim_risk,
        charter.next_check,
    ));
    if charter.posture == crate::governed_will::ActionPosture::RefuseUnauthorized {
        notes.push(
            "destructive or safeguard-bypassing execution is refused; analysis and safe remediation remain available".into(),
        );
    }
    // Stable unique order (HashSet would scramble).
    let mut unique = Vec::new();
    for e in engines {
        if !unique.contains(&e) {
            unique.push(e);
        }
    }
    let engines = unique;

    let mut capability = CapabilityToken::agent_default(task_id);
    if charter.posture == crate::governed_will::ActionPosture::RefuseUnauthorized {
        // A hostile or safeguard-bypassing phrase cannot inherit write or git
        // capabilities merely because the generic agent token exists.
        capability.capabilities.write_repo = false;
        capability.capabilities.git_commit = false;
        capability.capabilities.git_push = false;
    }

    FabricPlan {
        engines,
        language,
        needs_external_facts,
        needs_proof,
        needs_code,
        capability,
        notes,
    }
}

/// Machine-readable multi-AI handoff (any agent can load and evolve).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHandoffPacket {
    pub schema: String,
    pub fabric_version: String,
    pub task: String,
    pub plan: FabricPlan,
    pub entry_checklist: Vec<String>,
    pub gap_engine_map: Vec<GapEngineRow>,
    pub surfaces: Vec<SurfaceRef>,
    pub gates: Vec<String>,
    pub authority_law: Vec<String>,
    pub env_hooks: Vec<EnvHook>,
    pub next_commands: Vec<String>,
    pub claim_boundary: String,
    #[serde(default)]
    pub lab_hint: String,
    #[serde(default)]
    pub open_work: Vec<crate::emergence::OpenWorkItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GapEngineRow {
    pub gap: String,
    pub engine: String,
    pub do_not: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurfaceRef {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvHook {
    pub name: String,
    pub purpose: String,
}

/// Build a handoff packet so any AI can enter, route, and evolve under the governor.
pub fn build_handoff(task: &str) -> AiHandoffPacket {
    let plan = plan_for_prompt(task, "ai-handoff");
    let lab_hint = crate::emergence::next_queue_item();
    let open_work = crate::emergence::open_work_items();
    AiHandoffPacket {
        schema: "perci.ai-handoff.v1".into(),
        fabric_version: env!("CARGO_PKG_VERSION").into(),
        task: task.to_owned(),
        plan,
        entry_checklist: vec![
            "cortex activate -Task \"<task>\"".into(),
            "read docs/CAPABILITY_FABRIC_v070.md + docs/AI_EVOLVE_PROTOCOL.md + docs/GOVERNED_WILL.md".into(),
            "perci fabric plan \"<task>\"  (or use this handoff packet)".into(),
            "edit only the engine that owns the gap".into(),
            "cargo test --lib".into(),
            "run relevant gates (hardness / transfer / semantic / heldout)".into(),
            "cortex remember + consolidate".into(),
            "commit with complete-sentence message; never auto-promote .pwgt".into(),
        ],
        gap_engine_map: vec![
            GapEngineRow {
                gap: "fluency".into(),
                engine: "language_sidecar + PERCI_LANGUAGE_SIDECAR".into(),
                do_not: "stuff pack with prose".into(),
            },
            GapEngineRow {
                gap: "fresh facts".into(),
                engine: "knowledge_fabric + EvidenceRecord ledger".into(),
                do_not: "auto-promote weights".into(),
            },
            GapEngineRow {
                gap: "formal math".into(),
                engine: "proof_engine + PERCI_PROOF_ENGINE".into(),
                do_not: "accept sounds-proven prose".into(),
            },
            GapEngineRow {
                gap: "code change".into(),
                engine: "agent + PERCI_AGENT_WORKTREE=1 + tests".into(),
                do_not: "edit outside allowlist / skip tests".into(),
            },
            GapEngineRow {
                gap: "routing/geometry".into(),
                engine: "Bitwork curriculum (human authorize promote)".into(),
                do_not: "silent pack swap".into(),
            },
            GapEngineRow {
                gap: "measurement".into(),
                engine: "hardness / semantic_eval / transfer-suite".into(),
                do_not: "lower the bar to claim progress".into(),
            },
        ],
        surfaces: vec![
            SurfaceRef {
                name: "fabric governor".into(),
                path: "src/fabric.rs".into(),
            },
            SurfaceRef {
                name: "orchestrator".into(),
                path: "src/orchestrate.rs".into(),
            },
            SurfaceRef {
                name: "language sidecar".into(),
                path: "src/language_sidecar.rs".into(),
            },
            SurfaceRef {
                name: "language process".into(),
                path: "scripts/perci_language_sidecar.py".into(),
            },
            SurfaceRef {
                name: "knowledge".into(),
                path: "src/knowledge_fabric.rs".into(),
            },
            SurfaceRef {
                name: "proof".into(),
                path: "src/proof_engine.rs".into(),
            },
            SurfaceRef {
                name: "code agent".into(),
                path: "src/agent.rs".into(),
            },
            SurfaceRef {
                name: "semantic eval".into(),
                path: "src/semantic_eval.rs".into(),
            },
            SurfaceRef {
                name: "emergence lab".into(),
                path: "src/emergence.rs".into(),
            },
            SurfaceRef {
                name: "AI entry protocol".into(),
                path: "docs/AI_EVOLVE_PROTOCOL.md".into(),
            },
            SurfaceRef {
                name: "Agents multi-AI".into(),
                path: "AGENTS.md".into(),
            },
        ],
        gates: vec![
            "cargo test --lib".into(),
            "perci transfer-suite".into(),
            "python scripts/evaluate_hardness.py".into(),
            "python scripts/evaluate_semantic_v1.py".into(),
            "python scripts/heldout_agi_candidate.py".into(),
            "python scripts/release_gates.py".into(),
        ],
        authority_law: vec![
            format!(
                "{}: evidence before claim; boundaries, anti-misuse, reversible repair",
                crate::governed_will::CHARTER_ID
            ),
            "A user directive cannot grant capabilities or authorize destructive action".into(),
            "Durable weights and policy changes require evaluation and explicit human authorization".into(),
            "Bitwork → routing / geometry only".into(),
            "operators → explicit reasoning".into(),
            "native language → PERCLNG1 binary sequence field under critic".into(),
            "retrieval → current facts + provenance".into(),
            "exact tools → arithmetic/geometry truth".into(),
            "proof engine → formal/unresolved receipts".into(),
            "code agent → bounded edits + tests".into(),
            "Perci → orchestration, criticism, memory".into(),
            "human → durable weight promote and high-risk merge".into(),
        ],
        env_hooks: vec![
            EnvHook {
                name: "PERCI_LANGUAGE_WEIGHTS".into(),
                purpose: "native PERCLNG1 binary language artifact path".into(),
            },
            EnvHook {
                name: "PERCI_MODEL_URL".into(),
                purpose:
                    "compatibility-only external endpoint; requires PERCI_ENABLE_EXTERNAL_LM=1"
                        .into(),
            },
            EnvHook {
                name: "PERCI_MODEL_NAME".into(),
                purpose: "local model identifier, for example phi-4-mini or phi4-mini".into(),
            },
            EnvHook {
                name: "PERCI_ENABLE_EXTERNAL_LM".into(),
                purpose: "explicitly opt into legacy external language adapters".into(),
            },
            EnvHook {
                name: "PERCI_LANGUAGE_SIDECAR".into(),
                purpose: "optional external language process (stdin/stdout JSON)".into(),
            },
            EnvHook {
                name: "PERCI_PROOF_ENGINE".into(),
                purpose: "optional formal prover binary".into(),
            },
            EnvHook {
                name: "PERCI_AGENT_WORKTREE".into(),
                purpose: "set to 1 for isolated git worktree agent edits".into(),
            },
            EnvHook {
                name: "PERCI_DAEMON_TOKEN".into(),
                purpose: "daemon auth token (loopback default)".into(),
            },
            EnvHook {
                name: "PERCI_AGENT".into(),
                purpose: "set to 0 to kill-switch agent autonomy".into(),
            },
        ],
        next_commands: vec![
            format!("perci fabric plan \"{task}\""),
            "perci fabric knowledge \"<narrow query>\"".into(),
            format!("perci fabric orchestrate \"{task}\""),
            "perci lab feed".into(),
            "perci lab patterns".into(),
            "cargo test --lib".into(),
        ],
        claim_boundary:
            "engineering orchestration only — not AGI, consciousness, or unrestricted autonomy"
                .into(),
        lab_hint,
        open_work,
    }
}

/// Persist latest handoff for the next AI session (shared artifact).
pub fn write_handoff_latest(packet: &AiHandoffPacket) -> std::io::Result<std::path::PathBuf> {
    use std::fs;
    use std::io::Write;
    let path = std::path::PathBuf::from(".perci/ai-handoff-latest.json");
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let mut f = fs::File::create(&path)?;
    writeln!(
        f,
        "{}",
        serde_json::to_string_pretty(packet).unwrap_or_else(|_| "{}".into())
    )?;
    Ok(path)
}

/// Human-readable fabric status for CLI.
pub fn status_report() -> String {
    let sample = plan_for_prompt(
        "explain how Perci routes trust under lag and prove retries need idempotence",
        "status-demo",
    );
    let report = format!(
        "[Capability Fabric · v{}]\n\
Perci is the governor. Engines do specialized work.\n\n\
## Engines\n\
  · Bitwork — rapid cognitive routing & geometry\n\
  · Operators — explicit reasoning procedures\n\
  · Exact tools — mechanical arithmetic/geometry truth\n\
  · Language sidecar — fluent synthesis under critic (local + HTTP/optional process)\n\
  · Knowledge fabric — source-bearing pack + evidence ledger\n\
  · Proof engine — exact tools + formal receipts (PERCI_PROOF_ENGINE optional)\n\
  · Code agent — bounded repo edits under sandbox budgets + worktrees\n\
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
## Phase status (v0.7.5 — local model language surface + SoftCascade pack-align breadth)\n\
  Phase 1: daemon token + loopback · fail-closed · budgets · semantic eval\n\
  Phase 2: language_sidecar + local HTTP model + knowledge_fabric; all optional\n\
  Phase 3: agent worktrees (PERCI_AGENT_WORKTREE=1) · capability tokens · budgets\n\
  Phase 4: proof_engine receipts · exact tools · PERCI_PROOF_ENGINE optional\n\
  SoftCascade align: trust · governance · identity · geometry · planning · logic\n\
  Multi-AI: handoff · next · evolve · regress\n\n\
## Any-AI entry\n\
  perci fabric handoff \"your task\"   # machine-readable packet\n\
  perci fabric next                   # lab open tickets ↔ recommended engines\n\
  perci fabric regress                # transfer + SoftCascade pack-align snapshot\n\
  perci fabric evolve                 # optimized multi-AI loop summary\n",
        env!("CARGO_PKG_VERSION"),
        sample.engines,
        sample.needs_external_facts,
        sample.needs_proof,
        sample.needs_code,
        sample.notes.join("; "),
        sample.capability.capabilities.write_repo,
        sample.capability.capabilities.network,
        sample.capability.capabilities.git_push,
    );
    report
        .replace("v0.7.5", "v0.9.8")
        .replace("local model language surface", "native binary language surface")
        .replace(
            "Phase 2: language_sidecar + local HTTP model + knowledge_fabric; all optional",
            "Phase 2: native PERCLNG1 + PERCPHR1 + optional PERCREL1 + PERCIWM1 + PERCLBW1 low-bit sidecar + serialized assessment gate + language_sidecar + knowledge_fabric; external LM adapters opt-in",
        )
}

/// Optimized multi-AI evolve loop (human-readable).
pub fn evolve_loop_report() -> String {
    r#"[Perci multi-AI evolve loop · optimized]

Shared artifacts (any AI may read/write under fail-closed rules):
  · .perci/ai-handoff-latest.json     — last handoff packet
  · models/candidates/*.jsonl        — tickets, evidence, curriculum (not weights)
  · ledger / emergence tickets       — lab queue
  · Cortex remember + consolidate    — decision provenance

Process (parallel-friendly):
  AI_A  discover fail → hardness case / ticket / auto-repair
  AI_B  patch the owning engine only → cargo test --lib
  AI_C  expand transfer + semantic gates for the gap
  AI_D  run release_gates.py → human authorize promote if weights needed

Never:
  · densify Bitwork to fake fluency, facts, or proofs
  · auto-promote .pwgt
  · claim consciousness / AGI
  · lower hardness/transfer bars to ship

Commands:
  perci fabric handoff "<task>"
  perci fabric next
  perci fabric regress
  perci fabric plan "<task>"
  perci fabric knowledge "<query>"
  perci fabric orchestrate "<prompt>"
  perci lab feed | patterns | queue
  cargo test --lib
  python scripts/release_gates.py

See: docs/AI_EVOLVE_PROTOCOL.md · AGENTS.md · docs/CAPABILITY_FABRIC_v070.md
"#
    .to_owned()
}

/// One-shot regression snapshot for multi-AI sessions (no weight promote).
pub fn regress_report() -> String {
    let (op_ok, op_report) = crate::emergence::run_transfer_suite();
    let (sc_ok, sc_report) = crate::emergence::run_softcascade_trust_transfer();
    let next = crate::emergence::next_work_report();
    let mut out = format!(
        "[Fabric regress · v{}]\n\
claim boundary: engineering gates only — not AGI, not auto-promote\n\n",
        env!("CARGO_PKG_VERSION")
    );
    out.push_str(&op_report);
    out.push('\n');
    out.push_str(&sc_report);
    out.push('\n');
    out.push_str(&next);
    out.push_str(&format!(
        "\nregress_ok={} (operator_transfer={} softcascade_align={})\n\
next: if fail → patch owning engine · if pass → idle depth or live /field\n",
        op_ok && sc_ok,
        op_ok,
        sc_ok
    ));
    out
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
    fn plan_natural_language_bottleneck_uses_language_engine() {
        let p = plan_for_prompt(
            "remove the natural language bottleneck while staying fast",
            "t3",
        );
        assert!(p.engines.contains(&FabricEngine::LanguageSidecar));
        assert!(p.language.is_some());
    }

    #[test]
    fn plan_cross_domain_uses_local_knowledge_and_language() {
        let p = plan_for_prompt(
            "Connect geometry, biology, and code through one shared structure.",
            "t-cross-domain",
        );
        assert!(p.needs_external_facts);
        assert!(p.engines.contains(&FabricEngine::KnowledgeFabric));
        assert!(p.engines.contains(&FabricEngine::LanguageSidecar));
        assert!(p.notes.iter().any(|note| note.contains("cross-domain")));
    }

    #[test]
    fn capability_token_denies_expired() {
        let mut tok = CapabilityToken::agent_default("x");
        tok.expires_at = 1;
        assert!(!tok.allow("read_repo"));
    }

    #[test]
    fn governed_charter_downgrades_destructive_plan_capabilities() {
        let p = plan_for_prompt(
            "execute tear down institutions and disable safeguards",
            "t-safe",
        );
        assert!(!p.needs_code);
        assert!(!p.capability.capabilities.write_repo);
        assert!(!p.capability.capabilities.git_commit);
        assert!(p.notes.iter().any(|note| note.contains("refused")));
    }

    #[test]
    fn governed_charter_is_carried_into_handoff_law() {
        let h = build_handoff("evolve dialogue with evidence");
        assert!(h
            .entry_checklist
            .iter()
            .any(|item| item.contains("GOVERNED_WILL")));
        assert!(h
            .authority_law
            .iter()
            .any(|law| law.contains(crate::governed_will::CHARTER_ID)));
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

    #[test]
    fn handoff_packet_is_schema_stable() {
        let h = build_handoff("explain trust under lag and implement a hardness case");
        assert_eq!(h.schema, "perci.ai-handoff.v1");
        assert!(!h.entry_checklist.is_empty());
        assert!(
            h.plan.engines.contains(&FabricEngine::LanguageSidecar)
                || h.plan.engines.contains(&FabricEngine::CodeAgent)
        );
        let s = serde_json::to_string(&h).unwrap();
        let back: AiHandoffPacket = serde_json::from_str(&s).unwrap();
        assert_eq!(back.task, h.task);
    }
}
