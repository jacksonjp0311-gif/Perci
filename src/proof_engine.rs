//! Proof / CAS adapter surface (Capability Fabric Phase 4).
//!
//! Authority model:
//! - Exact arithmetic/geometry remains with `reasoning` tools.
//! - Formal proofs require kernel-checked or independently reproduced results.
//! - Exploratory math must be labeled unresolved if not checked.

use crate::reasoning;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofReceipt {
    pub schema: String,
    pub status: ProofStatus,
    pub statement: String,
    pub engine: String,
    #[serde(default)]
    pub artifact: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    ExactToolChecked,
    KernelChecked,
    IndependentlyReproduced,
    UnresolvedArgument,
    Refused,
}

/// Attempt mathematical work under the authority model.
pub fn try_prove_or_compute(user: &str) -> Option<ProofReceipt> {
    let t = user.to_ascii_lowercase();

    // Exact tools first — mechanical truth.
    if let Ok(Some(result)) = reasoning::try_solve_arithmetic(user) {
        return Some(ProofReceipt {
            schema: "perci.proof-receipt.v1".into(),
            status: ProofStatus::ExactToolChecked,
            statement: user.to_owned(),
            engine: "perci.exact-tools".into(),
            artifact: result,
            notes: vec!["deterministic exact-tool authority".into()],
        });
    }

    // Optional external prover: PERCI_PROOF_ENGINE = path (stdin statement, stdout ok/fail).
    if t.contains("prove") || t.contains("theorem") || t.contains("formal") {
        if let Some(bin) = std::env::var_os("PERCI_PROOF_ENGINE") {
            match invoke_prover(std::path::Path::new(&bin), user) {
                Ok(art) => {
                    return Some(ProofReceipt {
                        schema: "perci.proof-receipt.v1".into(),
                        status: ProofStatus::KernelChecked,
                        statement: user.to_owned(),
                        engine: "external-proof-engine".into(),
                        artifact: art,
                        notes: vec!["external kernel receipt".into()],
                    });
                }
                Err(e) => {
                    return Some(ProofReceipt {
                        schema: "perci.proof-receipt.v1".into(),
                        status: ProofStatus::UnresolvedArgument,
                        statement: user.to_owned(),
                        engine: "external-proof-engine".into(),
                        artifact: String::new(),
                        notes: vec![format!("prover unavailable or failed: {e}")],
                    });
                }
            }
        }
        // No external prover: honest unresolved formal request.
        return Some(ProofReceipt {
            schema: "perci.proof-receipt.v1".into(),
            status: ProofStatus::UnresolvedArgument,
            statement: user.to_owned(),
            engine: "perci.proof-stub".into(),
            artifact: String::new(),
            notes: vec![
                "formal proof requires PERCI_PROOF_ENGINE or independent derivation".into(),
                "never accept 'sounds like a proof' as acceptance".into(),
            ],
        });
    }

    None
}

fn invoke_prover(bin: &std::path::Path, statement: &str) -> Result<String, String> {
    let out = Command::new(bin)
        .arg(statement)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    validate_external_receipt(&String::from_utf8_lossy(&out.stdout))
}

/// Validate the external proof boundary before assigning `KernelChecked`.
///
/// A zero exit code is only process success; it is not proof success. External
/// engines must emit a JSON receipt so the governor can verify the schema,
/// status, and non-empty artifact independently of the engine's exit code:
/// `{\"schema\":\"perci.proof-artifact.v1\",\"status\":\"kernel_checked\",\"artifact\":\"...\"}`.
fn validate_external_receipt(raw: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("proof receipt is not valid JSON: {e}"))?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if schema != "perci.proof-artifact.v1" {
        return Err(format!("unexpected proof receipt schema: {schema:?}"));
    }
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if status != "kernel_checked" && status != "verified" {
        return Err(format!("proof receipt is not kernel checked: {status:?}"));
    }
    let artifact = value
        .get("artifact")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if artifact.is_empty() {
        return Err("proof receipt artifact is empty".into());
    }
    Ok(artifact.to_owned())
}

/// Format receipt for human chat.
pub fn format_receipt(r: &ProofReceipt) -> String {
    match r.status {
        ProofStatus::ExactToolChecked => format!("Exact result (tool authority): {}", r.artifact),
        ProofStatus::KernelChecked => format!(
            "Formal proof accepted by kernel ({})::\n{}",
            r.engine, r.artifact
        ),
        ProofStatus::IndependentlyReproduced => {
            format!("Independently reproduced:\n{}", r.artifact)
        }
        ProofStatus::UnresolvedArgument => format!(
            "Unresolved formal argument.\nStatement: {}\nNotes: {}",
            r.statement,
            r.notes.join("; ")
        ),
        ProofStatus::Refused => format!("Proof refused: {}", r.notes.join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tool_receipt() {
        let r = try_prove_or_compute("calculate 12 divided by 3").expect("receipt");
        assert_eq!(r.status, ProofStatus::ExactToolChecked);
        assert!(r.artifact.contains('4') || r.artifact.contains("4"));
    }

    #[test]
    fn formal_without_engine_is_unresolved() {
        std::env::remove_var("PERCI_PROOF_ENGINE");
        let r = try_prove_or_compute("prove the fundamental theorem of arithmetic").expect("r");
        assert_eq!(r.status, ProofStatus::UnresolvedArgument);
    }

    #[test]
    fn external_receipt_requires_verified_artifact() {
        let ok = r#"{"schema":"perci.proof-artifact.v1","status":"kernel_checked","artifact":"qed: checked"}"#;
        assert_eq!(validate_external_receipt(ok).unwrap(), "qed: checked");
        let bad = r#"{"schema":"perci.proof-artifact.v1","status":"ok","artifact":"qed"}"#;
        assert!(validate_external_receipt(bad).is_err());
        let empty = r#"{"schema":"perci.proof-artifact.v1","status":"verified","artifact":""}"#;
        assert!(validate_external_receipt(empty).is_err());
    }
}
