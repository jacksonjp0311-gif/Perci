use crate::cognitive::{CognitiveMatch, CognitiveWeights};
use serde::Serialize;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub trait LanguageBackend: Send {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String>;
    fn name(&self) -> &str;
    /// Optional multi-turn dialogue for natural continuity.
    fn set_dialogue_history(&mut self, _recent: &[(String, String)]) {}
    fn opening_insight(&self, _seed: u64) -> Option<String> {
        None
    }
    /// Bitwork geometry probe for cognition traces (operators still answer, but α/hops report).
    fn probe_cognition(&self, _user: &str) -> Option<CognitiveMatch> {
        None
    }
}

pub struct CognitiveBackend {
    weights: CognitiveWeights,
    backend_name: String,
    /// Recent turns for CTX bind into classify (session KV analog).
    recent: Vec<(String, String)>,
}

impl CognitiveBackend {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let weights = CognitiveWeights::load(path)?;
        let backend_name = format!(
            "Bitwork v{} | {:.1} MiB mapped | {} prototypes | {} concepts",
            weights.version(),
            weights.size_bytes() as f64 / (1024.0 * 1024.0),
            weights.prototype_count(),
            weights.concept_count(),
        );
        Ok(Self {
            weights,
            backend_name,
            recent: Vec::new(),
        })
    }

    pub fn discover() -> io::Result<Option<Self>> {
        let path = crate::cognitive::default_weight_path();

        if !path.exists() {
            return Ok(None);
        }

        Self::load(path).map(Some)
    }

    fn classify(&self, user: &str) -> io::Result<CognitiveMatch> {
        // Session context tokens = lightweight transformer KV analog for Bitwork.
        let ctx: Vec<String> = self
            .recent
            .iter()
            .rev()
            .take(3)
            .flat_map(|(u, _)| {
                u.split_whitespace()
                    .filter(|w| w.len() >= 4)
                    .map(|w| {
                        w.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                            .to_ascii_lowercase()
                    })
                    .filter(|w| w.len() >= 4)
                    .collect::<Vec<_>>()
            })
            .take(6)
            .collect();
        let refs: Vec<&str> = ctx.iter().map(|s| s.as_str()).collect();
        self.weights.classify_with_context(user, &refs)
    }
}

impl LanguageBackend for CognitiveBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let matched = self.classify(user)?;
        crate::emergence::record_match(user, &matched, "softcascade");
        Ok(render_cognitive_response_with_history(
            &matched,
            context,
            user,
            &self.recent,
        ))
    }

    fn set_dialogue_history(&mut self, recent: &[(String, String)]) {
        self.recent = recent.to_vec();
    }

    fn name(&self) -> &str {
        &self.backend_name
    }

    fn opening_insight(&self, seed: u64) -> Option<String> {
        self.weights.opening_insight(seed)
    }

    fn probe_cognition(&self, user: &str) -> Option<CognitiveMatch> {
        let m = self.classify(user).ok()?;
        crate::emergence::record_match(user, &m, "probe");
        Some(m)
    }
}

pub struct DeterministicBackend;

impl LanguageBackend for DeterministicBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let bits = crate::voice::weave_guidance(context, 1);
        let tip = bits
            .first()
            .map(|b| format!(" Practical angle: {b}"))
            .unwrap_or_default();
        Ok(format!(
            "I hear you: \"{}\". Weights aren't loaded, but exact tools and memory still work.{tip}",
            user.trim()
        ))
    }

    fn name(&self) -> &str {
        "deterministic fallback"
    }
}

pub struct CommandBackend {
    command: String,
}

#[derive(Serialize)]
struct BackendRequest<'a> {
    protocol: &'static str,
    system: &'a str,
    memory: &'a [String],
    user: &'a str,
}

impl CommandBackend {
    pub fn from_env() -> Option<Self> {
        env::var("PERCI_MODEL_CMD")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|command| Self { command })
    }
}

impl LanguageBackend for CommandBackend {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String> {
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", &self.command])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            Command::new("sh")
                .args(["-c", &self.command])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let payload = serde_json::to_vec(&BackendRequest {
            protocol: "perci-backend/1.0",
            system,
            memory: context,
            user,
        })
        .map_err(json_error)?;

        child
            .stdin
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "backend stdin unavailable"))?
            .write_all(&payload)?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "model command exited with {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ));
        }

        let response = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if response.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "model command returned an empty response",
            ));
        }

        Ok(response)
    }

    fn name(&self) -> &str {
        "external command"
    }
}

pub struct CompositeBackend {
    cognitive: Option<CognitiveBackend>,
    external: Option<CommandBackend>,
    backend_name: String,
    /// Multi-turn (user, assistant) pairs for natural continuity.
    recent: Vec<(String, String)>,
}

impl CompositeBackend {
    pub fn discover() -> io::Result<Self> {
        let cognitive = CognitiveBackend::discover()?;
        let external = CommandBackend::from_env();

        let backend_name = match (cognitive.as_ref(), external.as_ref()) {
            (Some(cognitive), Some(_)) => {
                format!("composite | {} + external model", cognitive.name())
            }
            (Some(cognitive), None) => cognitive.name().to_owned(),
            (None, Some(_)) => "external model | deterministic fallback".to_owned(),
            (None, None) => "deterministic fallback".to_owned(),
        };

        Ok(Self {
            cognitive,
            external,
            backend_name,
            recent: Vec::new(),
        })
    }

    pub fn set_recent(&mut self, recent: &[(String, String)]) {
        self.recent = recent.to_vec();
        if let Some(cognitive) = self.cognitive.as_mut() {
            cognitive.set_dialogue_history(recent);
        }
    }
}

impl LanguageBackend for CompositeBackend {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String> {
        let matched = self
            .cognitive
            .as_ref()
            .map(|cognitive| cognitive.classify(user))
            .transpose()?;

        if let Some(external) = self.external.as_mut() {
            let mut enriched = context.to_vec();
            if let Some(match_result) = matched.as_ref() {
                let skeleton = match_result.concept_skeleton(3);
                let frame = match_result.composition_frame(4);
                let mix = if skeleton.is_empty() {
                    String::new()
                } else {
                    format!("; mixture={}", skeleton.join(" | "))
                };
                let comp = if frame.is_empty() {
                    String::new()
                } else {
                    format!("; composition={}", frame.join(" · "))
                };
                enriched.push(format!(
                    "[Perci Bitwork hint: domain={} score={} overlap={} margin={}{}{}]; routing evidence, not truth]",
                    match_result.label,
                    match_result.score,
                    match_result.overlap,
                    match_result.margin,
                    mix,
                    comp,
                ));
            }
            // Brief dialogue memory for external models
            if !self.recent.is_empty() {
                let hist: String = self
                    .recent
                    .iter()
                    .rev()
                    .take(3)
                    .rev()
                    .map(|(u, a)| format!("User: {u}\nPerci: {a}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                enriched.push(format!("[Recent dialogue]\n{hist}"));
            }

            if let Ok(response) = external.generate(system, &enriched, user) {
                return Ok(response);
            }
        }

        if let Some(match_result) = matched.as_ref() {
            crate::emergence::record_match(user, match_result, "softcascade");
            return Ok(render_cognitive_response_with_history(
                match_result,
                context,
                user,
                &self.recent,
            ));
        }

        let mut fallback = DeterministicBackend;
        fallback.generate(system, context, user)
    }

    fn name(&self) -> &str {
        &self.backend_name
    }

    fn set_dialogue_history(&mut self, recent: &[(String, String)]) {
        self.set_recent(recent);
    }

    fn opening_insight(&self, seed: u64) -> Option<String> {
        self.cognitive
            .as_ref()
            .and_then(|cognitive| cognitive.opening_insight(seed))
    }

    fn probe_cognition(&self, user: &str) -> Option<CognitiveMatch> {
        self.cognitive
            .as_ref()
            .and_then(|cognitive| cognitive.probe_cognition(user))
    }
}

#[allow(dead_code)]
fn render_cognitive_response(matched: &CognitiveMatch, context: &[String], user: &str) -> String {
    render_cognitive_response_with_history(matched, context, user, &[])
}

/// Natural voice path with optional multi-turn history.
pub fn render_cognitive_response_with_history(
    matched: &CognitiveMatch,
    context: &[String],
    user: &str,
    recent: &[(String, String)],
) -> String {
    let variant = matched.variant as usize;
    let ul = user.to_ascii_lowercase();
    // Continuity: follow-up on a code/debug thread stays in code even if Bitwork drifts.
    let prior_code = recent.iter().rev().take(3).any(|(u, a)| {
        let t = format!("{u} {a}").to_ascii_lowercase();
        t.contains("error")
            || t.contains("cargo")
            || t.contains("rust")
            || t.contains("debug")
            || t.contains("compile")
            || a.contains("Reproduce")
            || a.contains("Re-run the same verify")
    });
    let followup = ul.contains("still")
        || ul.contains("failing")
        || ul.contains("what next")
        || ul.contains("same")
        || ul == "why?"
        || ul == "and then?";
    // Frustration + technical signal: prefer code craft over random mis-route (e.g. comparison).
    let label = if (crate::voice::detect_social(user) == crate::voice::SocialKind::Frustration
        && ul.split_whitespace().any(|w| {
            matches!(
                w,
                "bug" | "error" | "compile" | "cargo" | "rust" | "fail" | "panic" | "test"
            ) || w.contains("error")
        }))
        || (prior_code && followup)
        || (prior_code && (ul.contains("error") || ul.contains("bug") || ul.contains("fail")))
    {
        "code"
    } else if ul.contains("capable")
        || ul.contains("what can you")
        || ul.contains("what do you do")
        || ul.contains("abilities")
        || (ul.contains("what are you") && ul.contains("do"))
    {
        "identity" // richer capability copy below
    } else if ul.contains("trying to do") || ul.contains("what are we") || ul.contains("our goal") {
        "planning"
    } else {
        matched.label.as_str()
    };
    // Concept insight is seasoning for fluid prose — not a forced body dump.
    let concept = matched.insight.as_deref();
    // SoftCascade pack-alignment: when Bitwork primary is off-topic on trust/lag,
    // use a structural systems body so SoftCascade-only speech still transfers
    // without requiring the trust-systems operator (breakthrough path 2).
    let body = crate::auto_repairs::softcascade_trust_alignment_body(user)
        .unwrap_or_else(|| domain_body(label, variant));
    let woven = crate::voice::weave_guidance(context, 2);

    // Deep craft loops only when the user is clearly in debug/plan/code mode.
    // Open conversation uses fluid composition so we don't sound like a checklist.
    let text = if crate::reason::needs_reason_loop(label, user)
        && crate::voice::user_has_tech_signal(user)
    {
        let seed_body = concept.unwrap_or(body);
        let (reasoned, _score, _flags) =
            crate::reason::enhance_deep_answer(label, user, seed_body, &woven);
        if let Some((_, prev)) = recent.last() {
            if user.to_ascii_lowercase().contains("still")
                || user.to_ascii_lowercase().contains("again")
            {
                format!("Still on it. {reasoned}")
            } else if prev.len() > 40
                && user
                    .to_ascii_lowercase()
                    .split_whitespace()
                    .any(|w| matches!(w, "that" | "it" | "same"))
            {
                format!("Building on the last thread. {reasoned}")
            } else {
                reasoned
            }
        } else {
            reasoned
        }
    } else if crate::bridge::should_use_cascade(matched, user) {
        crate::bridge::compose_soft_cascade(user, matched, body, variant)
    } else {
        crate::voice::compose_reply(matched, user, body, context, recent)
    };

    // Final anti-generic guard: must bind the user's words when possible.
    let text = crate::voice::ensure_user_binding(user, &text, label, concept, recent);
    // Re-apply mixture / residual only when SoftCascade did not already weave them.
    // SoftCascade already applied length budget + [Cognition Trace].
    let used_cascade = crate::bridge::should_use_cascade(matched, user);
    let text = if used_cascade {
        text
    } else {
        let skeleton = matched.concept_skeleton(3);
        let text =
            crate::voice::weave_mixture_skeleton(user, &text, &skeleton, matched.variant as usize);
        let residual = matched.residual_skeleton(1);
        let text = if let Some(lat) = residual.first() {
            crate::voice::weave_residual_frame(&text, lat, matched.variant as usize)
        } else {
            text
        };
        // Non-cascade fluid path: still apply length scalar + cognition trace.
        let packet = crate::bridge::assemble(matched, user);
        let plan = crate::bridge::LengthPlan::from_bitwork(
            user,
            matched,
            &packet,
            crate::bridge::CognitionPath::Open,
        );
        let trimmed = crate::bridge::apply_word_budget(&text, plan.words);
        plan.envelope(&trimmed, false)
    };

    let mut text = text;
    if diagnostics_enabled() {
        let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
        let packet = crate::bridge::assemble(matched, user);
        text.push_str(&format!(
            "\n\n[Bitwork match: {} | score {} | overlap {} | mixture={} residual={} | attn={} | cascade={} frames={} | input {} bytes]",
            matched.label,
            matched.score,
            matched.overlap,
            matched.mixture.len(),
            residual_n,
            matched.primary_attention_pm,
            packet.rich,
            packet.frame_n,
            user.len()
        ));
    }
    text
}

fn domain_body(label: &str, variant: usize) -> &'static str {
    match label {
        "greeting" => choose(
            variant,
            &[
                "I can help with exact math, debug loops, memory, plans, or how this stack fits together.",
                "We can reason, check something exact, or map a next step.",
                "Tell me the problem, constraint, or idea — I'll keep it grounded.",
            ],
        ),
        "identity" => choose(
            variant,
            &[
                "I'm Perci — local Bitwork routing, exact math/geometry, intelligence packs, and selective memory. In Lumen I first-hop so GROK/NEMO/PHI can take deep work.",
                "I'm a local tool, not a cloud LLM and not conscious. I classify, do exact math, run short reason-loops, and remember only what you teach deliberately.",
                "Ask me to calculate, plan a fix, explain a system boundary, or store a short fact. For open chat fluency, use a full mind when keys allow — I cover the offline path.",
            ],
        ),
        "english" => choose(
            variant,
            &[
                "Keep the meaning, cut ambiguity, prefer concrete verbs, and drop filler.",
                "Name the subject, action, and object, then tighten until the sentence can't be misread.",
                "Rewrite for clarity without inventing a stronger claim than the original.",
            ],
        ),
        "logic" => choose(
            variant,
            &[
                "List premises, mark assumptions, only derive what follows, then hunt a counterexample.",
                "Keep correlation, possibility, and necessity in separate buckets.",
                "Walk evidence → assumptions → rule → conclusion → what would falsify it.",
            ],
        ),
        "math" => {
            "If you give numbers and an operation I support, I'll compute it exactly; otherwise I'll stay conceptual."
        }
        "geometry" => {
            "When the figure and measurements are clear, I can apply the exact formula; otherwise I'll ask for the missing piece."
        }
        "memory" => {
            "Memory is evidence you store on purpose — not automatic truth and not a hidden diary."
        }
        "code" => choose(
            variant,
            &[
                "Reproduce it, isolate the smallest failing path, read the exact error, patch one surface, then re-verify.",
                "Make the invariant explicit, check boundaries, and trust the compiler/tests over guesses.",
                "I need the failing output, the expected behavior, and a verify command when we go surgical.",
            ],
        ),
        "governance" => choose(
            variant,
            &[
                "Check authority, scope, rollback, and validation before anything durable.",
                "Permission and proof are different gates — neither replaces the other.",
                "If scope or recovery is fuzzy, sandbox or stop until it's explicit.",
            ],
        ),
        "planning" => choose(
            variant,
            &[
                "Objective, constraints, dependencies, small milestones, acceptance tests — in that order.",
                "Ship the smallest end-to-end slice first, measure it, then widen.",
                "Each milestone should leave a usable state you can roll back.",
            ],
        ),
        "explanation" => choose(
            variant,
            &[
                "Start with the mechanism, then one example, one edge case, and why it matters.",
                "Name the thing, say what it does, then connect cause to effect without skipping the middle.",
                "Define the target first so extra detail doesn't hide a category error.",
            ],
        ),
        "systems" => choose(
            variant,
            &[
                "Give each piece one job and one authority limit.",
                "Suggestions can be cheap; mutations need permission, tests, and a way back.",
                "Language interprets, Bitwork routes, cortex recalls, tools verify — keep those lanes clear.",
            ],
        ),
        "science" => choose(
            variant,
            &[
                "Define what you'd measure, the mechanism, a falsifiable prediction, then compare results with uncertainty.",
                "Models aren't evidence by themselves — hypothesis, measure, compare, reproduce.",
                "Check units and boundaries; a lot of 'contradictions' are scale mismatches.",
            ],
        ),
        "creativity" => choose(
            variant,
            &[
                "Combine two mechanisms that don't usually meet, then make the action and consequence obvious.",
                "Aim for a real rule of the system, not a reskin.",
                "Original only helps if someone can understand it and build it.",
            ],
        ),
        "comparison" => choose(
            variant,
            &[
                "Compare on capability, correctness, latency, cost of being wrong, and failure modes.",
                "Name the workload before crowning a winner.",
                "Separate the brochure from the measured implementation.",
            ],
        ),
        _ => choose(
            variant,
            &[
                "What outcome do you want, and what evidence do we already have?",
                "Let's find the smallest version we can test next.",
                "I won't fake certainty — what's the claim we can check directly?",
            ],
        ),
    }
}

fn diagnostics_enabled() -> bool {
    env::var("PERCI_DIAGNOSTICS")
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "on"))
        .unwrap_or(false)
}

fn choose<'a>(variant: usize, values: &'a [&'a str]) -> &'a str {
    values[variant % values.len()]
}

fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use crate::voice::weave_guidance;

    #[test]
    fn pack_guidance_is_woven() {
        let context = vec![
            "[Cortex evidence: src/main.rs:1-2 | abc] unrelated code".to_owned(),
            "[Pack: knowledge/packs/x.md | def] Prefer executable evidence over recollection."
                .to_owned(),
        ];
        let woven = weave_guidance(&context, 2);
        assert!(woven
            .iter()
            .any(|s| s.contains("Prefer executable evidence")));
    }
}
