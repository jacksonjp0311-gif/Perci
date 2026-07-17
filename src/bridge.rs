//! SoftCascade bridge — LLM-like multi-hypothesis answers without transformer latency.
//!
//! # Math (transformer jobs without matmuls)
//!
//! | Job | SoftCascade |
//! |-----|-------------|
//! | Multi-head attention | soft-α mixture + residual hops from Bitwork |
//! | Value projection | Willshaw concept insights + semantic frame lattice |
//! | Residual stream | hop-1/2 ANDNOT supports already on CognitiveMatch |
//! | Soft binding | VSA composition frame |
//! | Decode | structured compose (not token sampling) |
//!
//! All paths are integer/string work on already-classified evidence — target
//! warm path remains interactive (ms-scale after pack load).

use crate::cognitive::CognitiveMatch;
use crate::deliberation;
use std::cell::{Cell, RefCell};

thread_local! {
    /// Per-turn verbose cognition (set by chat for --verbose-cognition / session /think on).
    static TURN_VERBOSE: Cell<bool> = Cell::new(false);
    /// Last verbose plan text for `/think` without re-running classify.
    static LAST_VERBOSE: RefCell<Option<String>> = RefCell::new(None);
    /// Style depth from dialogue profile: 0 balanced · 1 concise · 2 deep.
    static STYLE_DEPTH: Cell<u8> = Cell::new(0);
    /// Last self-critique note for /think.
    static LAST_CRITIQUE: RefCell<Option<String>> = RefCell::new(None);
    /// Last visual prototype tree for /think.
    static LAST_TREE: RefCell<Option<String>> = RefCell::new(None);
    /// Load-bearing premise from the previous human-facing answer (session thought continuity).
    static LAST_PREMISE: RefCell<Option<String>> = RefCell::new(None);
}

/// 0 = balanced, 1 = concise, 2 = deep (from `/concise` · `/deep` style memory).
pub fn set_style_depth(depth: u8) {
    STYLE_DEPTH.with(|c| c.set(depth.min(2)));
}

pub fn style_depth() -> u8 {
    STYLE_DEPTH.with(|c| c.get())
}

/// Set whether this turn's SoftCascade/operator envelope uses the verbose plan.
pub fn set_turn_verbose(verbose: bool) {
    TURN_VERBOSE.with(|c| c.set(verbose));
}

pub fn turn_verbose() -> bool {
    TURN_VERBOSE.with(|c| c.get())
}

fn store_last_verbose(plan: &LengthPlan) {
    let mut block = plan.verbose_trace();
    if let Some(tree) = peek_tree() {
        block.push_str("\n\n");
        block.push_str(&tree);
    }
    if let Some(crit) = peek_critique() {
        block.push_str("\n\n");
        block.push_str(&crit);
    }
    LAST_VERBOSE.with(|c| {
        *c.borrow_mut() = Some(block);
    });
}

fn store_critique(report: &CritiqueReport) {
    LAST_CRITIQUE.with(|c| {
        *c.borrow_mut() = Some(report.format_backend());
    });
}

fn store_tree(tree: &str) {
    LAST_TREE.with(|c| {
        *c.borrow_mut() = Some(tree.to_owned());
    });
}

fn peek_critique() -> Option<String> {
    LAST_CRITIQUE.with(|c| c.borrow().clone())
}

fn peek_tree() -> Option<String> {
    LAST_TREE.with(|c| c.borrow().clone())
}

/// Peek last verbose cognition block (for `/think`).
pub fn peek_last_verbose_trace() -> Option<String> {
    LAST_VERBOSE.with(|c| c.borrow().clone())
}

/// Store the load-bearing first sentence of the last reply (session continuity).
pub fn remember_premise(answer: &str) {
    let premise = first_sentence_premise(answer);
    if premise.chars().count() >= 20 {
        LAST_PREMISE.with(|c| *c.borrow_mut() = Some(premise));
    }
}

fn peek_premise() -> Option<String> {
    LAST_PREMISE.with(|c| c.borrow().clone())
}

fn first_sentence_premise(text: &str) -> String {
    let t = text.trim();
    // Skip any accidental cognition lines if present.
    let t = t
        .lines()
        .find(|l| !l.trim_start().starts_with('[') && l.split_whitespace().count() >= 4)
        .unwrap_or(t);
    let end = t
        .find(['.', '!', '?'])
        .map(|i| i + 1)
        .unwrap_or_else(|| t.len().min(180));
    t[..end].trim().to_owned()
}

/// Evidence packet assembled for one soft-cascade reply.
#[derive(Debug, Clone)]
pub struct BridgePacket {
    /// Primary insight / lead claim.
    pub lead: Option<String>,
    /// Supporting facets ordered by attention (same-geometry mixture).
    pub supports: Vec<String>,
    /// Residual-stream facets (ANDNOT hops) — kept separate for hop-aware weave.
    pub residual_supports: Vec<String>,
    /// Semantic-frame clauses activated from the open lattice.
    pub frames: Vec<String>,
    /// Mechanism lines from activated frames (when distinct).
    pub mechanisms: Vec<String>,
    /// VSA role–filler tags from encode (soft binding → decode).
    pub composition: Vec<String>,
    /// Whether evidence is rich enough to replace stock domain_body prose.
    pub rich: bool,
    /// Contested geometry (low margin) → force multi-facet voice.
    pub contested: bool,
    /// Telemetry.
    pub attention_primary_pm: u16,
    pub mixture_n: usize,
    pub residual_n: usize,
    pub frame_n: usize,
}

/// Build the soft-cascade packet from a Bitwork match + user text.
pub fn assemble(matched: &CognitiveMatch, user: &str) -> BridgePacket {
    let mut supports: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    let push = |out: &mut Vec<String>, seen: &mut Vec<String>, s: &str| {
        let t = s.trim();
        if t.chars().count() < 16 || t.chars().count() > 180 {
            return;
        }
        let low = t.to_ascii_lowercase();
        if seen.iter().any(|e| e == &low || e.contains(&low[..low.len().min(36)])) {
            return;
        }
        // Suppress stock method cards.
        if low.contains("list premises")
            || low.contains("compare on capability")
            || low.contains("fake certainty")
            || low.contains("objective, constraints")
        {
            return;
        }
        seen.push(low);
        out.push(t.to_owned());
    };

    let multi = looks_multi_domain_user(user);
    let user_tokens = content_tokens_bridge(user);

    let lead = matched.insight.as_ref().and_then(|i| {
        let t = i.trim();
        if t.chars().count() < 16 || t.chars().count() > 200 {
            return None;
        }
        // Reject identity/capability meta-insights on non-capability questions
        // (classic SoftCascade contamination: "strongest honest claim about Perci…").
        if insight_is_self_meta(t) && !looks_capability_user(user) {
            return None;
        }
        if !multi
            && !looks_capability_user(user)
            && !insight_touches_tokens(t, &user_tokens)
            && matched.label == "identity"
        {
            return None;
        }
        Some(t.to_owned())
    });
    if let Some(ref l) = lead {
        seen.push(l.to_ascii_lowercase());
    }

    // Attention-ordered non-residual mixture first — filter cross-domain contamination.
    let mut mix: Vec<_> = matched
        .mixture
        .iter()
        .filter(|m| !m.residual)
        .filter(|m| support_is_relevant(user, matched.label.as_str(), m, multi, &user_tokens))
        .collect();
    mix.sort_by_key(|m| std::cmp::Reverse(m.attention_pm));
    for m in mix {
        if let Some(ref i) = m.insight {
            push(&mut supports, &mut seen, i);
        }
        if supports.len() >= 3 {
            break;
        }
    }

    // Residual stream (hop order) — separate channel for hop-aware transitions.
    let mut residual_supports: Vec<String> = Vec::new();
    let mut res: Vec<_> = matched
        .mixture
        .iter()
        .filter(|m| m.residual)
        .filter(|m| support_is_relevant(user, matched.label.as_str(), m, multi, &user_tokens))
        .collect();
    res.sort_by_key(|m| (m.hop, std::cmp::Reverse(m.attention_pm)));
    for m in res {
        if let Some(ref i) = m.insight {
            push(&mut residual_supports, &mut seen, i);
        }
        if residual_supports.len() >= 2 {
            break;
        }
    }

    // Semantic frame lattice — only frames that touch user tokens (or multi-domain).
    let activated = deliberation::activate_semantic_frames(user, 3);
    let mut frames = Vec::new();
    let mut mechanisms = Vec::new();
    for f in activated {
        if !multi && !frame_touches_user(&f.clause, &user_tokens) && f.score < 40 {
            continue;
        }
        push(&mut frames, &mut seen, &f.clause);
        if f.mechanism.chars().count() >= 20 {
            let mlow = f.mechanism.to_ascii_lowercase();
            if !seen.iter().any(|e| e.contains(&mlow[..mlow.len().min(30)])) {
                mechanisms.push(f.mechanism);
            }
        }
    }

    let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
    let frame_n = frames.len();
    let composition = matched.composition.clone();
    let contested =
        matched.margin < 16 || residual_n > 0 || frame_n >= 2 || composition.len() >= 3;
    let rich = lead.is_some()
        || !supports.is_empty()
        || !residual_supports.is_empty()
        || frame_n >= 2
        || composition.len() >= 3
        || (matched.primary_attention_pm >= 400 && !matched.mixture.is_empty());

    BridgePacket {
        lead,
        supports,
        residual_supports,
        frames,
        mechanisms,
        composition,
        rich,
        contested,
        attention_primary_pm: matched.primary_attention_pm,
        mixture_n: matched.mixture.iter().filter(|m| !m.residual).count(),
        residual_n,
        frame_n,
    }
}

/// Compose a multi-hypothesis answer as **thoughtful free-form prose**.
///
/// Emergent arc (no section labels):
/// **thesis → warrant → boundary → check**, drawn from Bitwork α / residual / frames.
/// Session premise may soft-bind when the user is continuing a thread.
pub fn compose_soft_cascade(
    user: &str,
    matched: &CognitiveMatch,
    domain_body: &str,
    variant: usize,
) -> String {
    let packet = assemble(matched, user);
    let tokens = content_tokens_bridge(user);
    let topic = if tokens.is_empty() {
        matched.label.clone()
    } else {
        tokens.iter().take(4).cloned().collect::<Vec<_>>().join(" ")
    };
    let ask = ask_shape(user);

    if !packet.rich {
        let body = domain_body.to_owned();
        remember_premise(&body);
        return body;
    }

    // Geometry speaks back: analyze field (+ ledger lessons) → prefer mixture thesis when
    // primary is off-topic; chronic labels force multipartite arc + critique.
    let geo = crate::emergence::analyze(matched, user);
    crate::emergence::set_session_policy(geo.clone());
    let mix_thesis = if geo.prefer_mixture_thesis {
        crate::emergence::preferred_mixture_insight(matched, user)
    } else {
        None
    };
    let used_mix = mix_thesis.is_some();
    let mut arc = ThoughtArc::from_packet(
        &packet,
        domain_body,
        matched.margin,
        style_depth(),
        geo.force_multipartite_arc,
    );
    if let Some(ref thesis) = mix_thesis {
        arc.thesis = thesis.trim().trim_end_matches('.').trim().to_owned();
        arc.contested = true;
    }
    // Geometry blind: primary and mixture both miss user tokens — still force contested tone.
    if geo.geometry_blind {
        arc.contested = true;
    }
    let mut out = arc.speak(user, &topic, ask, variant, peek_premise().as_deref());

    // VSA soft binding — never on identity/capability (schema dump ruins natural tone).
    // Also skip when voice gate says the frame is noise (agent:capable, ask:what only).
    if packet.composition.len() >= 2
        && !looks_capability_user(user)
        && matched.label != "identity"
        && matched.label != "greeting"
        && crate::voice::should_voice_composition_public(user, &packet.composition)
        && (variant % 3 != 2 || packet.supports.is_empty())
    {
        out = crate::voice::weave_composition_frame(&out, &packet.composition, variant);
    }

    // Bind user topic if diluted (always when geometry_blind or mixture-corrected).
    let ol = out.to_ascii_lowercase();
    let hit = tokens.iter().filter(|t| ol.contains(t.as_str())).count();
    let need_bind = tokens.len() >= 2 && hit == 0;
    let force_bind = geo.geometry_blind || (used_mix && hit < tokens.len().min(2));
    if need_bind || force_bind {
        if hit == 0 && tokens.len() >= 2 {
            out.push(' ');
            out.push_str(&format!(
                "All of that still answers {}.",
                tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
            ));
        } else if force_bind && hit < 2 && !tokens.is_empty() {
            out.push(' ');
            out.push_str(&format!(
                "On {}: hold the claim against the live constraints.",
                tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
            ));
        }
    }

    while out.contains("  ") {
        out = out.replace("  ", " ");
    }

    // Self-critique residual loop (second pass on thin drafts).
    // Contested + residual / chronic / geometry_blind → geometry forces deeper pass.
    let (mut refined, mut critique) = self_critique_refine(user, &out, &packet, matched);
    if geo.lower_critique_threshold && !critique.expanded {
        let prev = style_depth();
        set_style_depth(2);
        let (r2, c2) = self_critique_refine(user, &out, &packet, matched);
        set_style_depth(prev);
        if c2.expanded {
            refined = r2;
            critique = c2;
        }
    }
    store_critique(&critique);
    store_tree(&render_prototype_tree(matched, &packet));

    let mut plan = LengthPlan::from_bitwork(user, matched, &packet, CognitionPath::Cascade);
    plan = plan.apply_style_depth(style_depth());
    // Contested / multipartite / blind geometry → slightly more room to think.
    if (packet.contested || geo.force_multipartite_arc || geo.geometry_blind)
        && style_depth() != 1
    {
        plan.words = (plan.words.saturating_add(24)).min(LengthPlan::L_MAX as usize);
    }
    let body = apply_word_budget(&refined, plan.words);
    remember_premise(&body);
    // Close the loop: speech quality feeds the ledger so future lessons can fire.
    crate::emergence::record_speech_outcome(user, &body, used_mix);
    plan.seal_backend(&body)
}

/// Load-bearing thought roles emergent from Bitwork mass (not free association).
#[derive(Clone, Debug, Default)]
struct ThoughtArc {
    thesis: String,
    warrant: Option<String>,
    boundary: Option<String>,
    check: Option<String>,
    contested: bool,
}

impl ThoughtArc {
    fn from_packet(
        packet: &BridgePacket,
        domain_body: &str,
        margin: i32,
        depth: u8,
        force_multipartite: bool,
    ) -> Self {
        let clean = |s: &str| -> String {
            s.trim().trim_end_matches('.').trim().to_owned()
        };
        let mut thesis = packet
            .lead
            .as_ref()
            .map(|s| clean(s))
            .filter(|s| s.chars().count() >= 16)
            .or_else(|| {
                packet
                    .supports
                    .first()
                    .map(|s| clean(s))
                    .filter(|s| s.chars().count() >= 16)
            })
            .unwrap_or_else(|| clean(domain_body));

        if thesis.chars().count() < 12 {
            thesis = clean(domain_body);
        }

        // Warrant = best same-geometry support distinct from thesis.
        let warrant = packet.supports.iter().find_map(|s| {
            let c = clean(s);
            if c.chars().count() >= 16 && !near_dup(&c, &thesis) {
                Some(c)
            } else {
                None
            }
        });

        // Boundary = residual second thought OR mechanism (when the claim could fail).
        let boundary = packet
            .residual_supports
            .first()
            .map(|s| clean(s))
            .filter(|s| s.chars().count() >= 16 && !near_dup(s, &thesis))
            .or_else(|| {
                packet
                    .mechanisms
                    .first()
                    .map(|s| clean(s))
                    .filter(|s| s.chars().count() >= 20 && !near_dup(s, &thesis))
            })
            .or_else(|| {
                packet.frames.first().map(|s| clean(s)).filter(|s| {
                    s.chars().count() >= 16 && !near_dup(s, &thesis)
                })
            });

        // Check = second residual / second support when contested, multipartite force, or deep.
        let need_check =
            packet.contested || margin < 14 || depth == 2 || force_multipartite;
        let check = if need_check {
            packet
                .residual_supports
                .get(1)
                .map(|s| clean(s))
                .filter(|s| s.chars().count() >= 16)
                .or_else(|| {
                    packet.supports.get(1).map(|s| clean(s)).filter(|s| {
                        s.chars().count() >= 16
                            && !near_dup(s, &thesis)
                            && warrant.as_ref().map(|w| !near_dup(s, w)).unwrap_or(true)
                    })
                })
        } else {
            None
        };

        // Concise style: keep thesis + one of warrant/boundary.
        let (warrant, boundary, check) = if depth == 1 {
            (warrant.or(boundary), None, None)
        } else {
            (warrant, boundary, check)
        };

        Self {
            thesis,
            warrant,
            boundary,
            check,
            contested: packet.contested || margin < 12,
        }
    }

    fn speak(
        &self,
        user: &str,
        topic: &str,
        ask: AskShape,
        variant: usize,
        prior_premise: Option<&str>,
    ) -> String {
        let mut out = String::new();
        let lower = user.to_ascii_lowercase();
        let follow =
            lower.contains("what about")
                || lower.contains("and ")
                || lower.starts_with("also")
                || lower.contains("still")
                || lower.contains("then ")
                || lower.contains("under partition")
                || lower.contains("more carefully");

        // Soft session continuity — only when continuing, not on cold opens.
        if follow {
            if let Some(p) = prior_premise {
                if p.chars().count() >= 24 && p.chars().count() <= 160 {
                    let p0 = decapitalize_if_mid(p.trim_end_matches('.'));
                    match variant % 3 {
                        0 => out.push_str(&format!("Building on that — {p0}. ")),
                        1 => out.push_str(&format!("Keeping the last thread in view: {p0}. ")),
                        _ => out.push_str(&format!("From where we left off: {p0}. ")),
                    }
                }
            }
        }

        let t = self.thesis.trim_end_matches('.').to_owned();
        let t0 = decapitalize_if_mid(&t);

        // Thesis — answer first, human natural.
        match (ask, variant % 4) {
            (AskShape::Why, 0) => out.push_str(&format!("{t}.")),
            (AskShape::Why, 1) => out.push_str(&format!("For {topic}, {t0}.")),
            (AskShape::Why, _) => out.push_str(&format!(
                "It comes down to this: {t0}."
            )),
            (AskShape::How, 0) => out.push_str(&format!("{t}.")),
            (AskShape::How, 1) => out.push_str(&format!("In practice, {t0}.")),
            (AskShape::How, _) => out.push_str(&format!("The workable path is when {t0}.")),
            (AskShape::What, 0) => out.push_str(&format!("{t}.")),
            (AskShape::What, _) => out.push_str(&format!(
                "The cleanest read of {topic}: {t0}."
            )),
            (AskShape::Connect, 0) => out.push_str(&format!("{t}.")),
            (AskShape::Connect, _) => out.push_str(&format!(
                "A bridge for {topic}: {t0}."
            )),
            (AskShape::Open, 0) => out.push_str(&format!("{t}.")),
            (AskShape::Open, _) => out.push_str(&format!("On {topic}: {t0}.")),
        }

        // Warrant — why the thesis holds (mechanism, not slogan).
        if let Some(ref w) = self.warrant {
            let w0 = decapitalize_if_mid(w.trim_end_matches('.'));
            let bridge = match variant % 5 {
                0 => " That holds because ",
                1 => " The reason it lands is that ",
                2 => " Underneath that, ",
                3 => " You can see it when ",
                _ => " What makes it real is that ",
            };
            out.push_str(bridge);
            out.push_str(&w0);
            if !out.ends_with('.') {
                out.push('.');
            }
        }

        // Boundary — when it fails / what it is not (honest thought).
        if let Some(ref b) = self.boundary {
            let b0 = decapitalize_if_mid(b.trim_end_matches('.'));
            let bridge = match variant % 4 {
                0 => " It frays when ",
                1 => " The edge case is that ",
                2 => " Still, that only stays true if ",
                _ => " Where this gets fragile: ",
            };
            out.push_str(bridge);
            out.push_str(&b0);
            if !out.ends_with('.') {
                out.push('.');
            }
        }

        // Check — discriminating test / next thought under contest.
        if let Some(ref c) = self.check {
            let c0 = decapitalize_if_mid(c.trim_end_matches('.'));
            let bridge = match variant % 3 {
                0 => " A useful check is whether ",
                1 => " You can pressure-test it by asking if ",
                _ => " If that shifted, you'd also see that ",
            };
            out.push_str(bridge);
            out.push_str(&c0);
            if !out.ends_with('.') {
                out.push('.');
            }
        } else if self.contested
            && style_depth() != 1
            && !looks_capability_user(user)
            && !topic_is_identity(topic)
        {
            // Contested multipartite honesty — not on self-description answers.
            out.push_str(
                " I'm holding more than one working frame; these are the pieces that still cohere.",
            );
        }

        out
    }
}

fn topic_is_identity(topic: &str) -> bool {
    let t = topic.to_ascii_lowercase();
    t.contains("identity")
        || t.contains("capable")
        || t.contains("capability")
        || t.contains("perci")
}

fn near_dup(a: &str, b: &str) -> bool {
    let al = a.to_ascii_lowercase();
    let bl = b.to_ascii_lowercase();
    if al == bl {
        return true;
    }
    let take = |s: &str| s.chars().take(36).collect::<String>();
    take(&al) == take(&bl) || al.contains(&take(&bl)) || bl.contains(&take(&al))
}

// ─── Response length budget + cognition trace ───────────────────────────────

/// Cognitive path class for base budget B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CognitionPath {
    ExactTool,
    Operator,
    Social,
    Cascade,
    Open,
}

/// Integer length plan (nuanced, still float-free):
/// \( L = \min(L_{\max}, \lceil B(1 + 0.6\alpha + 1.2 H_r + 0.4\log_2(1+C) + I_u)\rceil) \)
///
/// Floor: social ≥ 1 short sentence; cap L_max unless deep intent.
#[derive(Clone, Debug)]
pub struct LengthPlan {
    pub words: usize,
    pub base_b: u32,
    pub alpha_pm: u32,
    pub residual_hops: u32,
    pub complexity_pm: u32,
    pub intent_pm: u32,
    pub factor_pm: u32,
    pub path: CognitionPath,
    pub domains: Vec<String>,
    pub composition: Vec<String>,
    pub mixture_n: usize,
    pub frame_n: usize,
    pub lead_snippet: Option<String>,
    pub residual_n: usize,
    pub margin: i32,
    pub label: String,
}

impl LengthPlan {
    const L_MAX: u32 = 600;
    const L_MAX_DEFAULT: u32 = 420;

    pub fn from_bitwork(
        user: &str,
        matched: &CognitiveMatch,
        packet: &BridgePacket,
        path: CognitionPath,
    ) -> Self {
        let residual_hops = matched
            .mixture
            .iter()
            .filter(|m| m.residual)
            .map(|m| m.hop as u32)
            .max()
            .unwrap_or(0)
            .min(2);
        let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
        let mut domains = vec![matched.label.clone()];
        for m in &matched.mixture {
            if !domains.iter().any(|d| d == &m.label) {
                domains.push(m.label.clone());
            }
        }
        let experts = domains.len() as u32;
        let cd_units = experts
            + (packet.frame_n as u32).min(4)
            + (matched.composition.len() as u32).min(4);
        let alpha_pm = matched.primary_attention_pm as u32;
        let intent_pm = intent_multiplier_pm(user);
        let base_b = base_words(path, user);
        let (factor_pm, complexity_pm) =
            length_factor_pm(alpha_pm, residual_hops, cd_units, intent_pm);
        let words = finalize_words(base_b, factor_pm, path, intent_pm);
        Self {
            words,
            base_b,
            alpha_pm,
            residual_hops,
            complexity_pm,
            intent_pm,
            factor_pm,
            path,
            domains,
            composition: matched.composition.clone(),
            mixture_n: packet.mixture_n,
            frame_n: packet.frame_n,
            lead_snippet: packet.lead.clone().map(|s| truncate_chars(&s, 80)),
            residual_n,
            margin: matched.margin,
            label: matched.label.clone(),
        }
    }

    /// Operator / exact-tool path without a full Bitwork match.
    pub fn from_light(user: &str, path: CognitionPath, domains: &[&str], operator: &str) -> Self {
        let intent_pm = intent_multiplier_pm(user);
        let base_b = base_words(path, user);
        let cd_units = domains.len() as u32;
        let (factor_pm, complexity_pm) = length_factor_pm(0, 0, cd_units, intent_pm);
        let words = finalize_words(base_b, factor_pm, path, intent_pm);
        Self {
            words,
            base_b,
            alpha_pm: 0,
            residual_hops: 0,
            complexity_pm,
            intent_pm,
            factor_pm,
            path,
            domains: domains.iter().map(|s| (*s).to_owned()).collect(),
            composition: Vec::new(),
            mixture_n: 0,
            frame_n: 0,
            lead_snippet: None,
            residual_n: 0,
            margin: 0,
            label: operator.to_owned(),
        }
    }

    /// Operator/tool answer **with** Bitwork geometry probe (α, residual hops, mixture).
    /// Answer text still comes from the operator; the plan reports sparse field state.
    pub fn from_operator_with_bitwork(
        user: &str,
        path: CognitionPath,
        operator: &str,
        operator_domains: &[&str],
        matched: Option<&CognitiveMatch>,
    ) -> Self {
        let Some(matched) = matched else {
            return Self::from_light(user, path, operator_domains, operator);
        };
        let packet = assemble(matched, user);
        // Use open/cascade base when Bitwork is multipartite so L can grow with geometry.
        let geometry_path = if matched.mixture.len() >= 1 || packet.residual_n > 0 {
            CognitionPath::Cascade
        } else {
            path
        };
        let mut plan = Self::from_bitwork(user, matched, &packet, geometry_path);
        // Keep operator identity while retaining Bitwork α/hops/domains.
        plan.path = path;
        plan.label = format!("{}|{}", operator, matched.label);
        // Domains: operator tag + Bitwork experts (dedup).
        let mut domains: Vec<String> = Vec::new();
        for d in operator_domains {
            let s = (*d).to_owned();
            if !domains.iter().any(|x| x == &s) {
                domains.push(s);
            }
        }
        for d in plan.domains.drain(..) {
            if !domains.iter().any(|x| x == &d) {
                domains.push(d);
            }
        }
        plan.domains = domains;
        // Recompute L: operator base B with Bitwork α/hops so backend plan is honest.
        let base_b = base_words(path, user);
        let cd_units = plan.domains.len() as u32
            + (plan.frame_n as u32).min(4)
            + (plan.composition.len() as u32).min(4);
        let (factor_pm, complexity_pm) =
            length_factor_pm(plan.alpha_pm, plan.residual_hops, cd_units, plan.intent_pm);
        plan.base_b = base_b;
        plan.factor_pm = factor_pm;
        plan.complexity_pm = complexity_pm;
        plan.words = finalize_words(base_b, factor_pm, path, plan.intent_pm);
        plan
    }

    /// Adjust L from durable style memory (`/concise` · `/deep` · balanced).
    pub fn apply_style_depth(mut self, depth: u8) -> Self {
        match depth {
            1 => {
                // concise: shrink budget ~35%
                self.words = ((self.words as u32).saturating_mul(65) / 100).max(8) as usize;
                self.intent_pm = self.intent_pm.min(1000);
            }
            2 => {
                // deep: grow budget ~30%, allow deeper cap
                self.words = ((self.words as u32).saturating_mul(130) / 100)
                    .min(LengthPlan::L_MAX) as usize;
                self.intent_pm = self.intent_pm.max(1500);
            }
            _ => {}
        }
        self
    }

    /// One-line backend summary (never prefixed to chat).
    pub fn short_trace(&self) -> String {
        let alpha_pct = self.alpha_pm / 10; // 0–100
        let domains = if self.domains.is_empty() {
            self.label.clone()
        } else {
            self.domains
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("+")
        };
        let path = if self.label.contains('|') {
            format!("{}+bitwork", path_tag(self.path))
        } else {
            path_tag(self.path).to_owned()
        };
        format!(
            "α={alpha_pct}% · hops={} · domains={domains} · L={} ({path}·B={})",
            self.residual_hops, self.words, self.base_b
        )
    }

    /// Rich backend plan for `/think` only — human chat never sees this.
    pub fn verbose_trace(&self) -> String {
        let alpha_pct = self.alpha_pm / 10;
        let alpha_fixed = self.alpha_pm; // ‰
        let factor_x100 = self.factor_pm / 10;
        let length_band = if self.words <= 40 {
            "tight"
        } else if self.words <= 120 {
            "medium"
        } else if self.words <= 240 {
            "expanded"
        } else {
            "deep"
        };
        let thought_mode = if self.margin < 12 {
            "contested multipartite"
        } else if self.residual_hops > 0 {
            "thesis+residual"
        } else {
            "locked attractor"
        };
        let path_disp = if self.label.contains('|') {
            format!("{}+bitwork", path_tag(self.path))
        } else {
            path_tag(self.path).to_owned()
        };
        let domains = if self.domains.is_empty() {
            "—".to_owned()
        } else {
            self.domains.iter().take(5).cloned().collect::<Vec<_>>().join(" + ")
        };
        let mut out = String::new();
        out.push_str("[Cognition Trace · backend]\n");
        out.push_str(&format!("• Thought mode: {thought_mode}\n"));
        // Emergent field phase from margin (locked / contested).
        let phase = if self.margin < 4 {
            "contested (multipartite read)"
        } else if self.margin < 14 {
            "soft lock"
        } else {
            "locked attractor"
        };
        out.push_str(&format!(
            "• Field phase: {phase} · margin={} · mix={} · residual_n={}\n",
            self.margin, self.mixture_n, self.residual_n
        ));
        out.push_str(&format!(
            "• Lead: 0.{:02}α ({domains})\n",
            alpha_pct.min(99)
        ));
        if self.residual_hops > 0 {
            out.push_str(&format!(
                "• Residual hop {}: multipartite second thought (n={})\n",
                self.residual_hops, self.residual_n
            ));
        } else {
            out.push_str("• Residual hop: none (locked or operator-only)\n");
        }
        out.push_str(&format!(
            "• Concepts: mix={} · frames={} · margin={}\n",
            self.mixture_n, self.frame_n, self.margin
        ));
        if !self.composition.is_empty() {
            out.push_str(&format!(
                "• VSA bind: {}\n",
                self.composition.iter().take(4).cloned().collect::<Vec<_>>().join(" · ")
            ));
        }
        out.push_str(&format!(
            "• Length decision: {length_band} (×{}.{:02}) → L={} words (B={} · path={path_disp})\n",
            factor_x100 / 100,
            factor_x100 % 100,
            self.words,
            self.base_b
        ));
        out.push_str(&format!(
            "• Law: L=min({}, ceil(B·(1+0.6α+1.2H_r+0.4log2(1+C)+N_r+I_u))) · α={}‰ · H_r={} · C_d={}‰ · I_u={}‰\n",
            Self::L_MAX,
            alpha_fixed,
            self.residual_hops,
            self.complexity_pm,
            self.intent_pm
        ));
        if let Some(ref lead) = self.lead_snippet {
            out.push_str(&format!("• Lead snippet: {lead}\n"));
        }
        out.push_str(&format!("• Route label: {}\n", self.label));
        out.push_str("note: inspectable Bitwork geometry — not private chain-of-thought; never shown in chat.");
        out
    }

    /// Apply length budget, store plan for `/think`, return **human body only**.
    pub fn seal_backend(&self, body: &str) -> String {
        store_last_verbose(self);
        remember_premise(body);
        // Human-facing output never includes cognition prefixes.
        let _ = turn_verbose(); // consume flag without prepending
        body.to_owned()
    }

    /// Backward-compatible name: human body only; plan stays backend.
    pub fn envelope(&self, body: &str, _verbose: bool) -> String {
        self.seal_backend(body)
    }
}

/// Explicit opt-in to surface cognition in chat (default **off** — human-facing clean).
/// Set `PERCI_COGNITION_TRACE=chat` only for debug dumps; normal path is backend-only.
pub fn cognition_trace_in_chat() -> bool {
    match std::env::var("PERCI_COGNITION_TRACE") {
        Ok(v) => {
            let l = v.to_ascii_lowercase();
            matches!(l.as_str(), "1" | "on" | "true" | "chat" | "debug")
        }
        Err(_) => false,
    }
}

// ─── Self-critique residual loop ───────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CritiqueReport {
    pub score: u32,
    pub expanded: bool,
    pub reason: String,
}

impl CritiqueReport {
    pub fn format_backend(&self) -> String {
        format!(
            "[Self-critique]\n• score={}/100 · expanded={} · {}",
            self.score,
            if self.expanded { "yes" } else { "no" },
            self.reason
        )
    }
}

/// Fast metacognition: is the draft clear, governed, and sufficient?
/// If thin, weave one residual/mechanism facet in natural speech (<50ms target).
pub fn self_critique_refine(
    user: &str,
    draft: &str,
    packet: &BridgePacket,
    matched: &CognitiveMatch,
) -> (String, CritiqueReport) {
    let lower = user.to_ascii_lowercase();
    let words = draft.split_whitespace().count() as u32;
    let tokens = content_tokens_bridge(user);
    let draft_l = draft.to_ascii_lowercase();
    let token_hits = tokens
        .iter()
        .filter(|t| t.len() >= 4 && draft_l.contains(t.as_str()))
        .count() as u32;

    let mut score: u32 = 50;
    // Coverage of user content
    score = score.saturating_add(token_hits.saturating_mul(8).min(24));
    // Adequate length for ask shape
    let wants_depth = lower.contains("why")
        || lower.contains("how")
        || lower.contains("explain")
        || lower.contains("connect");
    if wants_depth && words >= 40 {
        score = score.saturating_add(15);
    } else if wants_depth && words < 22 {
        score = score.saturating_sub(20);
    }
    if words >= 12 && words <= 220 {
        score = score.saturating_add(10);
    }
    // Multipartite evidence used
    if !packet.supports.is_empty() {
        score = score.saturating_add(8);
    }
    if !packet.residual_supports.is_empty() {
        // residual available but unused in draft → pressure to expand
        let used_res = packet
            .residual_supports
            .iter()
            .any(|r| draft_l.contains(&r.to_ascii_lowercase()[..r.len().min(28)]));
        if used_res {
            score = score.saturating_add(10);
        } else {
            score = score.saturating_sub(12);
        }
    }
    // Contested geometry should not be a one-liner
    if packet.contested && words < 28 {
        score = score.saturating_sub(15);
    }
    // Stock / thin patterns
    if draft_l.contains("list premises") || draft_l.contains("fake certainty") {
        score = score.saturating_sub(25);
    }
    // Locked high-α short answers on greetings are fine
    if matched.primary_attention_pm >= 600 && words >= 8 && !wants_depth {
        score = score.saturating_add(12);
    }
    score = score.min(100);

    let threshold = if style_depth() == 1 { 40 } else { 55 };
    if score >= threshold {
        return (
            draft.to_owned(),
            CritiqueReport {
                score,
                expanded: false,
                reason: "draft clear and sufficient under style depth".into(),
            },
        );
    }

    // Expand once from residual or mechanism — natural human tone, no labels.
    let facet = packet
        .residual_supports
        .first()
        .cloned()
        .or_else(|| packet.mechanisms.first().cloned())
        .or_else(|| packet.supports.last().cloned());

    let Some(facet) = facet else {
        return (
            draft.to_owned(),
            CritiqueReport {
                score,
                expanded: false,
                reason: "thin draft but no residual/mechanism mass to expand".into(),
            },
        );
    };
    let fl = facet.to_ascii_lowercase();
    if draft_l.contains(&fl[..fl.len().min(32)]) {
        return (
            draft.to_owned(),
            CritiqueReport {
                score,
                expanded: false,
                reason: "facet already present; no expand".into(),
            },
        );
    }

    let body = decapitalize_if_mid(facet.trim().trim_end_matches('.'));
    let bridge = match words % 3 {
        0 => " One more angle worth holding: ",
        1 => " Holding that lightly, also: ",
        _ => " And if you push one step further: ",
    };
    let mut refined = draft.trim_end().to_owned();
    if !refined.ends_with('.') && !refined.ends_with('?') && !refined.ends_with('!') {
        refined.push('.');
    }
    refined.push_str(bridge);
    refined.push_str(&body);
    if !refined.ends_with('.') {
        refined.push('.');
    }

    (
        refined,
        CritiqueReport {
            score,
            expanded: true,
            reason: format!("score {score} < {threshold}; wove residual/mechanism second angle"),
        },
    )
}

/// ASCII prototype tree for `/think` — lead · mix · residual · VSA.
pub fn render_prototype_tree(matched: &CognitiveMatch, packet: &BridgePacket) -> String {
    let mut lines = Vec::new();
    lines.push("[Prototype tree]".to_owned());
    let alpha = matched.primary_attention_pm / 10;
    lines.push(format!(
        "  ◆ lead  {}  α={}%" ,
        matched.label, alpha
    ));
    let mut mix: Vec<_> = matched.mixture.iter().filter(|m| !m.residual).collect();
    mix.sort_by_key(|m| std::cmp::Reverse(m.attention_pm));
    let res: Vec<_> = matched.mixture.iter().filter(|m| m.residual).collect();
    let total = mix.len() + res.len() + if packet.composition.is_empty() { 0 } else { 1 };
    let mut i = 0usize;
    for m in mix.iter().take(3) {
        i += 1;
        let branch = if i == total { "└─" } else { "├─" };
        let a = m.attention_pm / 10;
        let insight = m
            .insight
            .as_ref()
            .map(|s| truncate_chars(s, 42))
            .unwrap_or_else(|| "—".into());
        lines.push(format!(
            "  {branch} mix  {}  α={}%  · {insight}",
            m.label, a
        ));
    }
    for m in res.iter().take(2) {
        i += 1;
        let branch = if i == total { "└─" } else { "├─" };
        let a = m.attention_pm / 10;
        lines.push(format!(
            "  {branch} residual hop{}  {}  α={}%",
            m.hop, m.label, a
        ));
    }
    if !packet.composition.is_empty() {
        let branch = "└─";
        let binds = packet
            .composition
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(format!("  {branch} vsa  {binds}"));
    }
    if total == 0 && packet.composition.is_empty() {
        lines.push("  └─ (single attractor — no multipartite mass)".into());
    }
    lines.join("\n")
}

/// \(1 + 0.6\alpha + 1.2 H_r + 0.4\log_2(1+C) + N_r + I_u\) in permille.
/// \(N_r\): novelty/residual bonus when multipartite mass is present.
fn length_factor_pm(alpha_pm: u32, residual_hops: u32, cd_units: u32, intent_pm: u32) -> (u32, u32) {
    // 0.6 * α  → alpha_pm * 600 / 1000
    let alpha_term = (alpha_pm.saturating_mul(600)) / 1000;
    // 1.2 * H_r → hops * 1200 (H_r is dimensionless count 0–2)
    let hop_term = residual_hops.min(2).saturating_mul(1200);
    // 0.4 * log2(1+C) → log2(1+C) * 400
    let log_c = integer_log2_floor(1u32.saturating_add(cd_units));
    let complexity_pm = log_c.saturating_mul(400).min(500);
    // N_r: residual novelty — reward second thoughts (0.15 per hop, capped 0.30)
    let novelty_pm = residual_hops.min(2).saturating_mul(150);
    let factor_pm = 1000u32
        .saturating_add(alpha_term)
        .saturating_add(hop_term)
        .saturating_add(complexity_pm)
        .saturating_add(novelty_pm)
        .saturating_add(intent_pm);
    (factor_pm, complexity_pm)
}

fn integer_log2_floor(n: u32) -> u32 {
    if n <= 1 {
        0
    } else {
        31 - n.leading_zeros()
    }
}

fn finalize_words(base_b: u32, factor_pm: u32, path: CognitionPath, intent_pm: u32) -> usize {
    let raw = ((base_b as u64).saturating_mul(factor_pm as u64).saturating_add(999)) / 1000;
    let l_max = if intent_pm >= 1800 {
        LengthPlan::L_MAX
    } else {
        LengthPlan::L_MAX_DEFAULT
    };
    let floor = match path {
        CognitionPath::Social => 6,
        CognitionPath::ExactTool => 4,
        _ => 12,
    };
    (raw as u32).min(l_max).max(floor) as usize
}

fn base_words(path: CognitionPath, user: &str) -> u32 {
    let lower = user.to_ascii_lowercase();
    match path {
        CognitionPath::ExactTool => 30,
        CognitionPath::Social => 28,
        CognitionPath::Operator => {
            if lower.contains("plan") || lower.contains("step") {
                110
            } else {
                90
            }
        }
        CognitionPath::Cascade | CognitionPath::Open => {
            if lower.contains("detailed") || lower.contains("thorough") {
                160
            } else {
                120
            }
        }
    }
}

/// I_u as permille (1000 = 1.0).
fn intent_multiplier_pm(user: &str) -> u32 {
    let lower = user.to_ascii_lowercase();
    if lower.contains("detailed")
        || lower.contains("thorough")
        || lower.contains("in depth")
        || lower.contains("think step")
        || lower.contains("step by step")
    {
        1800
    } else if lower.contains("explain")
        || lower.contains("why ")
        || lower.starts_with("why")
        || lower.contains("how does")
        || lower.contains("how should")
        || lower.contains("how can ")
        || lower.contains("reason about")
    {
        1500
    } else if lower.contains("brief")
        || lower.contains("short answer")
        || lower.contains("tl;dr")
        || lower.contains("tldr")
    {
        700
    } else {
        1000
    }
}

fn path_tag(path: CognitionPath) -> &'static str {
    match path {
        CognitionPath::ExactTool => "tool",
        CognitionPath::Operator => "operator",
        CognitionPath::Social => "social",
        CognitionPath::Cascade => "cascade",
        CognitionPath::Open => "open",
    }
}

/// Truncate body to at most `max_words` whitespace-separated tokens, clean end.
pub fn apply_word_budget(text: &str, max_words: usize) -> String {
    if max_words == 0 {
        return String::new();
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.trim().to_owned();
    }
    let mut cut = words[..max_words].join(" ");
    // Prefer ending on sentence boundary if near the end of the kept window.
    if let Some(pos) = cut.rfind(['.', '!', '?']) {
        if pos + 1 >= cut.len() / 2 {
            cut.truncate(pos + 1);
            return cut;
        }
    }
    if !cut.ends_with('.') {
        cut.push('…');
    }
    cut
}

fn truncate_chars(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_owned()
    } else {
        t.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Envelope for operator / tool answers without SoftCascade.
pub fn envelope_light(
    user: &str,
    path: CognitionPath,
    domains: &[&str],
    operator: &str,
    body: &str,
    verbose: bool,
) -> String {
    envelope_with_bitwork(user, path, domains, operator, body, verbose, None)
}

/// Operator/tool envelope optionally fused with a Bitwork classify probe.
pub fn envelope_with_bitwork(
    user: &str,
    path: CognitionPath,
    domains: &[&str],
    operator: &str,
    body: &str,
    verbose: bool,
    matched: Option<&CognitiveMatch>,
) -> String {
    let mut body = body.to_owned();
    if let Some(m) = matched {
        let packet = assemble(m, user);
        store_tree(&render_prototype_tree(m, &packet));
        // Light critique on operator drafts too (reuse residual mass when thin).
        let (refined, critique) = self_critique_refine(user, &body, &packet, m);
        store_critique(&critique);
        body = refined;
    } else {
        store_tree("[Prototype tree]\n  └─ (operator path · no Bitwork probe)");
        store_critique(&CritiqueReport {
            score: 70,
            expanded: false,
            reason: "operator path without Bitwork probe".into(),
        });
    }
    let mut plan = LengthPlan::from_operator_with_bitwork(user, path, operator, domains, matched);
    plan = plan.apply_style_depth(style_depth());
    let trimmed = apply_word_budget(&body, plan.words);
    plan.envelope(&trimmed, verbose)
}

/// Strip optional cognition flags from user input. Returns (verbose_once, clean).
pub fn strip_cognition_flags(input: &str) -> (bool, String) {
    let mut verbose = false;
    let mut s = input.trim().to_owned();
    loop {
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("--verbose-cognition") {
            verbose = true;
            s = s["--verbose-cognition".len()..].trim_start().to_owned();
            continue;
        }
        // "think: <question>" as a one-shot verbose ask (not the /think command).
        if lower.starts_with("think:") {
            verbose = true;
            s = s["think:".len()..].trim_start().to_owned();
            continue;
        }
        break;
    }
    (verbose, s)
}

#[derive(Clone, Copy)]
enum AskShape {
    Why,
    How,
    What,
    Connect,
    Open,
}

fn ask_shape(user: &str) -> AskShape {
    let l = user.to_ascii_lowercase();
    if l.contains("why ") || l.starts_with("why") || l.contains("reason for") {
        AskShape::Why
    } else if l.contains("how ") || l.starts_with("how") || l.contains("in what way") {
        AskShape::How
    } else if l.contains("connect ") || l.contains("relate ") || l.contains("relationship") {
        AskShape::Connect
    } else if l.contains("what is") || l.contains("what are") || l.contains("explain ") {
        AskShape::What
    } else {
        AskShape::Open
    }
}

fn decapitalize_if_mid(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) if first.is_uppercase() => {
            // Keep acronyms / short all-caps.
            if s.chars().take(3).all(|ch| ch.is_uppercase() || !ch.is_alphabetic()) {
                s.to_owned()
            } else {
                first.to_lowercase().collect::<String>() + c.as_str()
            }
        }
        _ => s.to_owned(),
    }
}

/// Prefer SoftCascade when Bitwork evidence can support free-form multi-facet speech.
pub fn should_use_cascade(matched: &CognitiveMatch, user: &str) -> bool {
    let words = user.split_whitespace().count();
    if words < 3 {
        return false;
    }
    let social = user.to_ascii_lowercase();
    if matches!(
        social.trim(),
        "hi" | "hello" | "hey" | "thanks" | "thank you" | "bye" | "goodbye"
    ) {
        return false;
    }
    // Exact numeric asks stay on tool/natural_exact paths — cascade is for prose.
    if social.chars().filter(|c| c.is_ascii_digit()).count() >= 2
        && (social.contains("calculate")
            || social.contains("divided")
            || social.contains('%')
            || social.contains("plus")
            || social.contains("minus"))
    {
        return false;
    }
    let packet = assemble(matched, user);
    let open = looks_open_fluency_user(&social);
    packet.rich
        || matched.margin < 24
        || matched.mixture.len() >= 1
        || matched.composition.len() >= 2
        || (open && matched.insight.is_some())
        || (open && words >= 6 && matched.overlap >= 40)
}

fn looks_open_fluency_user(lower: &str) -> bool {
    lower.starts_with("why ")
        || lower.starts_with("how ")
        || lower.starts_with("what ")
        || lower.starts_with("explain ")
        || lower.contains("why does")
        || lower.contains("how should")
        || lower.contains("how does")
        || lower.contains("what about")
        || lower.contains("connect ")
        || lower.contains("tell me about")
}

fn content_tokens_bridge(user: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "if", "then", "than", "that", "this", "what",
        "when", "where", "which", "who", "why", "how", "can", "could", "would", "should",
        "will", "just", "really", "very", "your", "you", "me", "my", "our", "we", "i", "is",
        "are", "was", "were", "be", "been", "do", "does", "did", "to", "of", "in", "on", "for",
        "it", "its", "as", "at", "by", "not", "no", "please", "tell", "about", "with", "from",
    ];
    user.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .take(8)
        .collect()
}

fn looks_multi_domain_user(user: &str) -> bool {
    let l = user.to_ascii_lowercase();
    l.contains("connect ")
        || l.contains(" vs ")
        || l.contains(" versus ")
        || l.contains("difference between")
        || l.contains("relationship between")
        || l.contains("relate ")
        || l.contains("across ")
        || (l.contains(" and ") && l.split_whitespace().count() >= 6)
}

fn looks_capability_user(user: &str) -> bool {
    let l = user.to_ascii_lowercase();
    l.contains("capable")
        || l.contains("what can you")
        || l.contains("what do you do")
        || l.contains("abilities")
        || l.contains("capabilities")
        || (l.contains("what are you") && l.contains("do"))
}

/// Drop mixture supports that would contaminate a low-margin primary domain.
fn support_is_relevant(
    user: &str,
    primary: &str,
    m: &crate::cognitive::MixtureSupport,
    multi: bool,
    user_tokens: &[String],
) -> bool {
    if multi || m.label == primary {
        return true;
    }
    // Residual stream already survived ANDNOT + novelty scoring in classify.
    // Allow it through unless it is biological noise on exact/math prompts.
    if m.residual && m.insight.is_some() {
        let l = user.to_ascii_lowercase();
        let mathish = l.chars().any(|c| c.is_ascii_digit())
            || l.contains("equal")
            || l.contains("plus")
            || l.contains("calculate");
        if mathish {
            if let Some(ref i) = m.insight {
                let il = i.to_ascii_lowercase();
                if il.contains("death") || il.contains("life ") || il.contains("organism") {
                    return false;
                }
            }
        }
        return true;
    }
    // Capability asks: identity may legitimately support general.
    if looks_capability_user(user)
        && ((primary == "general" && m.label == "identity")
            || (primary == "identity" && m.label == "general"))
    {
        return true;
    }
    // Share a content token between user and insight.
    if let Some(ref insight) = m.insight {
        let il = insight.to_ascii_lowercase();
        if user_tokens.iter().any(|t| t.len() >= 4 && il.contains(t.as_str())) {
            return true;
        }
    }
    // High attention different-domain support needs stronger evidence.
    if m.attention_pm >= 250 && m.insight.is_some() {
        // Still reject biological/life-death noise on math-y prompts.
        let l = user.to_ascii_lowercase();
        let mathish = l.chars().any(|c| c.is_ascii_digit())
            || l.contains("equal")
            || l.contains("plus")
            || l.contains("calculate");
        if mathish {
            if let Some(ref i) = m.insight {
                let il = i.to_ascii_lowercase();
                if il.contains("death") || il.contains("organism") || il.contains("membrane") {
                    return false;
                }
            }
        }
        return m.score > 0 && m.overlap >= 6;
    }
    false
}

fn frame_touches_user(clause: &str, user_tokens: &[String]) -> bool {
    let cl = clause.to_ascii_lowercase();
    user_tokens
        .iter()
        .any(|t| t.len() >= 4 && cl.contains(t.as_str()))
}

fn insight_touches_tokens(insight: &str, user_tokens: &[String]) -> bool {
    let il = insight.to_ascii_lowercase();
    user_tokens
        .iter()
        .any(|t| t.len() >= 4 && il.contains(t.as_str()))
}

fn insight_is_self_meta(insight: &str) -> bool {
    let l = insight.to_ascii_lowercase();
    (l.contains("perci") || l.contains("strongest honest claim") || l.contains("i am a local"))
        && (l.contains("operational")
            || l.contains("routing")
            || l.contains("weights")
            || l.contains("governed")
            || l.contains("not a cloud")
            || l.contains("not conscious"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::{CognitiveMatch, MixtureSupport};

    fn sample_match() -> CognitiveMatch {
        CognitiveMatch {
            label: "systems".into(),
            variant: 1,
            concept_id: 1,
            insight: Some(
                "trust needs clear interfaces so authority and proof stay checkable".into(),
            ),
            score: 200,
            overlap: 40,
            runner_up_score: 180,
            margin: 8,
            query_popcount: 100,
            prototype_popcount: 120,
            positive_overlap: 30,
            negative_overlap: 5,
            hamming: 140,
            jaccard: 0.2,
            overlap_z: 2.0,
            mixture: vec![
                MixtureSupport {
                    label: "governance".into(),
                    score: 160,
                    overlap: 28,
                    concept_id: 2,
                    insight: Some(
                        "permission and proof are different gates under partial observability"
                            .into(),
                    ),
                    residual: false,
                    hop: 0,
                    attention_pm: 280,
                },
                MixtureSupport {
                    label: "memory".into(),
                    score: 90,
                    overlap: 12,
                    concept_id: 3,
                    insight: Some(
                        "memory reconstructs past state from stored traces under partial cues"
                            .into(),
                    ),
                    residual: true,
                    hop: 1,
                    attention_pm: 120,
                },
            ],
            composition: vec![
                "ask:why".into(),
                "agent:trust".into(),
                "domain:distributed".into(),
            ],
            primary_attention_pm: 400,
        }
    }

    #[test]
    fn assemble_marks_rich_when_insight_and_mixture() {
        let m = sample_match();
        let p = assemble(&m, "why does trust fail in distributed systems?");
        assert!(p.rich);
        assert!(p.lead.is_some());
        assert!(!p.supports.is_empty());
    }

    #[test]
    fn soft_cascade_binds_topic_and_avoids_empty() {
        let m = sample_match();
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "Give each piece one job.",
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(low.contains("trust") || low.contains("interface") || low.contains("permission"));
        assert!(out.split_whitespace().count() >= 12);
        assert!(!low.contains("list premises"));
        // No preset section labels.
        assert!(!low.contains("lattice:"));
        assert!(!low.contains("mixture read"));
        assert!(!low.contains("bound as"));
        assert!(!low.contains("multi-hypothesis readout"));
    }

    #[test]
    fn soft_cascade_why_opens_with_reason() {
        let m = sample_match();
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "placeholder body",
            0,
        );
        let low = out.to_ascii_lowercase();
        // Human chat must stay clean — no cognition dump.
        assert!(
            !low.contains("[cognition"),
            "cognition leaked into human output: {out}"
        );
        assert!(
            low.contains("because")
                || low.contains("comes down")
                || low.contains("structural")
                || low.contains("trust")
                || low.contains("interface")
                || low.contains("permission")
                || low.contains("holds because")
                || low.contains("frays"),
            "got: {out}"
        );
        // Backend plan still available.
        let plan = peek_last_verbose_trace().expect("plan sealed");
        assert!(plan.contains("Lead:") || plan.contains("α") || plan.contains("Length") || plan.contains("Thought mode"));
    }

    #[test]
    fn thought_arc_has_thesis_and_warrant() {
        let m = sample_match();
        let packet = assemble(&m, "why does trust fail in distributed systems?");
        let arc = ThoughtArc::from_packet(&packet, "fallback body long enough", 8, 0, false);
        assert!(arc.thesis.chars().count() >= 16);
        // sample has mixture support → warrant likely
        assert!(arc.warrant.is_some() || arc.boundary.is_some());
        let spoken = arc.speak(
            "why does trust fail in distributed systems?",
            "trust distributed",
            AskShape::Why,
            0,
            None,
        );
        assert!(spoken.split_whitespace().count() >= 10);
        assert!(!spoken.contains("Lattice:"));
    }

    #[test]
    fn session_premise_remembered() {
        remember_premise(
            "Trust fails when authority and evidence drift out of sync across nodes. Next sentence.",
        );
        let p = peek_premise().expect("premise");
        assert!(p.to_ascii_lowercase().contains("trust") || p.contains("authority"));
    }

    #[test]
    fn length_plan_integer_law_scales_with_intent() {
        let m = sample_match();
        let packet = assemble(&m, "why does trust fail in distributed systems?");
        let open = LengthPlan::from_bitwork(
            "why does trust fail in distributed systems?",
            &m,
            &packet,
            CognitionPath::Cascade,
        );
        let brief = LengthPlan::from_bitwork(
            "brief: trust fail",
            &m,
            &packet,
            CognitionPath::Cascade,
        );
        assert!(open.words >= 80, "open L={}", open.words);
        assert!(open.intent_pm >= 1500);
        assert!(brief.intent_pm <= 1000 || brief.words <= open.words);
        assert!(open.short_trace().contains("α="));
        let v = open.verbose_trace();
        assert!(
            v.contains("Lead:") || v.contains("0.") || v.contains("α"),
            "got: {v}"
        );
    }

    #[test]
    fn apply_word_budget_cuts_cleanly() {
        let long = "one two three four five six seven eight nine ten";
        let cut = apply_word_budget(long, 4);
        assert_eq!(cut.split_whitespace().count(), 4);
    }

    #[test]
    fn strip_verbose_cognition_flag() {
        let (v, clean) = strip_cognition_flags("--verbose-cognition why trust fails");
        assert!(v);
        assert_eq!(clean, "why trust fails");
        let (v2, c2) = strip_cognition_flags("think: explain residual hops");
        assert!(v2);
        assert!(c2.contains("explain"));
    }

    #[test]
    fn self_critique_expands_thin_draft_with_residual() {
        let m = sample_match();
        let packet = assemble(&m, "why does trust fail in distributed systems?");
        let thin = "Trust needs contracts.";
        let (out, report) = self_critique_refine(
            "why does trust fail in distributed systems?",
            thin,
            &packet,
            &m,
        );
        assert!(report.score < 80);
        // Either expanded or explained why not.
        if report.expanded {
            assert!(out.len() > thin.len());
            assert!(!out.to_ascii_lowercase().contains("second thought:"));
        }
    }

    #[test]
    fn prototype_tree_shows_lead_and_branches() {
        let m = sample_match();
        let packet = assemble(&m, "why does trust fail in distributed systems?");
        let tree = render_prototype_tree(&m, &packet);
        assert!(tree.contains("◆ lead"));
        assert!(tree.contains("mix") || tree.contains("residual") || tree.contains("vsa"));
    }

    #[test]
    fn style_depth_shrinks_concise_budget() {
        let m = sample_match();
        let packet = assemble(&m, "why does trust fail?");
        let base = LengthPlan::from_bitwork(
            "why does trust fail?",
            &m,
            &packet,
            CognitionPath::Cascade,
        );
        let concise = base.clone().apply_style_depth(1);
        let deep = base.clone().apply_style_depth(2);
        assert!(concise.words <= base.words);
        assert!(deep.words >= base.words);
    }

    #[test]
    fn operator_with_bitwork_carries_alpha_and_hops() {
        let m = sample_match();
        let plan = LengthPlan::from_operator_with_bitwork(
            "why does trust fail in distributed systems?",
            CognitionPath::Operator,
            "trust-systems",
            &["trust-systems"],
            Some(&m),
        );
        assert!(plan.alpha_pm > 0, "α should come from Bitwork primary");
        assert_eq!(plan.residual_hops, 1);
        assert!(plan.mixture_n >= 1 || plan.residual_n >= 1);
        assert!(plan.label.contains("trust-systems"));
        assert!(plan.label.contains('|'));
        let short = plan.short_trace();
        assert!(short.contains("bitwork") || short.contains("α="));
        assert!(plan.alpha_pm == 400);
        let sealed = plan.seal_backend("Human facing only.");
        assert_eq!(sealed, "Human facing only.");
        assert!(!sealed.contains("[Cognition"));
    }

    #[test]
    fn soft_cascade_weaves_residual_and_vsa_composition() {
        let m = sample_match();
        let p = assemble(&m, "why does trust fail in distributed systems?");
        assert!(
            !p.residual_supports.is_empty(),
            "residual channel should carry hop-1 insight"
        );
        assert!(p.composition.iter().any(|c| c.starts_with("ask:")));

        // Free-form residual second-thought language (no jargon "residual stream").
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "placeholder body",
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(
            !low.contains("residual stream") && !low.contains("further residual:"),
            "jargon residual labels leaked: {out}"
        );
        assert!(
            low.contains("another layer")
                || low.contains("second angle")
                || low.contains("also in play")
                || low.contains("one more thread")
                || low.contains("memory reconstructs"),
            "expected free-form residual weave, got: {out}"
        );
        // VSA soft-binding should surface or topic remains.
        let out1 = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "placeholder body",
            1,
        );
        let low1 = out1.to_ascii_lowercase();
        assert!(
            low1.contains("treating that as")
                || low1.contains("shaped as")
                || low1.contains("ask")
                || low1.contains("trust"),
            "expected VSA composition cue or topic, got: {out1}"
        );
    }

    #[test]
    fn soft_cascade_freeform_prefers_content_lead() {
        let m = sample_match();
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "placeholder body",
            0,
        );
        let low = out.to_ascii_lowercase();
        // Lead with substance; avoid stock method cards.
        assert!(low.contains("trust") || low.contains("interface") || low.contains("permission"));
        assert!(!low.contains("list premises"));
        assert!(!low.contains("lattice:"));
        // Free-form paragraphs should be multi-sentence when multi-facet.
        assert!(
            out.matches('.').count() >= 2,
            "expected multi-sentence free-form, got: {out}"
        );
    }

    #[test]
    fn soft_cascade_geometry_prefers_mixture_when_primary_off_topic() {
        // Primary insight is phenomenology fluff; mixture hits the user question.
        let mut m = sample_match();
        m.margin = 2;
        m.insight = Some(
            "Behavioral complexity is observable; subjective experience is inferred.".into(),
        );
        m.mixture[0].insight = Some(
            "Interfaces earn trust when timeouts and retries stay explicit under lag.".into(),
        );
        m.mixture[0].attention_pm = 350;
        let user = "how should interfaces earn trust under lag and retry?";
        let geo = crate::emergence::analyze(&m, user);
        assert!(
            geo.prefer_mixture_thesis,
            "geometry should prefer mixture: tags={:?}",
            geo.tags
        );
        let out = compose_soft_cascade(user, &m, "placeholder body that is long enough", 0);
        let low = out.to_ascii_lowercase();
        assert!(
            low.contains("trust")
                || low.contains("timeout")
                || low.contains("interface")
                || low.contains("lag")
                || low.contains("retry"),
            "mixture thesis or topic bind should surface user domain, got: {out}"
        );
        assert!(!low.contains("[cognition"), "cognition leak: {out}");
    }

    #[test]
    fn should_use_cascade_on_conceptual_multi_facet() {
        let m = sample_match();
        assert!(should_use_cascade(
            &m,
            "why does trust fail in distributed systems?"
        ));
        assert!(!should_use_cascade(&m, "hi"));
    }
}
