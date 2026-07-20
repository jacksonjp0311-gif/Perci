use crate::backend::LanguageBackend;
use crate::cortex::CortexBridge;
use crate::deliberation::{self, Deliberation};
use crate::learning::InteractionLearner;
use crate::memory::MemoryStore;
use crate::personality::Personality;
use crate::reasoning::{try_solve_arithmetic, try_solve_geometry};
use crate::reflex::{ReflexRouter, Route};
use crate::voice::{self, natural_exact};
use std::env;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TURNS: usize = 48;

#[derive(Debug)]
pub struct ChatResponse {
    pub route: Route,
    pub text: String,
}

pub struct ChatEngine {
    personality: Personality,
    memory: MemoryStore,
    router: ReflexRouter,
    backend: Box<dyn LanguageBackend>,
    cortex: Option<CortexBridge>,
    /// Recent (user, assistant) turns for continuity.
    recent: Vec<(String, String)>,
    /// Optional disk session for CLI continuity across process exits.
    session: Option<crate::session::SessionStore>,
    /// Governed interaction evidence + safe dialogue preference adaptation.
    learning: Option<InteractionLearner>,
    /// Last bounded cognitive audit; operational trace, never hidden reasoning.
    last_deliberation: Option<Deliberation>,
    /// Session flag: richer backend plans for `/think` (never prefixes chat).
    verbose_cognition: bool,
}

impl ChatEngine {
    pub fn new(
        personality: Personality,
        memory: MemoryStore,
        backend: Box<dyn LanguageBackend>,
        cortex: Option<CortexBridge>,
    ) -> Self {
        Self {
            personality,
            memory,
            router: ReflexRouter,
            backend,
            cortex,
            recent: Vec::new(),
            session: None,
            learning: None,
            last_deliberation: None,
            verbose_cognition: false,
        }
    }

    /// Toggle session preference for richer **backend** plans (`/think on|off`).
    /// Never prefixes chat — only affects what `/think` reports after a turn.
    pub fn set_verbose_cognition(&mut self, on: bool) {
        self.verbose_cognition = on;
        crate::bridge::set_turn_verbose(on);
    }

    pub fn verbose_cognition(&self) -> bool {
        self.verbose_cognition
    }

    /// Last SoftCascade/Bitwork plan (backend-only; never mixed into chat replies).
    pub fn cognition_think(&self) -> String {
        if let Some(v) = crate::bridge::peek_last_verbose_trace() {
            return v;
        }
        let audit = self.deliberation_trace();
        format!(
            "[Cognition Trace · backend]\nNo plan stored yet this process.\n(session deep-plan={})\n\nLast operator audit:\n{audit}",
            self.verbose_cognition
        )
    }

    /// Attach persistent session and preload recent turns.
    pub fn with_session(mut self, store: crate::session::SessionStore) -> Self {
        if let Ok(turns) = store.load_recent() {
            self.recent = turns;
        }
        self.session = Some(store);
        self
    }

    pub fn with_learning(mut self, learner: InteractionLearner) -> Self {
        self.learning = Some(learner);
        self
    }

    pub fn learning_status(&self) -> String {
        self.learning
            .as_ref()
            .map(InteractionLearner::status_label)
            .unwrap_or_else(|| "disabled".to_owned())
    }

    pub fn learning_path(&self) -> Option<String> {
        self.learning
            .as_ref()
            .map(|learner| learner.events_path().display().to_string())
    }

    pub fn stage_teaching(&mut self, claim: &str) -> io::Result<String> {
        let learner = self.learning.as_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "interaction learning is disabled",
            )
        })?;
        learner.stage_teaching(claim)
    }

    /// Durable style memory: concise · deep · balanced.
    pub fn set_style_depth(&mut self, mode: &str) -> io::Result<String> {
        let learner = self.learning.as_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "interaction learning is disabled — style memory needs a profile path",
            )
        })?;
        let msg = learner.set_style_depth(mode)?;
        self.sync_style_to_bridge();
        Ok(msg)
    }

    pub fn style_label(&self) -> String {
        self.learning
            .as_ref()
            .map(|l| l.style_label().to_owned())
            .unwrap_or_else(|| "balanced".into())
    }

    fn sync_style_to_bridge(&self) {
        let depth = self
            .learning
            .as_ref()
            .map(|l| match l.style_label() {
                "concise" => 1u8,
                "deep" => 2u8,
                _ => 0u8,
            })
            .unwrap_or(0);
        crate::bridge::set_style_depth(depth);
    }

    pub fn session_path(&self) -> Option<String> {
        self.session
            .as_ref()
            .map(|s| s.path().display().to_string())
    }

    pub fn deliberation_trace(&self) -> String {
        self.last_deliberation
            .as_ref()
            .map(Deliberation::trace)
            .unwrap_or_else(|| "No cognitive operator has run in this process yet.".to_owned())
    }

    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    pub fn opening_insight(&self) -> String {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            ^ std::process::id() as u64;
        self.backend
            .opening_insight(seed)
            .unwrap_or_else(|| crate::voice::offline_opening_insight(seed))
    }

    pub fn personality(&self) -> &Personality {
        &self.personality
    }

    pub fn cortex_status(&self) -> String {
        self.cortex
            .as_ref()
            .map(CortexBridge::status_label)
            .unwrap_or_else(|| "not found".to_owned())
    }

    pub fn respond(&mut self, input: &str) -> io::Result<ChatResponse> {
        self.last_deliberation = None;
        // Flags only request deeper **backend** storage for /think — never chat prefixes.
        let (flag_verbose, clean) = crate::bridge::strip_cognition_flags(input);
        // Route a bounded repaired view so a dropped character or one
        // transposition cannot change the cognitive operator selected for the
        // turn. The repaired spelling is also what the answer can refer back
        // to, avoiding a second mismatch between routing and voice.
        let repaired = crate::text_normalize::repair_typos(clean.as_str());
        let input = repaired.as_str();
        crate::bridge::set_turn_verbose(flag_verbose || self.verbose_cognition);
        self.sync_style_to_bridge();
        let _flag_verbose = flag_verbose; // reserved: richer plan retention

        let route = self.router.route(input);
        match route {
            Route::Help => {
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Help,
                    text: help_text().into(),
                });
            }
            Route::MemoryWrite if !deliberation::is_session_only_instruction(input) => {
                return self.remember(input)
            }
            Route::MemoryWrite => {}
            Route::MemorySearch => return self.recall(input),
            Route::Chat | Route::Math | Route::Geometry => {}
        }

        if voice::is_teaching_recall(input) {
            let claims = self
                .learning
                .as_ref()
                .map(|learner| learner.recent_teaching_claims(5))
                .transpose()?
                .unwrap_or_default();
            let text = if claims.is_empty() {
                "You have not staged any teaching candidates in this learning record yet."
                    .to_owned()
            } else {
                format!(
                    "You taught me these pending candidates:\n- {}\nThey are recorded for review, not silently promoted into truth or weights.",
                    claims.join("\n- ")
                )
            };
            self.last_deliberation = Some(
                Deliberation::new("teaching-state-recall", text.clone())
                    .observed(format!("pending_candidates={}", claims.len()))
                    .confidence(1.0),
            );
            self.push_turn(input, &text);
            return Ok(ChatResponse {
                route: Route::Chat,
                text,
            });
        }

        // Natural teaching is the primary human interface. `/teach` remains a
        // transparent CLI shortcut, but ordinary conversation can stage the
        // same governed candidate without requiring command syntax.
        if let Some(claim) = voice::extract_teaching_claim(input) {
            let text = match self.stage_teaching(claim) {
                Ok(id) => format!(
                    "I staged that as knowledge candidate {id}: “{claim}.” It is pending review—not active truth or a weight change. Add a source or a test when you can."
                ),
                Err(error) => format!("I did not stage that claim: {error}."),
            };
            self.last_deliberation = Some(
                Deliberation::new("governed-teaching-stage", text.clone())
                    .observed("explicit natural-language teaching request")
                    .inferred("record as pending; do not promote automatically")
                    .confidence(0.99),
            );
            self.push_turn(input, &text);
            return Ok(ChatResponse {
                route: Route::Chat,
                text,
            });
        }

        let teaching_claims = self
            .learning
            .as_ref()
            .map(|learner| learner.recent_teaching_claims(5))
            .transpose()?
            .unwrap_or_default();
        if let Some(mut result) =
            deliberation::try_deliberate(input, &self.recent, &teaching_claims)
        {
            result = crate::operator_program::apply_dialogue_workspace_runtime(
                input,
                &self.recent,
                result,
            );
            let raw = result.answer.clone();
            // Operators answer, but Bitwork still probes geometry for the cognition trace
            // (α, residual hops, mixture domains) — otherwise traces stay α=0 forever.
            self.backend.set_dialogue_history(&self.recent);
            let bitwork = self.backend.probe_cognition(input);
            if let Some(ref m) = bitwork {
                crate::emergence::record_match(input, m, result.operator);
            }
            let control = crate::reasoning_controller::derive(
                input,
                &self.recent,
                bitwork.as_ref(),
                result.operator,
            );
            result
                .observations
                .push(format!("reasoning_controller={}", control.hint()));
            result.inferences.push(format!(
                "controller_steps={} · binary_state={:016x}",
                control.steps.join("→"),
                control.state_fingerprint
            ));
            // Capability Fabric: knowledge + native language under critic (governor retains control).
            let enriched = crate::orchestrate::enrich_answer(input, result.operator, &raw);
            let enriched = voice::shape_for_conversation(&enriched, input, &self.recent);
            let text = crate::bridge::envelope_with_bitwork(
                input,
                crate::bridge::CognitionPath::Operator,
                &[result.operator],
                result.operator,
                &enriched,
                false,
                bitwork.as_ref(),
            );
            result.answer = text.clone();
            crate::decision_trace::append(input, &result);
            self.last_deliberation = Some(result);
            self.push_turn(input, &text);
            crate::bridge::set_turn_verbose(false);
            return Ok(ChatResponse {
                route: Route::Chat,
                text,
            });
        }

        // Formal proof fabric path only (exact arithmetic still handled below with richer receipts).
        let lower_in = input.to_ascii_lowercase();
        if lower_in.contains("prove")
            || lower_in.contains("theorem")
            || lower_in.contains("formal proof")
        {
            if let Some(proof) = crate::orchestrate::try_proof_or_exact(input) {
                self.backend.set_dialogue_history(&self.recent);
                let bitwork = self.backend.probe_cognition(input);
                let text = crate::bridge::envelope_with_bitwork(
                    input,
                    crate::bridge::CognitionPath::ExactTool,
                    &["proof"],
                    "proof-engine",
                    &proof,
                    false,
                    bitwork.as_ref(),
                );
                self.last_deliberation = Some(
                    Deliberation::new("proof-engine", text.clone())
                        .observed("fabric formal proof path")
                        .inferred("kernel-checked or explicitly unresolved")
                        .confidence(0.9),
                );
                self.push_turn(input, &text);
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Chat,
                    text,
                });
            }
        }

        // Relational dialogue acts must be resolved before exact-tool parsing.
        // Natural phrases such as "defend the distinction" can contain token
        // fragments that resemble arithmetic operators without asking for math.
        let dialogue_act = voice::detect_dialogue_act(input);
        if let Some(text) = voice::dialogue_reply(
            dialogue_act,
            input,
            &self.recent,
            self.learning.as_ref().map(|learner| learner.profile()),
        ) {
            let mut deliberation = Deliberation::new("dialogue-act", text.clone())
                .observed(format!("dialogue_act={dialogue_act:?}"))
                .inferred("recent dialogue constrained the response")
                .confidence(0.90);
            deliberation = crate::operator_program::apply_dialogue_workspace_runtime(
                input,
                &self.recent,
                deliberation,
            );
            let text = deliberation.answer.clone();
            self.backend.set_dialogue_history(&self.recent);
            let bitwork = self.backend.probe_cognition(input);
            let text = crate::bridge::envelope_with_bitwork(
                input,
                crate::bridge::CognitionPath::Open,
                &["dialogue"],
                "dialogue-act",
                &text,
                false,
                bitwork.as_ref(),
            );
            deliberation.answer = text.clone();
            self.last_deliberation = Some(deliberation);
            self.push_turn(input, &text);
            crate::bridge::set_turn_verbose(false);
            return Ok(ChatResponse {
                route: Route::Chat,
                text,
            });
        }

        match try_solve_arithmetic(input) {
            Ok(Some(value)) => {
                let mut text = natural_exact("math", &value);
                let lower = input.to_ascii_lowercase();
                if lower.contains("which layer")
                    || lower.contains("exactly which layer")
                    || (lower.contains("tool") && lower.contains("authority"))
                    || (lower.contains("bitwork") && lower.contains("calculate"))
                    || lower.contains("metaphor")
                    || (lower.contains("authority") && lower.contains("different"))
                {
                    text.push_str(
                        " That number is exact-tool authority (checked rational arithmetic), not a metaphor or associative prototype vote — definitions and rules decide the value; Bitwork only routes.",
                    );
                }
                let mut deliberation = Deliberation::new("exact-arithmetic", text.clone())
                    .observed(format!("checked rational result={value}"))
                    .inferred(
                        "deterministic parser and checked rational arithmetic established the value",
                    )
                    .confidence(1.0);
                deliberation = crate::operator_program::apply_program_runtime(input, deliberation);
                let raw = deliberation.answer.clone();
                text = crate::bridge::envelope_light(
                    input,
                    crate::bridge::CognitionPath::ExactTool,
                    &["math"],
                    "exact-arithmetic",
                    &raw,
                    false,
                );
                deliberation.answer = text.clone();
                crate::decision_trace::append(input, &deliberation);
                self.last_deliberation = Some(deliberation);
                self.push_turn(input, &text);
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Math,
                    text,
                });
            }
            Ok(None) => {}
            Err(error) => {
                // Never surface raw parser errors for non-calculation prompts.
                // Explanatory math should have returned Ok(None); remaining
                // errors are true tool failures on calculation-shaped inputs.
                let text = format!("I couldn't complete that calculation: {error}");
                let deliberation = Deliberation::new("exact-arithmetic-error", text.clone())
                    .observed(error.to_string())
                    .uncertain("parser rejected the arithmetic form")
                    .confidence(0.4);
                crate::decision_trace::append(input, &deliberation);
                self.last_deliberation = Some(deliberation);
                self.push_turn(input, &text);
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Math,
                    text,
                });
            }
        }

        match try_solve_geometry(input) {
            Ok(Some(value)) => {
                let mut text = natural_exact("geometry", &value);
                let lower = input.to_ascii_lowercase();
                if lower.contains("bitwork")
                    || lower.contains("deterministic")
                    || lower.contains("which part")
                    || lower.contains("which layer")
                    || lower.contains("provenance")
                {
                    text.push_str(
                        " Bitwork helped classify and route the request; the numeric result came from deterministic geometry code, not from associative prototype voting.",
                    );
                }
                let mut deliberation = Deliberation::new("exact-geometry", text.clone())
                    .observed(format!("symbolic geometry result={value}"))
                    .inferred("deterministic geometry rules established the value")
                    .confidence(1.0);
                deliberation = crate::operator_program::apply_program_runtime(input, deliberation);
                let raw = deliberation.answer.clone();
                text = crate::bridge::envelope_light(
                    input,
                    crate::bridge::CognitionPath::ExactTool,
                    &["geometry"],
                    "exact-geometry",
                    &raw,
                    false,
                );
                deliberation.answer = text.clone();
                crate::decision_trace::append(input, &deliberation);
                self.last_deliberation = Some(deliberation);
                self.push_turn(input, &text);
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Geometry,
                    text,
                });
            }
            Ok(None) => {}
            Err(error) => {
                let text = format!("Geometry check failed: {error}");
                self.push_turn(input, &text);
                return Ok(ChatResponse {
                    route: Route::Geometry,
                    text,
                });
            }
        }

        // Fast social path: pure greetings/thanks/bye without pack load.
        let social = voice::detect_social(input);
        let lower = input.to_ascii_lowercase();
        if matches!(
            social,
            voice::SocialKind::Greeting
                | voice::SocialKind::HowAreYou
                | voice::SocialKind::Thanks
                | voice::SocialKind::Goodbye
                | voice::SocialKind::SmallTalk
        ) && !lower.contains("bug")
            && !lower.contains("error")
            && !lower.contains("cargo")
        {
            if let Some(text) = voice::social_reply(social, self.recent.len()) {
                let text = crate::bridge::envelope_light(
                    input,
                    crate::bridge::CognitionPath::Social,
                    &["greeting"],
                    "social-reflex",
                    &text,
                    false,
                );
                self.last_deliberation = Some(
                    Deliberation::new("social-reflex", text.clone())
                        .observed(format!("social_kind={social:?}"))
                        .confidence(0.97),
                );
                self.push_turn(input, &text);
                crate::bridge::set_turn_verbose(false);
                return Ok(ChatResponse {
                    route: Route::Chat,
                    text,
                });
            }
        }

        // Derive a small, inspectable turn record once so context collection and
        // response generation agree on the active act, referent, and depth.
        let workspace = crate::dialogue_workspace::DialogueWorkspace::derive(input, &self.recent);
        let context = if should_load_context(input) {
            self.collect_context(input, 4, context_budget(input))?
        } else {
            Vec::new()
        };

        let mut ctx = context;
        ctx.insert(
            0,
            format!("[Perci dialogue workspace: {}]", workspace.hint()),
        );
        if !self.recent.is_empty() {
            let hist = self
                .recent
                .iter()
                .rev()
                .take(3)
                .rev()
                .map(|(u, a)| format!("User: {u} | Perci: {a}"))
                .collect::<Vec<_>>()
                .join(" || ");
            ctx.insert(0, format!("[Recent dialogue] {hist}"));
        }

        self.backend.set_dialogue_history(&self.recent);
        let control =
            crate::reasoning_controller::derive(input, &self.recent, None, "fluid-associative");
        let generated = self
            .backend
            .generate(&self.personality.prompt, &ctx, input)?;
        let empty_generation = generated.trim().is_empty();
        let generated = if empty_generation {
            workspace.safe_fallback(input)
        } else {
            generated
        };
        let shaped = voice::shape_for_conversation(&generated, input, &self.recent);
        let shaped_empty = shaped.trim().is_empty();
        let generated = if shaped_empty {
            workspace.safe_fallback(input)
        } else {
            shaped
        };
        // Fluency pass: rewrite SoftCascade/checklist texture into chat prose.
        // Seed-bound; does not invent. External LM can replace this when opted in.
        let generated = if generated.split_whitespace().count() >= 10 {
            crate::language_sidecar::fluent_rewrite(input, &generated)
        } else {
            generated
        };
        // Style + anti-generic: fluid binding survives learned compression.
        let text = if let Some(learner) = &self.learning {
            let styled = voice::apply_learned_style(
                &generated,
                learner.profile().prefer_concise,
                learner.profile().avoid_structured_chat,
            );
            let aligned = voice::apply_profile_alignment(&styled, input, learner.profile());
            voice::ensure_user_binding(input, &aligned, "general", None, &self.recent)
        } else {
            voice::ensure_user_binding(input, &generated, "general", None, &self.recent)
        };

        let mut deliberation = Deliberation::new("fluid-associative", text.clone())
            .observed(format!("context_items={}", ctx.len()))
            .observed(format!("reasoning_controller={}", control.hint()))
            .inferred("fluid composition bound reply to user content under Bitwork routing")
            .inferred(format!(
                "controller_steps={} · binary_state={:016x}",
                control.steps.join("→"),
                control.state_fingerprint
            ))
            .uncertain("associative prose is not exact-tool evidence")
            .confidence(0.78);
        if empty_generation || shaped_empty {
            deliberation.observations.push(
                "response stage returned empty text; workspace fallback supplied text".into(),
            );
            deliberation.confidence = 0.45;
        }
        deliberation = crate::operator_program::apply_dialogue_workspace_runtime(
            input,
            &self.recent,
            deliberation,
        );
        let text = deliberation.answer.clone();
        crate::decision_trace::append(input, &deliberation);
        self.last_deliberation = Some(deliberation);

        self.push_turn(input, &text);
        crate::bridge::set_turn_verbose(false);
        Ok(ChatResponse {
            route: Route::Chat,
            text,
        })
    }

    fn push_turn(&mut self, user: &str, assistant: &str) {
        let previous = self.recent.last().cloned();
        self.recent
            .push((user.trim().to_string(), assistant.trim().to_string()));
        while self.recent.len() > MAX_TURNS {
            self.recent.remove(0);
        }
        if let Some(store) = &self.session {
            let _ = store.append(user, assistant);
        }
        if let Some(learner) = &mut self.learning {
            let _ = learner.observe(user, assistant, previous.as_ref());
        }
    }

    fn remember(&mut self, input: &str) -> io::Result<ChatResponse> {
        let content = strip_memory_prefix(input);
        self.memory.append_kind("note", content)?;

        let cortex_note = match self.cortex.as_mut() {
            Some(cortex) if cortex.ready() => match cortex.remember("note", content) {
                Ok(()) => " Also noted in Cortex when the daemon is warm.",
                Err(_) => " Local memory saved; Cortex sync was unavailable.",
            },
            Some(_) => " Local memory saved; Cortex still needs bootstrap.",
            None => " Saved to local memory.",
        };

        let text = format!("Got it — I'll remember: {content}.{cortex_note}");
        self.push_turn(input, &text);
        Ok(ChatResponse {
            route: Route::MemoryWrite,
            text,
        })
    }

    fn recall(&mut self, input: &str) -> io::Result<ChatResponse> {
        let query = strip_recall_prefix(input);
        let found = self.collect_context(query, 5, 700)?;

        let text = if found.is_empty() {
            "I don't have a matching local or pack note for that yet.".to_owned()
        } else {
            let lines: Vec<String> = found
                .iter()
                .take(4)
                .map(|f| {
                    f.split_once("] ")
                        .map(|(_, b)| b.trim().to_string())
                        .unwrap_or_else(|| f.clone())
                })
                .collect();
            format!("Here's what I can pull up:\n- {}", lines.join("\n- "))
        };
        self.push_turn(input, &text);
        Ok(ChatResponse {
            route: Route::MemorySearch,
            text,
        })
    }

    fn collect_context(
        &mut self,
        query: &str,
        local_limit: usize,
        cortex_budget: usize,
    ) -> io::Result<Vec<String>> {
        let mut context = self.memory.search(query, local_limit)?;

        let pack_limit = if cortex_budget >= 600 { 3 } else { 2 };
        if let Ok(hits) = crate::intel_packs::retrieve(query, pack_limit) {
            context.extend(crate::intel_packs::format_guidance(&hits));
        }

        if let Some(cortex) = self.cortex.as_mut() {
            if cortex.ready() {
                if let Ok(mut evidence) = cortex.retrieve(query, cortex_budget) {
                    context.append(&mut evidence);
                }
            }
        }

        context.dedup();
        Ok(context)
    }
}

fn should_load_context(input: &str) -> bool {
    let mode = env::var("PERCI_CORTEX_MODE")
        .unwrap_or_else(|_| "auto".to_owned())
        .to_ascii_lowercase();

    if mode == "off" || mode == "0" || mode == "false" {
        return false;
    }
    if mode == "always" || mode == "on" || mode == "1" || mode == "true" {
        return true;
    }

    if matches!(
        voice::detect_social(input),
        voice::SocialKind::Greeting
            | voice::SocialKind::Thanks
            | voice::SocialKind::Goodbye
            | voice::SocialKind::HowAreYou
            | voice::SocialKind::SmallTalk
    ) {
        return false;
    }

    let normalized = input
        .trim()
        .to_ascii_lowercase()
        .trim_matches(|character: char| !character.is_ascii_alphanumeric())
        .to_owned();

    const FAST_PATHS: &[&str] = &[
        "hello",
        "hello perci",
        "hi",
        "hi perci",
        "hey",
        "hey perci",
        "thanks",
        "thank you",
        "goodbye",
        "bye",
        "what can you do",
        "who are you",
        "what is your purpose",
    ];

    if FAST_PATHS.contains(&normalized.as_str()) {
        return false;
    }

    const DEEP_SIGNALS: &[&str] = &[
        "analyze",
        "architecture",
        "compare",
        "contradiction",
        "debug",
        "design",
        "evidence",
        "explain",
        "failure",
        "how ",
        "implement",
        "investigate",
        "memory",
        "plan",
        "prove",
        "reason",
        "repository",
        "science",
        "system",
        "test",
        "tradeoff",
        "verify",
        "why ",
        "bug",
        "error",
        "stuck",
    ];

    if DEEP_SIGNALS
        .iter()
        .any(|signal| normalized.contains(signal))
    {
        return true;
    }

    normalized.split_whitespace().count() >= 7 || normalized.len() >= 52
}

fn context_budget(input: &str) -> usize {
    let words = input.split_whitespace().count();
    if words >= 24 {
        return 1200;
    }
    if words >= 12 {
        return 950;
    }
    700
}

fn strip_memory_prefix(input: &str) -> &str {
    strip_prefixes(
        input,
        &[
            "remember that",
            "remember",
            "store",
            "save",
            "note that",
            "note",
        ],
    )
}

fn strip_recall_prefix(input: &str) -> &str {
    strip_prefixes(
        input,
        &[
            "recall",
            "search memory for",
            "search memory",
            "find memory for",
            "find memory",
            "what did i remember about",
            "what do you remember about",
        ],
    )
}

fn strip_prefixes<'a>(input: &'a str, prefixes: &[&str]) -> &'a str {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();

    for prefix in prefixes {
        if lower.starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start_matches([' ', ':']);
        }
    }

    trimmed
}

pub fn help_text() -> &'static str {
    "Commands:\n  /help               show commands\n  /status             show runtime status\n  /learning           show adaptive profile + pending evidence path\n  /teach <claim>      stage a governed knowledge candidate\n  /trace              show last audit (operator + program steps + critic)\n  /think [on|off]     show last backend cognition plan (never mixed into chat)\n  /field              geometry emergence (curriculum-authority view + laws)\n  /lab                self-improve queue (tickets + next work item)\n  /patterns           emergent laws from ledger (pattern intelligence)\n  /feed               all five intelligence channels status\n  /concise            durable shorter answers (style memory)\n  /deep               durable deeper answers (style memory)\n  /balanced           reset style memory to natural default\n  /intel              run transparent live intelligence probes\n  /cortex             show Cortex attachment status\n  /prompt             show personality prompt\n  /quit               exit\nFlags:\n  --verbose-cognition <msg>   ask + store richer backend plan (chat still clean)\n  think: <msg>                same\nCLI:\n  perci ask <msg>     one-shot with durable session continuity\n  perci learning      inspect governed interaction learning\n  perci teach <claim> stage a governed knowledge candidate\n  perci agent run <goal> [--merge-if-green] [--dry-run]\n  perci agent lab --from-hardness [--dry-run]\n  perci traces [n]        show recent decision traces\n  perci session path|clear\n  perci classify <msg>\n  perci intel         labels + margins + z-scores + similarity\nNatural tools:\n  calculate 12 divided by 5\n  triangle area base 8 height 5\n  remember that Perci uses governed memory\n  recall governed memory\nCognition:\n  chat = human answer only; /think = plan + prototype tree + self-critique\n  L = min(cap, ceil(B·(1+0.6α+1.2H_r+0.4log2(1+C)+I_u))) · style /concise|/deep\nLearning lanes:\n  conversation       session context + safe style adaptation\n  /teach <claim>     pending evidence requiring review\n  remember that ...  deliberate durable note\n  active weights     evaluated rebuild + explicit promotion only\nPerformance:\n  response headers show measured elapsed time\n  deep prompts may use packs + optional Cortex daemon"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_prefix_is_removed() {
        assert_eq!(
            strip_recall_prefix("recall the triangle formula"),
            "the triangle formula"
        );
    }

    #[test]
    fn memory_prefix_is_removed() {
        assert_eq!(
            strip_memory_prefix("remember that 2 plus 2 equals 4"),
            "2 plus 2 equals 4"
        );
    }

    #[test]
    fn greetings_use_fast_path() {
        assert!(!should_load_context("hello perci"));
        assert!(!should_load_context("what can you do"));
    }

    #[test]
    fn substantive_prompts_load_context() {
        assert!(should_load_context(
            "Analyze this architecture and identify its failure modes"
        ));
        assert!(should_load_context(
            "How should I debug a Rust ownership error?"
        ));
    }
}
