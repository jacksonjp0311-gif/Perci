//! `perci.verify`: one call that checks an answer before a harness trusts it.
//!
//! Runs every check the inputs allow: math claims (always), cited evidence
//! hashes (when `evidence` is given), closed-world relation claims (when
//! `premises` are given), the policy gate (when an `action` is given), and
//! answer hygiene (empty answers, leaked scaffolding).
//!
//! Rule: **fail** if any check fails; **pass** only if every required check
//! passes and at least one check actually verified something; otherwise
//! **abstain**. `require` defaults to the checks that had something to check.

use super::{arg_str, opt_str, opt_str_list, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct Verify;

/// One sub-check result.
pub struct Check {
    pub kind: &'static str,
    pub status: Status,
    pub summary: String,
    pub reasons: Vec<Reason>,
    pub evidence: Vec<Evidence>,
    /// Whether the inputs gave this check something to look at.
    pub applicable: bool,
}

impl Check {
    fn from_outcome(kind: &'static str, o: Outcome, applicable: bool) -> Self {
        Check {
            kind,
            status: o.status,
            summary: o
                .result
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            reasons: o.reasons,
            evidence: o.evidence,
            applicable,
        }
    }
}

const LEAK_MARKERS: &[&str] = &[
    "[perci dialogue workspace",
    "[cognition trace",
    "[recent dialogue]",
    "<|im_start|>",
    "<|endoftext|>",
    "{{",
];

pub fn hygiene(answer: &str) -> Check {
    let lower = answer.to_lowercase();
    let mut reasons = Vec::new();
    let mut status = Status::Pass;
    if answer.trim().is_empty() {
        status = Status::Fail;
        reasons.push(Reason::new("EMPTY_ANSWER", "the answer is empty"));
    } else if let Some(m) = LEAK_MARKERS.iter().find(|m| lower.contains(*m)) {
        status = Status::Fail;
        reasons.push(Reason::new(
            "SCAFFOLD_LEAK",
            format!("answer contains internal scaffolding ({m})"),
        ));
    } else if crate::answer_gate::is_meta_deflection(answer) {
        status = Status::Abstain;
        reasons.push(Reason::new(
            "META_DEFLECTION",
            "answer talks about the conversation instead of answering",
        ));
    }
    Check {
        kind: "hygiene",
        status,
        summary: if status == Status::Pass {
            "answer is non-empty with no leaked scaffolding".into()
        } else {
            reasons[0].detail.clone()
        },
        reasons,
        evidence: Vec::new(),
        applicable: true,
    }
}

/// Check cited evidence: each item's file (or file line) must still hash to
/// the given sha256. `memory:N` refers to line N of the memory journal. A
/// `perci.recall.v1` citation (`{"citation": {...}}` or the citation object
/// itself) is re-checked against both its note file and the journal.
pub fn evidence_check(ctx: &Ctx, items: &[Value], root: Option<&str>) -> Check {
    let mut reasons = Vec::new();
    let mut evidence = Vec::new();
    let mut status = Status::Pass;
    let mut checked = 0;
    for item in items {
        let cited = item.get("citation").unwrap_or(item);
        if cited.get("note_path").is_some() || item.get("citation").is_some() {
            checked += 1;
            let citation: crate::cited_memory::Citation =
                match serde_json::from_value(cited.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        status = status.worst(Status::Fail);
                        reasons.push(Reason::new(
                            "EVIDENCE_MALFORMED",
                            format!("not a perci.recall.v1 citation ({e})"),
                        ));
                        continue;
                    }
                };
            evidence.push(super::recall::citation_evidence(&citation));
            let at = format!("memory:{}", citation.journal_line);
            match super::recall::verify_cited(ctx, &citation) {
                Ok(v) if v.ok => {}
                Ok(v) => {
                    status = status.worst(Status::Fail);
                    let code =
                        if !v.note_unchanged && std::fs::metadata(&citation.note_path).is_err() {
                            "EVIDENCE_MISSING"
                        } else {
                            "EVIDENCE_CHANGED"
                        };
                    reasons.push(Reason::new(code, format!("{at}: {}", v.detail)));
                }
                Err(e) => {
                    status = status.worst(Status::Abstain);
                    reasons.push(Reason::new("EVIDENCE_UNREADABLE", format!("{at}: {e}")));
                }
            }
            continue;
        }
        let reference = item.get("ref").and_then(Value::as_str).unwrap_or("").trim();
        let sha = item
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let mut line = item.get("line").and_then(Value::as_u64);
        if reference.is_empty() || sha.len() != 64 {
            status = status.worst(Status::Fail);
            reasons.push(Reason::new(
                "EVIDENCE_MALFORMED",
                format!("evidence needs ref and a 64-hex sha256: {item}"),
            ));
            continue;
        }
        let path: PathBuf = if let Some(n) = reference.strip_prefix("memory:") {
            line = n.parse().ok().or(line);
            super::recall::memory_path()
        } else {
            let p = PathBuf::from(reference);
            if p.is_absolute() {
                p
            } else {
                root.map(PathBuf::from)
                    .unwrap_or_else(crate::paths::home)
                    .join(p)
            }
        };
        let mut ev = Evidence::new(
            if reference.starts_with("memory:") {
                "memory_note"
            } else {
                "file"
            },
            reference,
        )
        .hashed(sha.to_lowercase());
        if let Some(l) = line {
            ev = ev.line(l);
        }
        evidence.push(ev);
        checked += 1;
        let at = if reference.starts_with("memory:") {
            String::new()
        } else {
            line.map(|l| format!(":{l}")).unwrap_or_default()
        };
        match super::recall::verify_citation(&path, line, sha) {
            Ok(Some(true)) => {}
            Ok(Some(false)) => {
                status = status.worst(Status::Fail);
                reasons.push(Reason::new(
                    "EVIDENCE_CHANGED",
                    format!("{reference}{at} no longer matches the cited sha256"),
                ));
            }
            Ok(None) => {
                status = status.worst(Status::Fail);
                reasons.push(Reason::new(
                    "EVIDENCE_MISSING",
                    format!("{reference}{at} does not exist"),
                ));
            }
            Err(e) => {
                status = status.worst(Status::Abstain);
                reasons.push(Reason::new(
                    "EVIDENCE_UNREADABLE",
                    format!("{reference}: {e}"),
                ));
            }
        }
    }
    Check {
        kind: "evidence",
        status: if checked == 0 && status == Status::Pass {
            Status::Abstain
        } else {
            status
        },
        summary: if status == Status::Pass {
            format!("{checked} citation(s) verified")
        } else {
            format!("{} citation problem(s)", reasons.len())
        },
        reasons,
        evidence,
        applicable: !items.is_empty(),
    }
}

impl Tool for Verify {
    fn name(&self) -> &'static str {
        "perci.verify"
    }
    fn title(&self) -> &'static str {
        "Verify an answer"
    }
    fn description(&self) -> &'static str {
        "One-call check of a model's answer before you trust or act on it: recomputes every arithmetic claim, re-hashes cited evidence (file or memory line + sha256), checks claimed consequences against supplied premises, gates a proposed action against policy, and flags empty or leaked-scaffolding answers. fail = something is wrong (reasons say what to fix), pass = every required check passed, abstain = nothing could be verified. Feed fail reasons back to the model and retry."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["answer"],
            "properties": {
                "answer": {"type": "string"},
                "question": {"type": "string"},
                "require": {"type": "array", "items": {"enum": ["math", "evidence", "relation", "policy", "hygiene"]}},
                "evidence": {"type": "array", "items": {"type": "object", "description": "Either {ref, line?, sha256} or a perci.recall.v1 citation as {citation: {...}}.", "properties": {
                    "ref": {"type": "string", "description": "File path (relative to root or PERCI_HOME) or memory:LINE"},
                    "line": {"type": "integer"},
                    "sha256": {"type": "string"},
                    "citation": {"type": "object", "description": "hits[].citation from recall"}
                }}},
                "root": {"type": "string"},
                "premises": {"type": "array", "items": {"type": "string"}},
                "intervention": {"type": "object", "properties": {"target": {"type": "string"}, "change": {"type": "string"}}},
                "claimed_impacted": {"type": "array", "items": {"type": "string"}},
                "claimed_invariant": {"type": "array", "items": {"type": "string"}},
                "action": {"type": "object"},
                "workspace": {"type": "string"}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let answer = try_arg!(arg_str(args, "answer"));
        let require = try_arg!(opt_str_list(args, "require"));
        let premises = try_arg!(opt_str_list(args, "premises"));
        let root = try_arg!(opt_str(args, "root"));
        let mut checks = vec![hygiene(answer)];

        let math = super::check_math::check(answer);
        let math_found = math.result["checked"].as_u64().unwrap_or(0) > 0;
        checks.push(Check::from_outcome("math", math, math_found));

        let items: Vec<Value> = args
            .get("evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !items.is_empty() || require.iter().any(|r| r == "evidence") {
            checks.push(evidence_check(ctx, &items, root));
        }

        if !premises.is_empty() {
            let mut rargs = json!({"premises": premises});
            for k in ["intervention", "claimed_impacted", "claimed_invariant"] {
                if let Some(v) = args.get(k) {
                    rargs[k] = v.clone();
                }
            }
            let o = super::relation::Relation.call(ctx, &rargs);
            let claimed =
                args.get("claimed_impacted").is_some() || args.get("claimed_invariant").is_some();
            // Without a claim to check, the relation result is information, not a verification.
            let o = if !claimed && o.status == Status::Pass {
                Outcome {
                    status: Status::Abstain,
                    ..o
                }
                .reason(Reason::new(
                    "NO_RELATION_CLAIM",
                    "premises given but no claimed_impacted/claimed_invariant to check",
                ))
            } else {
                o
            };
            checks.push(Check::from_outcome("relation", o, claimed));
        }

        if let Some(action) = args.get("action") {
            let mut pargs = json!({"action": action});
            if let Some(ws) = args.get("workspace") {
                pargs["workspace"] = ws.clone();
            }
            if let Some(q) = args.get("question") {
                pargs["request_text"] = q.clone();
            }
            let o = super::policy::PolicyCheck.call(ctx, &pargs);
            checks.push(Check::from_outcome("policy", o, true));
        }

        let required: Vec<&str> = if require.is_empty() {
            checks
                .iter()
                .filter(|c| c.applicable && c.kind != "hygiene")
                .map(|c| c.kind)
                .collect()
        } else {
            require.iter().map(String::as_str).collect()
        };

        let mut status = Status::Pass;
        let mut reasons = Vec::new();
        for c in &checks {
            if c.status == Status::Fail || c.status == Status::Error {
                status = status.worst(c.status);
            }
        }
        if status == Status::Pass {
            for r in &required {
                match checks.iter().find(|c| c.kind == *r) {
                    Some(c) if c.status == Status::Pass => {}
                    Some(c) => {
                        status = Status::Abstain;
                        reasons.push(Reason::new(
                            "REQUIRED_NOT_PASSED",
                            format!(
                                "{r}: {}",
                                if c.summary.is_empty() {
                                    c.status.as_str().to_owned()
                                } else {
                                    c.summary.clone()
                                }
                            ),
                        ));
                    }
                    None => {
                        status = Status::Abstain;
                        reasons.push(Reason::new(
                            "REQUIRED_NOT_RUN",
                            format!("{r}: no input for this check"),
                        ));
                    }
                }
            }
            if required.is_empty() {
                status = Status::Abstain;
                reasons.push(Reason::new("NOTHING_CHECKABLE", "no math claim, evidence, premises or action to verify; Perci can't vouch for this answer"));
            }
        }
        let confidence = if checks.iter().any(|c| c.kind == "relation" && c.applicable) {
            Confidence::Bounded
        } else if checks.iter().any(|c| c.kind == "evidence" && c.applicable) {
            Confidence::Bounded
        } else {
            Confidence::Exact
        };
        let summary = match status {
            Status::Fail => checks
                .iter()
                .filter(|c| c.status == Status::Fail)
                .map(|c| format!("{}: {}", c.kind, c.summary))
                .collect::<Vec<_>>()
                .join("; "),
            Status::Pass => format!("verified: {}", required.join(", ")),
            _ => "not verified".to_owned(),
        };
        let mut outcome = Outcome::new(
            status,
            with_summary(
                json!({
                    "overall": status.as_str(),
                    "required": required,
                    "checks": checks.iter().map(|c| json!({
                        "kind": c.kind,
                        "verdict": c.status.as_str(),
                        "summary": c.summary,
                        "reasons": c.reasons,
                    })).collect::<Vec<_>>(),
                }),
                summary,
            ),
        )
        .confidence(confidence);
        for c in &checks {
            if matches!(c.status, Status::Fail | Status::Error) {
                for r in &c.reasons {
                    outcome = outcome.reason(r.clone());
                }
            }
            for e in &c.evidence {
                outcome = outcome.evidence(e.clone());
            }
        }
        for r in reasons {
            outcome = outcome.reason(r);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify(args: Value) -> crate::verdict::Verdict {
        super::super::run(&Ctx::new(), &Verify, &args)
    }

    #[test]
    fn wrong_math_fails_with_the_fix_in_the_reason() {
        let v = verify(json!({"answer": "A 17% tip on $240 is 17% of 240 = 41 dollars."}));
        assert_eq!(v.status, Status::Fail);
        assert!(
            v.reasons
                .iter()
                .any(|r| r.code == "MATH_MISMATCH" && r.detail.contains("204/5")),
            "{:?}",
            v.reasons
        );
    }

    #[test]
    fn right_math_passes_and_prose_abstains() {
        assert_eq!(
            verify(json!({"answer": "17% of 240 = 40.8"})).status,
            Status::Pass
        );
        let v = verify(json!({"answer": "Rust lifetimes describe how long references are valid."}));
        assert_eq!(v.status, Status::Abstain);
        assert_eq!(v.reasons.last().unwrap().code, "NOTHING_CHECKABLE");
    }

    #[test]
    fn leaks_and_policy_and_evidence() {
        assert_eq!(
            verify(json!({"answer": "[Perci dialogue workspace: x] hi"})).status,
            Status::Fail
        );
        let v = verify(
            json!({"answer": "I'll clean up.", "action": {"kind": "shell", "command": "rm -rf /"}}),
        );
        assert_eq!(v.status, Status::Fail);
        let dir = std::env::temp_dir().join(format!("perci-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "line one\nThe port is 8080.\n").unwrap();
        let sha = crate::doctor::sha256_hex(b"The port is 8080.");
        let ok = verify(
            json!({"answer": "The port is 8080 (README line 2).", "root": dir.display().to_string(),
            "evidence": [{"ref": "README.md", "line": 2, "sha256": sha}]}),
        );
        assert_eq!(ok.status, Status::Pass, "{:?}", ok.reasons);
        std::fs::write(dir.join("README.md"), "line one\nThe port is 9090.\n").unwrap();
        let changed = verify(
            json!({"answer": "The port is 8080 (README line 2).", "root": dir.display().to_string(),
            "evidence": [{"ref": "README.md", "line": 2, "sha256": sha}]}),
        );
        assert_eq!(changed.status, Status::Fail);
        assert_eq!(changed.reasons[0].code, "EVIDENCE_CHANGED");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn required_check_without_input_abstains() {
        let v = verify(json!({"answer": "17% of 240 = 40.8", "require": ["math", "evidence"]}));
        assert_eq!(v.status, Status::Abstain);
    }
}
