use crate::cognitive::{CognitiveMatch, CognitiveWeights};
use serde::Serialize;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub trait LanguageBackend: Send {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String>;
    fn name(&self) -> &str;
}

pub struct CognitiveBackend {
    weights: CognitiveWeights,
    backend_name: String,
}

impl CognitiveBackend {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let weights = CognitiveWeights::load(path)?;
        let backend_name = format!(
            "perci-cognitive-v0.1 Â· {:.1} MiB Â· {} prototypes",
            weights.size_bytes() as f64 / (1024.0 * 1024.0),
            weights.prototype_count()
        );
        Ok(Self {
            weights,
            backend_name,
        })
    }

    pub fn discover() -> io::Result<Option<Self>> {
        let path = env::var_os("PERCI_WEIGHTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("models/perci-cognitive-v0.1.pwgt"));

        if !path.exists() {
            return Ok(None);
        }

        Self::load(path).map(Some)
    }

    fn classify(&self, user: &str) -> io::Result<CognitiveMatch> {
        self.weights.classify(user)
    }
}

impl LanguageBackend for CognitiveBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let matched = self.classify(user)?;
        Ok(render_cognitive_response(&matched, context, user))
    }

    fn name(&self) -> &str {
        &self.backend_name
    }
}

pub struct DeterministicBackend;

impl LanguageBackend for DeterministicBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let memory_note = if context.is_empty() {
            String::new()
        } else {
            format!(" I found {} bounded context item(s).", context.len())
        };

        Ok(format!(
            "I understand the request: \"{}\".{} Perci's cognitive weights are unavailable, so I am using the deterministic fallback. Exact tools and explicit memory remain available.",
            user.trim(),
            memory_note
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

/// Composition keeps Bitwork active when an external language model is attached.
///
/// The cognitive match becomes a bounded routing hint for the external backend.
/// If the external process fails, Perci falls back to its own cognitive response.
pub struct CompositeBackend {
    cognitive: Option<CognitiveBackend>,
    external: Option<CommandBackend>,
    backend_name: String,
}

impl CompositeBackend {
    pub fn discover() -> io::Result<Self> {
        let cognitive = CognitiveBackend::discover()?;
        let external = CommandBackend::from_env();

        let backend_name = match (cognitive.as_ref(), external.as_ref()) {
            (Some(cognitive), Some(_)) => {
                format!("composite Â· {} + external model", cognitive.name())
            }
            (Some(cognitive), None) => cognitive.name().to_owned(),
            (None, Some(_)) => "external model Â· deterministic fallback".to_owned(),
            (None, None) => "deterministic fallback".to_owned(),
        };

        Ok(Self {
            cognitive,
            external,
            backend_name,
        })
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
                enriched.push(format!(
                    "[Perci Bitwork hint: domain={} score={} overlap={}; use as routing evidence, not truth]",
                    match_result.label, match_result.score, match_result.overlap
                ));
            }

            if let Ok(response) = external.generate(system, &enriched, user) {
                return Ok(response);
            }
        }

        if let Some(match_result) = matched.as_ref() {
            return Ok(render_cognitive_response(match_result, context, user));
        }

        let mut fallback = DeterministicBackend;
        fallback.generate(system, context, user)
    }

    fn name(&self) -> &str {
        &self.backend_name
    }
}

fn render_cognitive_response(matched: &CognitiveMatch, context: &[String], user: &str) -> String {
    let variant = matched.variant as usize;
    let memory_note = if context.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nI found {} bounded context item(s). They are evidence, not automatically verified truth.",
            context.len()
        )
    };

    let body = match matched.label.as_str() {
        "greeting" => choose(
            variant,
            &[
                "Hello. I am Perci. Give me the problem, constraint, or idea you want examined.",
                "I am here. We can reason, inspect memory, solve exact math, or structure a build.",
                "Ready. I will separate what is known, inferred, and still unverified.",
            ],
        ),
        "identity" => choose(
            variant,
            &[
                "I am Perci, a compact local cognitive control layer. My 200 MiB model is a binary associative network, not a transformer. I combine learned routing, Cortex evidence, exact tools, explicit memory, and governance boundaries.",
                "Perci coordinates forms of intelligence. Bitwork recognizes domains, Cortex supplies bounded provenance-bearing context, deterministic tools handle exact operations, and an optional model can provide richer language.",
                "My strengths are local operation, inspectability, selective memory, exact tools, and candid limits. My built-in prose remains narrower than a pretrained language model.",
            ],
        ),
        "english" => choose(
            variant,
            &[
                "Preserve intended meaning, remove ambiguity, tighten sentence structure, and choose concrete verbs. Provide the passage when you need a direct rewrite.",
                "For clear English, identify the subject, action, object, and intended tone. Then remove filler and resolve unclear references.",
                "I can classify grammar, explain vocabulary, and structure concise prose. A direct rewrite requires the original text.",
            ],
        ),
        "logic" => choose(
            variant,
            &[
                "List the premises, separate assumptions, derive only supported consequences, test for contradiction, then state the conclusion with uncertainty.",
                "Do not confuse correlation, possibility, and necessity. A valid conclusion must follow without importing an unstated rule.",
                "Evaluate the chain as evidence â†’ assumptions â†’ inference rule â†’ conclusion â†’ counterexample check.",
            ],
        ),
        "math" => "This is mathematical. When the request matches a supported exact form, Perci executes it deterministically; otherwise it remains a conceptual language question.",
        "geometry" => "This is geometric. Exact formulas run only when the requested shape, operation, and required measurements are present.",
        "memory" => "Use `remember that ...` for an explicit append and `recall ...` for selective retrieval. Cortex evidence retains provenance and remains subordinate to current source and tests.",
        "code" => choose(
            variant,
            &[
                "Reproduce the behavior, isolate the smallest failing path, inspect the exact error, patch the smallest coherent surface, then run focused tests before integration.",
                "Make invariants explicit, validate boundaries, avoid hidden allocation in hot paths, and benchmark release builds rather than inferring performance from source.",
                "A reliable patch needs the relevant files, compiler output, and expected behavior. Without them, design the approach but do not claim verification.",
            ],
        ),
        "governance" => choose(
            variant,
            &[
                "Establish authority, load current origin state, classify observe/sandbox/durable scope, define rollback, validate, then append evidence.",
                "No durable mutation should compound from an unaligned or unverified state. Permission and successful validation are separate gates.",
                "Block or sandbox until scope, authority, expected effect, validation, and recovery are explicit.",
            ],
        ),
        "planning" => choose(
            variant,
            &[
                "Plan in five layers: objective, constraints, dependencies, executable milestones, and acceptance tests.",
                "Start with the smallest end-to-end vertical slice. Prove the loop, measure it, then expand one unknown at a time.",
                "Define success, failure detection, and the route back to a known-good state.",
            ],
        ),
        "explanation" => choose(
            variant,
            &[
                "Begin with the central mechanism, then give one concrete example, one boundary case, and the practical implication.",
                "Separate the concept's name from what it does, then connect cause to effect without skipping the intermediate step.",
                "Make the target concept explicit so the answer does not sound confident while missing the intended question.",
            ],
        ),
        "systems" => choose(
            variant,
            &[
                "A coherent stack uses Perci as coordinator, Cortex as selective memory, Bitwork as learned cognition, exact tools for mechanical truth, and governance before durable execution.",
                "The key boundary is suggestion versus mutation: Perci can classify and propose, while permission, tests, provenance, and rollback control durability.",
                "Use language for interpretation, Bitwork for cognitive hints, Cortex for rehydration, and exact tools for mechanically verifiable claims.",
            ],
        ),
        "science" => choose(
            variant,
            &[
                "Define measurable variables, identify the mechanism, state a falsifiable prediction, control alternatives, and distinguish observation from interpretation.",
                "A model is not evidence by itself. Use hypothesis â†’ measurement â†’ uncertainty â†’ comparison â†’ reproducible conclusion.",
                "State units and boundary conditions; many contradictions are scale or definition mismatches.",
            ],
        ),
        "creativity" => choose(
            variant,
            &[
                "Combine mechanisms that normally remain separate, then give the user a clear action and visible consequence.",
                "Push toward a distinctive rule rather than a visual reskin: define what changes, why, and what can be discovered.",
                "Originality becomes valuable when the concept remains understandable, interactive, and achievable.",
            ],
        ),
        "comparison" => choose(
            variant,
            &[
                "Compare capability, correctness, latency, memory, adaptability, interpretability, failure modes, and operating cost.",
                "Identify the workload and cost of a wrong answer, then choose the architecture whose tradeoffs fit that regime.",
                "Separate theoretical capability from measured implementation quality.",
            ],
        ),
        _ => choose(
            variant,
            &[
                "Define the desired outcome and available evidence; I will separate facts, assumptions, options, and the next testable action.",
                "Avoid scaling complexity before demonstrating the core mechanism. Establish the smallest falsifiable version first.",
                "I can examine the idea, but should not manufacture certainty. Identify the claim that can be tested directly.",
            ],
        ),
    };

    format!(
        "{}\n\n[Bitwork match: {} Â· score {} Â· overlap {} Â· input {} bytes]{}",
        body,
        matched.label,
        matched.score,
        matched.overlap,
        user.len(),
        memory_note
    )
}

fn choose<'a>(variant: usize, values: &'a [&'a str]) -> &'a str {
    values[variant % values.len()]
}

fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
