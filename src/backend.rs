use crate::cognitive::{CognitiveMatch, CognitiveWeights};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Language backend contract. The deterministic shell does not depend on a
/// specific model format or inference library.
pub trait LanguageBackend: Send {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String>;
    fn name(&self) -> &str;
}

/// Perci's built-in 200 MiB packed associative model.
///
/// This backend performs real integer-only weight inference. It is deliberately
/// described as an associative cognitive model—not as a transformer—because
/// its open-ended language is template/retrieval based while exact arithmetic
/// and geometry are delegated to deterministic tools.
pub struct CognitiveBackend {
    weights: CognitiveWeights,
    backend_name: String,
}

impl CognitiveBackend {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let weights = CognitiveWeights::load(path)?;
        let backend_name = format!(
            "perci-cognitive-v0.1 · {:.1} MiB · {} prototypes",
            weights.size_bytes() as f64 / (1024.0 * 1024.0),
            weights.prototype_count()
        );
        Ok(Self { weights, backend_name })
    }

    /// Load `PERCI_WEIGHTS` when set, otherwise try the bundled default path.
    pub fn discover() -> io::Result<Option<Self>> {
        let path = env::var_os("PERCI_WEIGHTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("models/perci-cognitive-v0.1.pwgt"));
        if !path.exists() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }
}

impl LanguageBackend for CognitiveBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let matched = self.weights.classify(user)?;
        Ok(render_cognitive_response(&matched, context, user))
    }

    fn name(&self) -> &str {
        &self.backend_name
    }
}

/// Useful fallback that keeps Perci operational if the weight file is removed.
pub struct DeterministicBackend;

impl LanguageBackend for DeterministicBackend {
    fn generate(&mut self, _system: &str, context: &[String], user: &str) -> io::Result<String> {
        let memory_note = if context.is_empty() {
            String::new()
        } else {
            format!(" I found {} relevant local memory item(s).", context.len())
        };
        Ok(format!(
            "I understand the request: \"{}\".{} Perci's cognitive weight file is not attached, so I am using the deterministic fallback. Exact math, geometry, memory, and routing commands remain available.",
            user.trim(), memory_note
        ))
    }
    fn name(&self) -> &str { "deterministic fallback" }
}

/// Runs any local model process implementing a tiny stdin/stdout protocol.
/// Set `PERCI_MODEL_CMD`, for example to a llama.cpp wrapper script. An external
/// command deliberately overrides the built-in weights so a future GGUF model
/// can be attached without changing the chat engine.
pub struct CommandBackend { command: String }

impl CommandBackend {
    pub fn from_env() -> Option<Self> {
        env::var("PERCI_MODEL_CMD").ok().filter(|s| !s.trim().is_empty()).map(|command| Self { command })
    }
}

impl LanguageBackend for CommandBackend {
    fn generate(&mut self, system: &str, context: &[String], user: &str) -> io::Result<String> {
        let mut child = if cfg!(windows) {
            Command::new("cmd").args(["/C", &self.command])
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?
        } else {
            Command::new("sh").args(["-c", &self.command])
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?
        };
        let payload = format!("SYSTEM:\n{system}\n\nMEMORY:\n{}\n\nUSER:\n{user}\n", context.join("\n---\n"));
        child.stdin.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "backend stdin unavailable"))?.write_all(payload.as_bytes())?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("model command exited with {}", output.status)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
    fn name(&self) -> &str { "external command" }
}

fn render_cognitive_response(matched: &CognitiveMatch, context: &[String], user: &str) -> String {
    let variant = matched.variant as usize;
    let memory_note = if context.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nI found {} relevant local memory item(s). I am treating them as context, not automatically as verified truth.",
            context.len()
        )
    };

    let body = match matched.label.as_str() {
        "greeting" => choose(variant, &[
            "Hello. I am Perci. Give me the problem, constraint, or idea you want examined.",
            "I am here. We can reason, inspect memory, solve exact math, or structure a build.",
            "Ready. I will separate what is known, inferred, and still unverified.",
        ]),
        "identity" => choose(variant, &[
            "I am Perci, a compact local neuro-symbolic assistant. My 200 MiB built-in model is a binary associative network, not a transformer. I combine learned routing and retrieval with exact arithmetic, geometry, local memory, and explicit governance.",
            "Perci is the interface and reasoning layer. The built-in weights recognize domains and retrieve trained response patterns; deterministic tools handle exact operations. I am useful, but I am not equivalent to a large pretrained language model.",
            "My strongest qualities are local operation, inspectability, compact memory, deterministic tools, and candid boundaries. My weakest area is unrestricted open-ended language generation.",
        ]),
        "english" => choose(variant, &[
            "This is English-language work. Preserve the intended meaning, remove ambiguity, tighten sentence structure, and choose concrete verbs. Provide the exact passage when you need a direct rewrite.",
            "For clear English, identify the subject, action, object, and intended tone. Then remove filler and resolve pronouns whose references are unclear.",
            "I can classify grammar, explain vocabulary, and structure concise prose. A direct rewrite requires the original text so the meaning is not invented.",
        ]),
        "logic" => choose(variant, &[
            "Reasoning pass: list the premises, separate assumptions, derive only supported consequences, test for contradiction, then state the conclusion with its uncertainty.",
            "Do not confuse correlation, possibility, and necessity. A valid conclusion must follow from the stated premises without importing an unstated rule.",
            "I would evaluate this as: evidence → assumptions → inference rule → conclusion → counterexample check.",
        ]),
        "math" => "This belongs in Perci's exact arithmetic path. State the expression with explicit operators, for example `calculate 144 divided by 12`, so the deterministic solver—not the associative model—produces the result.",
        "geometry" => "This belongs in Perci's exact geometry path. Name the shape and known measurements, for example `triangle area base 8 height 5` or `pythagorean sides 3 and 4`.",
        "memory" => "Use `remember that ...` to append a fact to local memory, or `recall ...` to search it. Retrieved memory is contextual evidence and can still be outdated or mistaken.",
        "code" => choose(variant, &[
            "Treat this as a code investigation: reproduce the behavior, isolate the smallest failing path, inspect the exact error, apply the smallest coherent patch, then run focused tests before broader integration.",
            "For systems code, make invariants explicit, validate every boundary, avoid hidden allocation in hot paths, and benchmark the release build rather than inferring performance from source alone.",
            "A reliable patch needs the relevant files, compiler output, and expected behavior. Without those, I can design the approach but should not claim the implementation is verified.",
        ]),
        "governance" => choose(variant, &[
            "Governance decision: establish authority, load the current origin state, classify the action as observe/sandbox/durable, define rollback, execute only within scope, then append evidence to the ledger.",
            "No durable mutation should compound from an unaligned or unverified state. Permission and successful validation are separate gates; both are required.",
            "I would block or sandbox the action until scope, authority, expected effect, validation test, and recovery path are explicit.",
        ]),
        "planning" => choose(variant, &[
            "Plan it in five layers: objective, constraints, dependencies, executable milestones, and acceptance tests. Each milestone should leave a usable verified state.",
            "Start with the smallest end-to-end vertical slice. Prove the core loop, measure it, then expand capability without changing several unknowns at once.",
            "Define what success looks like, what can fail, how failure is detected, and how the system returns to a known-good state.",
        ]),
        "explanation" => choose(variant, &[
            "A strong explanation begins with the central mechanism, then gives one concrete example, one boundary case, and the practical implication.",
            "Separate the name of the concept from what it actually does. Then connect cause to effect without skipping the intermediate step.",
            "I can explain this precisely once the target concept is explicit; otherwise a broad answer risks sounding confident while missing your intended question.",
        ]),
        "systems" => choose(variant, &[
            "A coherent Perci stack uses Lumen as the shell, Cortex as retrievable memory, Bitwork as the fast reflex layer, deterministic engines for exact reasoning, and governance gates before durable execution.",
            "The key boundary is between suggestion and mutation: Perci can classify and propose locally, while permission, tests, provenance, and rollback control what becomes durable.",
            "Use the language layer for interpretation, Bitwork for rapid routing, Cortex for rehydration, and exact tools for claims that can be mechanically verified.",
        ]),
        "science" => choose(variant, &[
            "Treat the claim scientifically: define measurable variables, identify the mechanism, state a falsifiable prediction, control alternatives, and distinguish observation from interpretation.",
            "A model is not evidence by itself. The useful chain is hypothesis → measurement → uncertainty → comparison → reproducible conclusion.",
            "State the units and boundary conditions. Many apparent contradictions disappear when two claims use different scales, definitions, or regimes.",
        ]),
        "creativity" => choose(variant, &[
            "A useful original concept combines two mechanisms that normally remain separate, then gives the user a clear action and visible consequence.",
            "I would push the idea toward a distinctive rule, not merely a new visual skin: define what changes, why it changes, and what the participant can discover.",
            "Originality becomes valuable when the concept remains understandable, interactive, and technically achievable.",
        ]),
        "comparison" => choose(variant, &[
            "Compare them using explicit criteria: capability, correctness, latency, memory, adaptability, interpretability, failure modes, and operating cost.",
            "Neither option is universally better. Identify the workload and the cost of a wrong answer, then choose the architecture whose tradeoffs fit that regime.",
            "Separate theoretical capability from measured implementation quality; a stronger design can still lose to a simpler system that is well optimized and verified.",
        ]),
        _ => choose(variant, &[
            "I recognize this as a general analysis request. Define the desired outcome and the evidence available, and I will separate facts, assumptions, options, and the next testable action.",
            "My initial position is to avoid scaling complexity before the core mechanism is demonstrated. Establish the smallest falsifiable version first.",
            "I can help examine the idea, but I should not manufacture certainty. The next useful step is to identify the claim that can be tested directly.",
        ]),
    };

    format!(
        "{}\n\n[Bitwork match: {} · score {} · overlap {} · input {} bytes]{}",
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
