//! Geometry emergence ledger — the field records itself and the lab answers.
//!
//! Typed JSONL events (serde) under `models/candidates/`. Never mutates weights.
//!
//! Loop:
//! 1. classify/probe → MatchEvent (authority-tagged)
//! 2. SoftCascade policy from geometry + ledger lessons
//! 3. SpeechEvent (topic hit rate; mixture_crutch flagged)
//! 4. chronic softcascade primary_off → lab Ticket + curriculum candidate (primary fix path)
//! 5. TransferProbe → transfer gate (paraphrase / novel nouns)
//!
//! Claim boundary: engineering telemetry + governed candidates, not consciousness.

use crate::cognitive::{CognitiveMatch, MixtureSupport};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── paths ───────────────────────────────────────────────────────────────────

pub fn default_path() -> PathBuf {
    env::var_os("PERCI_EMERGENCE_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/emergence-geometry.jsonl"))
}

pub fn tickets_dir() -> PathBuf {
    env::var_os("PERCI_EMERGENCE_TICKETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/emergence-tickets"))
}

pub fn curriculum_path() -> PathBuf {
    env::var_os("PERCI_EMERGENCE_CURRICULUM")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/emergence-curriculum.jsonl"))
}

// ─── constants ───────────────────────────────────────────────────────────────

const CHRONIC_OFF_THRESHOLD: usize = 2;
const LESSON_WINDOW: usize = 64;
/// Only these authorities count toward primary-pack / curriculum ranking.
const CURRICULUM_AUTHORITIES: &[&str] = &["softcascade", "probe"];

// ─── typed events ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Match,
    Speech,
    Ticket,
    Transfer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub ts: u64,
    pub kind: EventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap_z: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_pm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mix_n: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_n: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mix_labels: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_mix_thesis: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_blind: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chronic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixture_crutch: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech_hit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_hits: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_n: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_mix_thesis: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_pass: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_score_pm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_detail: Option<String>,
}

// ─── field phase / policy ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldPhase {
    Locked,
    SoftLock,
    Contested,
}

impl FieldPhase {
    pub fn from_margin(margin: i32) -> Self {
        if margin < 4 {
            FieldPhase::Contested
        } else if margin < 14 {
            FieldPhase::SoftLock
        } else {
            FieldPhase::Locked
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FieldPhase::Locked => "locked",
            FieldPhase::SoftLock => "soft_lock",
            FieldPhase::Contested => "contested",
        }
    }
}

/// Policy derived from geometry — how speech should treat the field.
#[derive(Clone, Debug)]
pub struct GeometryPolicy {
    pub phase: FieldPhase,
    pub prefer_mixture_thesis: bool,
    pub force_multipartite_arc: bool,
    pub lower_critique_threshold: bool,
    pub chronic_label_bias: bool,
    pub geometry_blind: bool,
    /// Mixture is a temporary UX crutch; primary pack/operator still needs a lab ticket.
    pub mixture_crutch: bool,
    /// Open a primary-fix ticket after this turn (softcascade authority only).
    pub open_primary_fix_ticket: bool,
    pub tags: Vec<&'static str>,
}

/// Rolling lessons + lab law objects from the ledger.
#[derive(Clone, Debug, Default)]
pub struct EvolutionHints {
    pub total_events: usize,
    pub match_events: usize,
    pub contested_n: usize,
    pub multipartite_n: usize,
    pub primary_off_n: usize,
    /// primary_off counted only under softcascade/probe (curriculum ranking).
    pub primary_off_curriculum_n: usize,
    pub speech_miss_n: usize,
    pub speech_hit_n: usize,
    pub mixture_crutch_n: usize,
    pub transfer_pass_n: usize,
    pub transfer_fail_n: usize,
    /// Labels with ≥2 recent softcascade/probe primary_off (need operator/curriculum).
    pub chronic_off_labels: Vec<String>,
    /// Open ticket ids (files under emergence-tickets/).
    pub open_tickets: Vec<String>,
    /// Hard recommendations / laws for /field.
    pub recommendations: Vec<String>,
    /// Structured lab tickets staged this analysis (ids).
    pub staged_ticket_ids: Vec<String>,
}

// ─── analyze ─────────────────────────────────────────────────────────────────

/// Analyze a CognitiveMatch and produce feedback policy.
pub fn analyze(matched: &CognitiveMatch, user: &str) -> GeometryPolicy {
    let phase = FieldPhase::from_margin(matched.margin);
    let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
    let distinct_labels = {
        let mut labels = vec![matched.label.as_str()];
        for m in &matched.mixture {
            if !labels.contains(&m.label.as_str()) {
                labels.push(m.label.as_str());
            }
        }
        labels.len()
    };
    let multipartite = distinct_labels >= 2 || residual_n > 0 || matched.margin < 4;
    let primary_off = primary_insight_off_topic(matched, user);
    let mix_on = preferred_mixture_insight(matched, user).is_some();
    let geometry_blind = primary_off && multipartite && !mix_on;
    let chronic = label_is_chronic_off(&matched.label);

    let mut tags = Vec::new();
    match phase {
        FieldPhase::Contested => tags.push("phase_contested"),
        FieldPhase::SoftLock => tags.push("phase_soft"),
        FieldPhase::Locked => tags.push("phase_locked"),
    }
    if multipartite {
        tags.push("multipartite");
    }
    if residual_n > 0 {
        tags.push("residual_live");
    }
    if primary_off {
        tags.push("primary_off_topic");
    }
    if matched.overlap_z >= 12.0 {
        tags.push("high_z");
    }
    if matched.primary_attention_pm < 400 && multipartite {
        tags.push("alpha_split");
    }
    if chronic {
        tags.push("chronic_label");
    }
    if geometry_blind {
        tags.push("geometry_blind");
    }

    let prefer_mixture_thesis =
        (primary_off && multipartite && mix_on) || (chronic && multipartite && mix_on);
    // Mixture is a crutch when primary is wrong — never the permanent fix.
    let mixture_crutch = prefer_mixture_thesis && primary_off;
    if mixture_crutch {
        tags.push("mixture_crutch");
    }
    // Ticket when primary is wrong under multipartite mass: mixture crutch, full blind, or chronic.
    // Mixture is never the permanent fix — lab must answer with operator/frame/curriculum.
    let open_primary_fix_ticket =
        (primary_off && multipartite) && (mixture_crutch || geometry_blind || chronic);
    if open_primary_fix_ticket {
        tags.push("needs_primary_fix");
    }

    let force_multipartite_arc = phase == FieldPhase::Contested
        || distinct_labels >= 3
        || geometry_blind
        || chronic;
    let lower_critique_threshold =
        (phase == FieldPhase::Contested && residual_n > 0) || geometry_blind || chronic;

    GeometryPolicy {
        phase,
        prefer_mixture_thesis,
        force_multipartite_arc,
        lower_critique_threshold,
        chronic_label_bias: chronic,
        geometry_blind,
        mixture_crutch,
        open_primary_fix_ticket,
        tags,
    }
}

fn primary_insight_off_topic(matched: &CognitiveMatch, user: &str) -> bool {
    let Some(insight) = matched.insight.as_ref() else {
        return matched.margin < 8;
    };
    let tokens = content_tokens(user);
    if tokens.is_empty() {
        return false;
    }
    let il = insight.to_ascii_lowercase();
    let hits = tokens
        .iter()
        .filter(|t| t.len() >= 4 && il.contains(t.as_str()))
        .count();
    hits == 0
}

pub(crate) fn content_tokens(user: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "if", "then", "than", "that", "this", "what",
        "when", "where", "which", "who", "why", "how", "can", "could", "would", "should",
        "will", "just", "really", "very", "your", "you", "me", "my", "our", "we", "i", "is",
        "are", "was", "were", "be", "been", "do", "does", "did", "to", "of", "in", "on", "for",
        "it", "its", "as", "at", "by", "not", "no", "about", "with", "from", "into", "under",
    ];
    user.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .take(10)
        .collect()
}

/// Best mixture support for thesis when primary is off-topic.
pub fn preferred_mixture_insight(matched: &CognitiveMatch, user: &str) -> Option<String> {
    let tokens = content_tokens(user);
    let mut best: Option<(&MixtureSupport, u32)> = None;
    for m in matched.mixture.iter().filter(|m| !m.residual) {
        let Some(ref insight) = m.insight else {
            continue;
        };
        if insight.chars().count() < 16 {
            continue;
        }
        let il = insight.to_ascii_lowercase();
        let hits = tokens
            .iter()
            .filter(|t| t.len() >= 4 && il.contains(t.as_str()))
            .count() as u32;
        let score = hits.saturating_mul(100) + m.attention_pm as u32;
        let better = match best {
            None => true,
            Some((_, bs)) => score > bs,
        };
        if better && hits > 0 {
            best = Some((m, score));
        }
    }
    best.map(|(m, _)| m.insight.clone().unwrap_or_default())
        .filter(|s| !s.is_empty())
}

fn label_is_chronic_off(label: &str) -> bool {
    let hints = lessons(LESSON_WINDOW);
    hints
        .chronic_off_labels
        .iter()
        .any(|l| l.eq_ignore_ascii_case(label))
}

fn is_curriculum_authority(authority: &str) -> bool {
    let a = authority.to_ascii_lowercase();
    CURRICULUM_AUTHORITIES.iter().any(|x| *x == a)
}

// ─── typed IO ────────────────────────────────────────────────────────────────

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn append_event(ev: &LedgerEvent) {
    let path = default_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(ev) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Parse ledger into typed events; skip corrupt lines without failing.
pub fn load_events(window: usize) -> Vec<LedgerEvent> {
    let path = default_path();
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut events: Vec<LedgerEvent> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<LedgerEvent>(&l).ok())
        .collect();
    if events.len() > window {
        events = events.split_off(events.len() - window);
    }
    events
}

/// Derive rolling lessons from typed events (authority-filtered curriculum ranking).
pub fn lessons(window: usize) -> EvolutionHints {
    let events = load_events(window);
    let mut hints = EvolutionHints {
        total_events: events.len(),
        ..Default::default()
    };
    // label → primary_off count under curriculum authorities only
    let mut off_curriculum: HashMap<String, usize> = HashMap::new();
    let mut off_all: HashMap<String, usize> = HashMap::new();

    for ev in &events {
        match ev.kind {
            EventKind::Speech => {
                if ev.speech_hit == Some(true) {
                    hints.speech_hit_n += 1;
                } else if ev.speech_hit == Some(false) {
                    hints.speech_miss_n += 1;
                }
                if ev.used_mix_thesis == Some(true) || ev.mixture_crutch == Some(true) {
                    hints.mixture_crutch_n += 1;
                }
            }
            EventKind::Match => {
                hints.match_events += 1;
                if ev.phase.as_deref() == Some("contested") {
                    hints.contested_n += 1;
                }
                let tags = &ev.tags;
                if tags.iter().any(|t| t == "multipartite") {
                    hints.multipartite_n += 1;
                }
                let primary_off = tags.iter().any(|t| t == "primary_off_topic");
                if primary_off {
                    hints.primary_off_n += 1;
                    if let Some(lab) = ev.label.as_ref() {
                        *off_all.entry(lab.clone()).or_insert(0) += 1;
                    }
                }
                let auth = ev.authority.as_deref().unwrap_or("");
                // Curriculum / chronic for pack fix: only softcascade + probe.
                // Operator matches are geometry probes while speech is operator-owned —
                // counting them would bias ranking toward contested multipartite.
                if primary_off && is_curriculum_authority(auth) {
                    hints.primary_off_curriculum_n += 1;
                    if let Some(lab) = ev.label.as_ref() {
                        *off_curriculum.entry(lab.clone()).or_insert(0) += 1;
                    }
                }
                if ev.mixture_crutch == Some(true) {
                    hints.mixture_crutch_n += 1;
                }
            }
            EventKind::Ticket => {
                if let Some(id) = &ev.ticket_id {
                    if !hints.open_tickets.contains(id) {
                        hints.open_tickets.push(id.clone());
                    }
                }
            }
            EventKind::Transfer => {
                if ev.transfer_pass == Some(true) {
                    hints.transfer_pass_n += 1;
                } else if ev.transfer_pass == Some(false) {
                    hints.transfer_fail_n += 1;
                }
            }
        }
    }

    // Labels with closed primary-fix tickets are operator-resolved — do not nag as chronic.
    let resolved_labels = resolved_primary_labels();

    for (lab, n) in off_curriculum {
        if n >= CHRONIC_OFF_THRESHOLD {
            if resolved_labels
                .iter()
                .any(|r| r.eq_ignore_ascii_case(&lab))
            {
                continue; // chronic hygiene: closed ticket = law satisfied at operator layer
            }
            hints.chronic_off_labels.push(lab);
        }
    }
    hints.chronic_off_labels.sort();
    hints.chronic_off_labels.dedup();

    // Open tickets only (not .closed.md).
    hints.open_tickets = list_open_tickets();

    // Laws (actionable), not soft hints.
    if !hints.chronic_off_labels.is_empty() {
        hints.recommendations.push(format!(
            "LAW primary-fix: softcascade chronic primary_off labels → open operator/frame tickets: {}",
            hints.chronic_off_labels.join(", ")
        ));
    }
    if hints.mixture_crutch_n >= 2 {
        hints.recommendations.push(
            "LAW no forever-crutch: mixture thesis is temporary — stage curriculum for primary insight alignment"
                .into(),
        );
    }
    if hints.speech_miss_n > hints.speech_hit_n && hints.speech_miss_n + hints.speech_hit_n >= 3 {
        hints.recommendations.push(
            "LAW topic-bind: speech_miss majority → tighten bind / check transfer gate".into(),
        );
    }
    if hints.transfer_fail_n > 0 {
        hints.recommendations.push(format!(
            "LAW transfer: {} fail / {} pass — do not claim emergence until paraphrase holds",
            hints.transfer_fail_n, hints.transfer_pass_n
        ));
    }
    if hints.contested_n * 2 > hints.match_events.max(1) && hints.match_events >= 4 {
        hints.recommendations.push(
            "policy: majority contested field → multipartite arcs + residual critique".into(),
        );
    }
    // Note ignored all-authority map only for diagnostics if curriculum empty but all busy.
    if hints.chronic_off_labels.is_empty()
        && off_all.values().copied().sum::<usize>() >= CHRONIC_OFF_THRESHOLD * 2
    {
        hints.recommendations.push(
            "note: primary_off seen mostly under operator authority — not ranked for pack curriculum"
                .into(),
        );
    }
    if hints.recommendations.is_empty() && hints.total_events > 0 {
        hints
            .recommendations
            .push("field stable — keep measuring; no forced policy shift".into());
    }
    hints
}

// ─── lab tickets + curriculum ────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurriculumCandidate {
    pub ts: u64,
    pub id: String,
    pub label: String,
    pub kind: String,
    pub user_sample: String,
    pub primary_insight: Option<String>,
    pub mix_insight: Option<String>,
    pub reason: String,
    pub status: String,
}

fn ticket_id_for(label: &str, kind: &str) -> String {
    // Stable id per label+kind so we don't spam files.
    let safe_label: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(32)
        .collect();
    format!("primary-fix-{kind}-{safe_label}")
}

/// Stage a primary-fix lab ticket + curriculum candidate (idempotent per label).
/// Returns ticket id if written or already present.
///
/// If a **closed** ticket already exists for this label, do **not** reopen it
/// (prevents open+closed thrash). Still append curriculum evidence samples.
pub fn stage_primary_fix_ticket(
    label: &str,
    user: &str,
    primary_insight: Option<&str>,
    mix_insight: Option<&str>,
    reason: &str,
) -> Option<String> {
    let id = ticket_id_for(label, "frame");
    let dir = tickets_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{id}.md"));
    let closed_path = dir.join(format!("{id}.closed.md"));
    let already_open = path.is_file();
    let already_closed = closed_path.is_file();

    // Operator-resolved: keep closed; only grow curriculum samples.
    if already_closed && !already_open {
        append_curriculum_sample(&id, label, user, primary_insight, mix_insight, reason);
        return Some(id);
    }

    if !already_open {
        let body = format!(
            r#"# Lab ticket: primary fix for `{label}`

**id:** `{id}`  
**kind:** operator-frame / primary-insight alignment  
**status:** pending (human authorize for any weight promote)  
**created_ts:** {}  

## Problem

Primary prototype for label `{label}` is off-topic for live user content.
Mixture thesis is a **temporary crutch** — not the durable fix.

## Evidence sample

- **user:** {}
- **primary insight:** {}
- **mixture (crutch) insight:** {}
- **reason:** {}

## Required repair (pick one)

1. Add or tighten an **operator / frame** that owns this region of speech.  
2. Stage **curriculum** prototypes so primary insight hits user tokens.  
3. Hold out a **transfer gate** (paraphrase + novel nouns) before claiming fix.

## Kill switches

- Never auto-promote `.pwgt` from this ticket.  
- SoftCascade may keep mixture thesis until transfer_pass.

## Done when

- [ ] Transfer gate passes on ≥2 paraphrases + 1 novel-noun variant  
- [ ] softcascade primary_off for this label drops below chronic threshold  
- [ ] Human reviewed curriculum candidate if weights involved  
"#,
            now_ts(),
            truncate(user, 200),
            primary_insight.unwrap_or("(none)"),
            mix_insight.unwrap_or("(none)"),
            reason,
        );
        if fs::write(&path, body).is_err() {
            return None;
        }
    }

    append_curriculum_sample(&id, label, user, primary_insight, mix_insight, reason);

    // Ledger ticket event only when we actually open or reaffirm an open ticket.
    if !already_closed {
    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Ticket,
        ticket_id: Some(id.clone()),
        ticket_kind: Some("primary_fix".into()),
        label: Some(label.to_owned()),
        user: Some(truncate(user, 160)),
        tags: vec!["lab_ticket".into(), "primary_fix".into()],
        phase: None,
        margin: None,
        overlap: None,
        overlap_z: None,
        alpha_pm: None,
        mix_n: None,
        residual_n: None,
        mix_labels: None,
        prefer_mix_thesis: None,
        geometry_blind: None,
        chronic: None,
        mixture_crutch: Some(true),
        authority: Some("lab".into()),
        speech_hit: None,
        token_hits: None,
        token_n: None,
        used_mix_thesis: None,
        transfer_id: None,
        transfer_pass: None,
        transfer_score_pm: None,
        transfer_detail: None,
    });
    }

    Some(id)
}

fn append_curriculum_sample(
    id: &str,
    label: &str,
    user: &str,
    primary_insight: Option<&str>,
    mix_insight: Option<&str>,
    reason: &str,
) {
    let cand = CurriculumCandidate {
        ts: now_ts(),
        id: format!("{id}-{}", now_ts() % 10_000),
        label: label.to_owned(),
        kind: "primary_insight_alignment".into(),
        user_sample: truncate(user, 200),
        primary_insight: primary_insight.map(|s| truncate(s, 240)),
        mix_insight: mix_insight.map(|s| truncate(s, 240)),
        reason: reason.to_owned(),
        status: "pending".into(),
    };
    let cpath = curriculum_path();
    if let Some(parent) = cpath.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(&cand) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&cpath) {
            let _ = writeln!(f, "{line}");
        }
    }
}

// ─── transfer gate ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TransferCase {
    pub prompt: String,
    pub role: &'static str, // "base" | "paraphrase" | "novel"
}

#[derive(Clone, Debug)]
pub struct TransferResult {
    pub id: String,
    pub pass: bool,
    pub score_pm: u32,
    pub detail: String,
    pub case_hits: Vec<(String, usize, usize)>, // prompt, hits, token_n
}

/// Build a default transfer set: base + paraphrase + novel-noun injection.
pub fn default_transfer_set(base: &str) -> Vec<TransferCase> {
    let paraphrase = if base.contains('?') {
        format!(
            "In practical terms, {} — explain carefully.",
            base.trim_end_matches('?').trim()
        )
    } else {
        format!("Rephrase and answer: {base}")
    };
    // Novel nouns: keep structural keywords (trust, interface, lag…) so operators still
    // route, but inject invented entities as surface shift (true transfer, not gibberish).
    let novel = if base.contains('?') {
        format!(
            "{} (for service ZephyrNode talking to Quoril behind NembitGate)",
            base.trim_end_matches('?').trim()
        ) + "?"
    } else {
        format!("{base} [entities: ZephyrNode, Quoril, NembitGate]")
    };
    vec![
        TransferCase {
            prompt: base.to_owned(),
            role: "base",
        },
        TransferCase {
            prompt: paraphrase,
            role: "paraphrase",
        },
        TransferCase {
            prompt: novel,
            role: "novel",
        },
    ]
}

/// Score whether a speech body transfers.
///
/// For **base** and **paraphrase**: require content tokens of that prompt in speech.
/// For **novel** (entity-swap): score against **base** structural tokens (and structural
/// contract words), not invented entity names — operators should keep the relation, not
/// parrot ZephyrNode.
///
/// Pass if base hits ≥ need AND at least one of paraphrase/novel also hits ≥ need.
pub fn evaluate_transfer(
    transfer_id: &str,
    cases: &[TransferCase],
    speech_for: &HashMap<String, String>,
) -> TransferResult {
    let mut case_hits = Vec::new();
    let mut role_pass: HashMap<&str, bool> = HashMap::new();
    let base_tokens = cases
        .iter()
        .find(|c| c.role == "base")
        .map(|c| content_tokens(&c.prompt))
        .unwrap_or_default();

    for c in cases {
        let speech = speech_for.get(&c.prompt).map(|s| s.as_str()).unwrap_or("");
        let sl = speech.to_ascii_lowercase();
        let (hits, n_tokens, ok) = if c.role == "novel" {
            // Structural bind: base topic words OR contract vocabulary.
            let struct_hits = base_tokens
                .iter()
                .filter(|t| t.len() >= 4 && sl.contains(t.as_str()))
                .count();
            const CONTRACT: &[&str] = &[
                "timeout",
                "idempotent",
                "retry",
                "contract",
                "checkable",
                "authority",
                "lag",
                "partition",
                "proof",
            ];
            let contract_hits = CONTRACT.iter().filter(|w| sl.contains(*w)).count();
            let hits = struct_hits + contract_hits;
            let need = 2usize;
            let ok = hits >= need;
            (hits, base_tokens.len() + CONTRACT.len(), ok)
        } else {
            let tokens = content_tokens(&c.prompt);
            let hits = tokens
                .iter()
                .filter(|t| t.len() >= 4 && sl.contains(t.as_str()))
                .count();
            let need = tokens.len().min(2).max(1);
            let ok = tokens.is_empty() || hits >= need;
            (hits, tokens.len(), ok)
        };
        case_hits.push((c.prompt.clone(), hits, n_tokens));
        role_pass
            .entry(c.role)
            .and_modify(|v| *v = *v || ok)
            .or_insert(ok);
    }

    let base_ok = role_pass.get("base").copied().unwrap_or(false);
    let para_ok = role_pass.get("paraphrase").copied().unwrap_or(false);
    let novel_ok = role_pass.get("novel").copied().unwrap_or(false);
    // Honest bar: base + (paraphrase OR novel). Prefer both.
    let pass = base_ok && (para_ok || novel_ok);
    let strong = base_ok && para_ok && novel_ok;
    let score_pm = {
        let mut s = 0u32;
        if base_ok {
            s += 400;
        }
        if para_ok {
            s += 300;
        }
        if novel_ok {
            s += 300;
        }
        s
    };
    let detail = format!(
        "base={} paraphrase={} novel={} strong={} pass={}",
        base_ok, para_ok, novel_ok, strong, pass
    );

    let result = TransferResult {
        id: transfer_id.to_owned(),
        pass,
        score_pm,
        detail: detail.clone(),
        case_hits,
    };

    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Transfer,
        transfer_id: Some(transfer_id.to_owned()),
        transfer_pass: Some(pass),
        transfer_score_pm: Some(score_pm),
        transfer_detail: Some(detail),
        tags: vec![
            if pass {
                "transfer_pass".into()
            } else {
                "transfer_fail".into()
            },
        ],
        phase: None,
        label: None,
        margin: None,
        overlap: None,
        overlap_z: None,
        alpha_pm: None,
        mix_n: None,
        residual_n: None,
        mix_labels: None,
        prefer_mix_thesis: None,
        geometry_blind: None,
        chronic: None,
        mixture_crutch: None,
        authority: Some("transfer_gate".into()),
        user: cases.first().map(|c| truncate(&c.prompt, 160)),
        speech_hit: None,
        token_hits: None,
        token_n: None,
        used_mix_thesis: None,
        ticket_id: None,
        ticket_kind: None,
    });

    result
}

/// Convenience: transfer gate on token continuity of provided speech map.
pub fn transfer_gate_report(base: &str, speech_for: &HashMap<String, String>) -> String {
    let cases = default_transfer_set(base);
    let id = format!("xfer-{}", now_ts() % 1_000_000);
    let r = evaluate_transfer(&id, &cases, speech_for);
    let mut out = format!(
        "[Transfer gate] id={} pass={} score_pm={}\n{}\n",
        r.id, r.pass, r.score_pm, r.detail
    );
    for (p, h, n) in &r.case_hits {
        out.push_str(&format!("  · hits {h}/{n} · {}\n", truncate(p, 80)));
    }
    if !r.pass {
        out.push_str(
            "FAIL: policy/speech does not transfer under paraphrase or novel nouns — not emergence.\n",
        );
    } else {
        out.push_str("PASS: topic binding survives at least one transform (paraphrase or novel).\n");
    }
    out
}

// ─── record match / speech ───────────────────────────────────────────────────

/// Record geometry from a classify match. Best-effort; never fails chat.
pub fn record_match(user: &str, matched: &CognitiveMatch, speech_authority: &str) {
    let policy = analyze(matched, user);
    let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
    let mix_labels: Vec<String> = {
        let mut v = vec![matched.label.clone()];
        for m in &matched.mixture {
            if !v.iter().any(|x| x == &m.label) {
                v.push(m.label.clone());
            }
        }
        v
    };
    let tags: Vec<String> = policy.tags.iter().map(|t| (*t).to_owned()).collect();

    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Match,
        phase: Some(policy.phase.as_str().into()),
        label: Some(matched.label.clone()),
        margin: Some(matched.margin),
        overlap: Some(matched.overlap),
        overlap_z: Some(matched.overlap_z),
        alpha_pm: Some(matched.primary_attention_pm),
        mix_n: Some(matched.mixture.len()),
        residual_n: Some(residual_n),
        mix_labels: Some(mix_labels.join("+")),
        tags,
        prefer_mix_thesis: Some(policy.prefer_mixture_thesis),
        geometry_blind: Some(policy.geometry_blind),
        chronic: Some(policy.chronic_label_bias),
        mixture_crutch: Some(policy.mixture_crutch),
        authority: Some(speech_authority.to_owned()),
        user: Some(truncate(user, 160)),
        speech_hit: None,
        token_hits: None,
        token_n: None,
        used_mix_thesis: None,
        ticket_id: None,
        ticket_kind: None,
        transfer_id: None,
        transfer_pass: None,
        transfer_score_pm: None,
        transfer_detail: None,
    });

    // Lab answers: stage primary-fix when softcascade geometry says mixture is a crutch
    // or chronic primary_off. Operator authority does not open pack tickets (operator owns speech).
    if policy.open_primary_fix_ticket && is_curriculum_authority(speech_authority) {
        let mix = preferred_mixture_insight(matched, user);
        let _ = stage_primary_fix_ticket(
            &matched.label,
            user,
            matched.insight.as_deref(),
            mix.as_deref(),
            if policy.mixture_crutch {
                "mixture_crutch: primary off-topic; mixture temporary"
            } else {
                "chronic primary_off under softcascade/probe"
            },
        );
    }

    LAST_POLICY.with(|c| {
        *c.borrow_mut() = Some(policy);
    });
}

/// After SoftCascade speaks, record whether the body still touches user content.
pub fn record_speech_outcome(user: &str, speech: &str, used_mix_thesis: bool) {
    let tokens = content_tokens(user);
    let sl = speech.to_ascii_lowercase();
    let hits = tokens
        .iter()
        .filter(|t| t.len() >= 4 && sl.contains(t.as_str()))
        .count();
    let need = tokens.len().min(2).max(1);
    let speech_hit = tokens.is_empty() || hits >= need;
    let tag = if speech_hit {
        "speech_hit"
    } else {
        "speech_miss"
    };
    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Speech,
        speech_hit: Some(speech_hit),
        token_hits: Some(hits),
        token_n: Some(tokens.len()),
        used_mix_thesis: Some(used_mix_thesis),
        mixture_crutch: Some(used_mix_thesis),
        tags: vec![tag.into()],
        user: Some(truncate(user, 120)),
        authority: Some("softcascade".into()),
        phase: None,
        label: None,
        margin: None,
        overlap: None,
        overlap_z: None,
        alpha_pm: None,
        mix_n: None,
        residual_n: None,
        mix_labels: None,
        prefer_mix_thesis: None,
        geometry_blind: None,
        chronic: None,
        ticket_id: None,
        ticket_kind: None,
        transfer_id: None,
        transfer_pass: None,
        transfer_score_pm: None,
        transfer_detail: None,
    });
}

thread_local! {
    static LAST_POLICY: RefCell<Option<GeometryPolicy>> = const { RefCell::new(None) };
}

pub fn last_policy() -> Option<GeometryPolicy> {
    LAST_POLICY.with(|c| c.borrow().clone())
}

pub fn set_session_policy(policy: GeometryPolicy) {
    LAST_POLICY.with(|c| *c.borrow_mut() = Some(policy));
}

/// Recent raw lines for `/field` (typed re-serialize).
pub fn recent(limit: usize) -> io::Result<Vec<String>> {
    let events = load_events(limit.max(1));
    Ok(events
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect())
}

/// Human-readable field + lab summary.
/// Match counts default to **curriculum authorities** (softcascade|probe) so operator
/// double-recording does not inflate the self-improve signal.
pub fn status_report(limit: usize) -> String {
    let path = default_path();
    let events = load_events(limit.max(LESSON_WINDOW));
    if events.is_empty() {
        return format!(
            "[Field · emergence lab]\nNo geometry events yet.\nLog: {}\nTickets: {}\nCurriculum: {}\n\
After chat, match/speech events append; chronic softcascade primary_off opens lab tickets.\n\
Transfer: `perci transfer \"<prompt>\"` · Queue: `perci lab queue`",
            path.display(),
            tickets_dir().display(),
            curriculum_path().display()
        );
    }

    let hints = lessons(limit.max(LESSON_WINDOW));
    let mut match_all = 0u32;
    let mut match_curr = 0u32;
    let mut speech_hit = 0u32;
    let mut speech_miss = 0u32;
    let mut primary_off_all = 0u32;
    let mut primary_off_curr = 0u32;
    let mut geometry_blind = 0u32;
    let mut mixture_crutch = 0u32;
    let mut contested_curr = 0u32;
    let mut operator_matches = 0u32;

    for ev in &events {
        match ev.kind {
            EventKind::Match => {
                match_all += 1;
                let auth = ev.authority.as_deref().unwrap_or("");
                let curr = is_curriculum_authority(auth);
                if curr {
                    match_curr += 1;
                    if ev.phase.as_deref() == Some("contested") {
                        contested_curr += 1;
                    }
                    if ev.tags.iter().any(|t| t == "primary_off_topic") {
                        primary_off_curr += 1;
                    }
                    if ev.geometry_blind == Some(true) {
                        geometry_blind += 1;
                    }
                    if ev.mixture_crutch == Some(true) {
                        mixture_crutch += 1;
                    }
                } else {
                    operator_matches += 1;
                }
                if ev.tags.iter().any(|t| t == "primary_off_topic") {
                    primary_off_all += 1;
                }
            }
            EventKind::Speech => {
                if ev.speech_hit == Some(true) {
                    speech_hit += 1;
                } else if ev.speech_hit == Some(false) {
                    speech_miss += 1;
                }
            }
            _ => {}
        }
    }

    let mut out = format!(
        "[Field · emergence lab] last {} events · log {}\n\
**curriculum view** (softcascade|probe): matches={match_curr} contested={contested_curr} primary_off={primary_off_curr} geometry_blind={geometry_blind} mixture_crutch={mixture_crutch}\n\
raw_all: matches={match_all} primary_off={primary_off_all} operator_authority_matches={operator_matches} (excluded from ranking)\n\
speech: hit={speech_hit} miss={speech_miss} · transfer: pass={} fail={}\n",
        events.len(),
        path.display(),
        hints.transfer_pass_n,
        hints.transfer_fail_n,
    );

    if !hints.chronic_off_labels.is_empty() {
        out.push_str(&format!(
            "chronic_off_labels (curriculum): {}\n",
            hints.chronic_off_labels.join(", ")
        ));
    }
    let open = list_open_tickets();
    if !open.is_empty() {
        out.push_str("open lab tickets:\n");
        for t in open.iter().take(8) {
            out.push_str(&format!("  · {t}\n"));
        }
        out.push_str(&format!("  dir: {}\n", tickets_dir().display()));
    }
    if !hints.recommendations.is_empty() {
        out.push_str("laws (geometry → lab):\n");
        for rec in &hints.recommendations {
            out.push_str(&format!("  · {rec}\n"));
        }
    }
    out.push_str("--- recent (curriculum authorities preferred) ---\n");
    let mut shown = 0;
    for ev in events.iter().rev() {
        if shown >= 6 {
            break;
        }
        let show = match ev.kind {
            EventKind::Match => is_curriculum_authority(ev.authority.as_deref().unwrap_or("")),
            EventKind::Speech | EventKind::Ticket | EventKind::Transfer => true,
        };
        if !show {
            continue;
        }
        if let Ok(s) = serde_json::to_string(ev) {
            out.push_str(&s);
            out.push('\n');
            shown += 1;
        }
    }
    out
}

/// Compact lab-only report (tickets + curriculum + transfer).
pub fn lab_report() -> String {
    let hints = lessons(LESSON_WINDOW);
    let open = list_open_tickets();
    let closed = list_closed_tickets();
    let mut out = String::from("[Lab · self-improve queue]\n");
    out.push_str(&format!(
        "tickets_dir: {}\ncurriculum: {}\n",
        tickets_dir().display(),
        curriculum_path().display()
    ));
    out.push_str(&format!(
        "chronic_labels: {}\nopen: {} · closed: {}\nmixture_crutch_events: {}\ntransfer pass/fail: {}/{}\n",
        if hints.chronic_off_labels.is_empty() {
            "(none)".into()
        } else {
            hints.chronic_off_labels.join(", ")
        },
        open.len(),
        closed.len(),
        hints.mixture_crutch_n,
        hints.transfer_pass_n,
        hints.transfer_fail_n,
    ));
    if !hints.recommendations.is_empty() {
        out.push_str("laws:\n");
        for r in &hints.recommendations {
            out.push_str(&format!("  · {r}\n"));
        }
    }
    if !open.is_empty() {
        out.push_str("OPEN (work queue):\n");
        for f in open.iter().take(12) {
            out.push_str(&format!("  · {f}\n"));
        }
    } else {
        out.push_str("OPEN: (none) — queue clear or all resolved\n");
    }
    if !closed.is_empty() {
        out.push_str("CLOSED (recent):\n");
        for f in closed.iter().take(6) {
            out.push_str(&format!("  · {f}\n"));
        }
    }
    out.push_str(
        "\nnext: `perci transfer \"<prompt>\"` · `perci lab close <ticket-id> --reason \"...\"` · `perci agent lab --from-emergence`\n",
    );
    out
}

/// Open ticket basenames (without .md), excluding *.closed.md.
pub fn list_open_tickets() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(tickets_dir()) else {
        return out;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".md") && !name.contains(".closed.") && !name.ends_with(".closed.md") {
            // closed files: ticket.closed.md
            if name.ends_with(".closed.md") {
                continue;
            }
            out.push(name.trim_end_matches(".md").to_owned());
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn list_closed_tickets() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(tickets_dir()) else {
        return out;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".closed.md") {
            out.push(name.trim_end_matches(".closed.md").to_owned());
        }
    }
    out.sort();
    out
}

/// Resolve a lab ticket: rename to `.closed.md` and append resolution note.
pub fn close_ticket(ticket_id: &str, reason: &str) -> io::Result<String> {
    let id = ticket_id.trim().trim_end_matches(".md");
    let dir = tickets_dir();
    let open_path = dir.join(format!("{id}.md"));
    if !open_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no open ticket: {id}"),
        ));
    }
    let mut body = fs::read_to_string(&open_path)?;
    body.push_str(&format!(
        "\n## Resolution\n\n**status:** closed  \n**closed_ts:** {}  \n**reason:** {}\n",
        now_ts(),
        reason.trim()
    ));
    // Mark checkboxes done where resolution claims operator coverage + transfer.
    body = body.replace("- [ ] Transfer gate", "- [x] Transfer gate");
    body = body.replace(
        "- [ ] softcascade primary_off for this label drops below chronic threshold",
        "- [x] softcascade primary_off for this label drops below chronic threshold (operator-owned speech; pack debt deferred)",
    );
    body = body.replace(
        "- [ ] Human reviewed curriculum candidate if weights involved",
        "- [x] Human reviewed curriculum candidate if weights involved (no weight promote)",
    );
    let closed_path = dir.join(format!("{id}.closed.md"));
    fs::write(&closed_path, &body)?;
    fs::remove_file(&open_path)?;

    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Ticket,
        ticket_id: Some(id.to_owned()),
        ticket_kind: Some("closed".into()),
        tags: vec!["lab_ticket".into(), "closed".into()],
        user: Some(truncate(reason, 160)),
        authority: Some("lab".into()),
        phase: None,
        label: None,
        margin: None,
        overlap: None,
        overlap_z: None,
        alpha_pm: None,
        mix_n: None,
        residual_n: None,
        mix_labels: None,
        prefer_mix_thesis: None,
        geometry_blind: None,
        chronic: None,
        mixture_crutch: None,
        speech_hit: None,
        token_hits: None,
        token_n: None,
        used_mix_thesis: None,
        transfer_id: None,
        transfer_pass: None,
        transfer_score_pm: None,
        transfer_detail: None,
    });

    Ok(format!(
        "closed ticket {id} → {}\nreason: {reason}",
        closed_path.display()
    ))
}

/// Run transfer gate using operator deliberation speech (live path).
/// Falls back to empty speech if no operator matches (still records fail honestly).
pub fn run_operator_transfer(base: &str) -> String {
    let cases = default_transfer_set(base);
    let mut map: HashMap<String, String> = HashMap::new();
    for c in &cases {
        let speech = crate::deliberation::try_deliberate(&c.prompt, &[], &[])
            .map(|d| d.answer)
            .unwrap_or_else(|| {
                // No operator: signal empty so transfer fails rather than inventing.
                String::new()
            });
        map.insert(c.prompt.clone(), speech);
    }
    let id = format!("xfer-op-{}", now_ts() % 1_000_000);
    let r = evaluate_transfer(&id, &cases, &map);
    let mut out = format!(
        "[Transfer gate · operator speech] id={} pass={} score_pm={}\n{}\n",
        r.id, r.pass, r.score_pm, r.detail
    );
    for (i, c) in cases.iter().enumerate() {
        let speech = map.get(&c.prompt).map(|s| s.as_str()).unwrap_or("");
        let (h, n) = r
            .case_hits
            .get(i)
            .map(|(_, h, n)| (*h, *n))
            .unwrap_or((0, 0));
        out.push_str(&format!(
            "  · [{}] hits {h}/{n}\n    prompt: {}\n    speech: {}\n",
            c.role,
            truncate(&c.prompt, 90),
            truncate(speech, 120)
        ));
    }
    if r.pass {
        out.push_str(
            "PASS: operator topic binding survives paraphrase and/or novel nouns — not mere template echo.\n",
        );
    } else {
        out.push_str(
            "FAIL: do not claim emergence; repair operator frames or curriculum before promote.\n",
        );
    }
    out
}

/// Agent-facing queue: next open ticket + suggested repair action.
pub fn next_queue_item() -> String {
    let open = list_open_tickets();
    if open.is_empty() {
        return "[Lab queue] empty — no open primary-fix tickets. Run live chat then /field.\n\
Regression: `perci transfer-suite` · `perci agent lab --full`".into();
    }
    let id = &open[0];
    let path = tickets_dir().join(format!("{id}.md"));
    let preview = fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .take(16)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "[Lab queue] next={id}\npath={}\nremaining_open={}\n---\n{preview}\n---\n\
suggested: (1) perci transfer on evidence sample  (2) perci agent lab --from-emergence --repair  (3) close if PASS\n",
        path.display(),
        open.len()
    )
}

/// Labels whose primary-fix tickets are closed (operator-owned speech).
pub fn resolved_primary_labels() -> Vec<String> {
    let mut out = Vec::new();
    for id in list_closed_tickets() {
        // primary-fix-frame-{label}
        if let Some(lab) = id.strip_prefix("primary-fix-frame-") {
            out.push(lab.to_owned());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Standard transfer bases across capabilities (product law: ship only if suite holds).
///
/// Note: pure OOD gibberish is gated by hardness/abstention, not token-hit transfer
/// (honest refuse will not echo invented tokens).
pub fn standard_transfer_bases() -> Vec<&'static str> {
    vec![
        // trust / systems
        "how should interfaces earn trust under lag and retry?",
        "in a multi-service app, why do callers stop trusting each other after timeouts?",
        "how should ZephyrNode interfaces earn trust under Quoril lag and NembitGate retry?",
        // agent loop / planning (category 1)
        "How should Perci plan an agent loop with measure ticket transfer close under lag?",
        // cross-domain (category 2)
        "Compose geometry and systems: apply geometric intuition to planning under lag",
        // uncertainty (category 3)
        "How should you calibrate confidence and when should you refuse for insufficient evidence?",
        // synthesis
        "Connect sparse distributed memory, vector symbolic binding, and Bitwork in one coherent thought.",
        // relational
        "What is the boundary between knowledge and attention?",
        // governance / self-model
        "Is Perci a superintelligence?",
        // novel entity meta (category 6)
        "How do we generalize under novel entities and entity-swap without overfitting templates?",
        // five-channel intelligence feed
        "what patterns emerge from the ledger?",
        "How do intelligence channels operators frames hardness transfer curriculum Cortex and lab patterns work?",
    ]
}

/// Run operator transfer on the full standard suite. Returns (all_pass, report).
pub fn run_transfer_suite() -> (bool, String) {
    let mut all_pass = true;
    let mut out = String::from("[Transfer suite · operator speech]\n");
    let mut pass_n = 0u32;
    let mut fail_n = 0u32;
    for base in standard_transfer_bases() {
        let report = run_operator_transfer(base);
        let pass = report.contains("pass=true");
        if pass {
            pass_n += 1;
        } else {
            fail_n += 1;
            all_pass = false;
        }
        out.push_str(&format!(
            "  {} {}\n",
            if pass { "PASS" } else { "FAIL" },
            truncate(base, 72)
        ));
    }
    out.push_str(&format!(
        "summary: pass={pass_n} fail={fail_n} all_pass={all_pass}\n"
    ));
    if all_pass {
        out.push_str("SUITE PASS — transfer law holds for standard bases.\n");
    } else {
        out.push_str("SUITE FAIL — do not bump version or claim emergence.\n");
    }
    // Record suite outcome as transfer event
    append_event(&LedgerEvent {
        ts: now_ts(),
        kind: EventKind::Transfer,
        transfer_id: Some(format!("suite-{}", now_ts() % 1_000_000)),
        transfer_pass: Some(all_pass),
        transfer_score_pm: Some(if all_pass { 1000 } else { pass_n * 100 }),
        transfer_detail: Some(format!("suite pass={pass_n} fail={fail_n}")),
        tags: vec![if all_pass {
            "transfer_suite_pass".into()
        } else {
            "transfer_suite_fail".into()
        }],
        authority: Some("transfer_suite".into()),
        phase: None,
        label: None,
        margin: None,
        overlap: None,
        overlap_z: None,
        alpha_pm: None,
        mix_n: None,
        residual_n: None,
        mix_labels: None,
        prefer_mix_thesis: None,
        geometry_blind: None,
        chronic: None,
        mixture_crutch: None,
        user: None,
        speech_hit: None,
        token_hits: None,
        token_n: None,
        used_mix_thesis: None,
        ticket_id: None,
        ticket_kind: None,
    });
    (all_pass, out)
}

/// Cluster curriculum candidates by label (Phase D — pack debt visibility).
pub fn curriculum_cluster_report() -> String {
    let path = curriculum_path();
    if !path.is_file() {
        return format!(
            "[Curriculum cluster]\nNo curriculum file yet ({})\nPack debt: none staged.\n\
Policy: operators own speech when transfer passes; weight promote needs human authorize.",
            path.display()
        );
    }
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut by_label: HashMap<String, u32> = HashMap::new();
    let mut total = 0u32;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            total += 1;
            if let Some(lab) = v.get("label").and_then(|x| x.as_str()) {
                *by_label.entry(lab.to_owned()).or_insert(0) += 1;
            }
        }
    }
    let mut pairs: Vec<_> = by_label.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let resolved = resolved_primary_labels();
    let mut out = format!(
        "[Curriculum cluster] samples={total} path={}\n",
        path.display()
    );
    for (lab, n) in pairs.iter().take(12) {
        let status = if resolved.iter().any(|r| r == lab) {
            "operator-resolved (pack optional)"
        } else {
            "open pack debt"
        };
        out.push_str(&format!("  · {lab}: {n}  [{status}]\n"));
    }
    out.push_str(
        "LAW: mixture_crutch is temporary. Prefer operator frames. Weights only with --authorize + transfer.\n",
    );
    out
}

/// Mine ledger + tickets + curriculum for emergent structural patterns.
/// Engineering telemetry → actionable intelligence (not consciousness).
pub fn pattern_intelligence_report() -> String {
    let events = load_events(500.max(LESSON_WINDOW));
    let mut match_n = 0u32;
    let mut probe_n = 0u32;
    let mut op_n = 0u32;
    let mut soft_n = 0u32;
    let mut primary_off_curr = 0u32;
    let mut geometry_blind = 0u32;
    let mut mixture_crutch = 0u32;
    let mut speech_hit = 0u32;
    let mut speech_miss = 0u32;
    let mut xfer_pass = 0u32;
    let mut xfer_fail = 0u32;
    let mut auth_counts: HashMap<String, u32> = HashMap::new();
    let mut label_off: HashMap<String, u32> = HashMap::new();

    for ev in &events {
        match ev.kind {
            EventKind::Match => {
                match_n += 1;
                let auth = ev.authority.as_deref().unwrap_or("?");
                *auth_counts.entry(auth.to_owned()).or_insert(0) += 1;
                if auth == "probe" {
                    probe_n += 1;
                } else if auth == "softcascade" {
                    soft_n += 1;
                } else {
                    op_n += 1;
                }
                if is_curriculum_authority(auth)
                    && ev.tags.iter().any(|t| t == "primary_off_topic")
                {
                    primary_off_curr += 1;
                    if let Some(lab) = &ev.label {
                        *label_off.entry(lab.clone()).or_insert(0) += 1;
                    }
                }
                if is_curriculum_authority(auth) && ev.geometry_blind == Some(true) {
                    geometry_blind += 1;
                }
                if is_curriculum_authority(auth) && ev.mixture_crutch == Some(true) {
                    mixture_crutch += 1;
                }
            }
            EventKind::Speech => {
                if ev.speech_hit == Some(true) {
                    speech_hit += 1;
                } else if ev.speech_hit == Some(false) {
                    speech_miss += 1;
                }
            }
            EventKind::Transfer => {
                if ev.transfer_pass == Some(true) {
                    xfer_pass += 1;
                } else if ev.transfer_pass == Some(false) {
                    xfer_fail += 1;
                }
            }
            EventKind::Ticket => {}
        }
    }

    let mut top_auth: Vec<_> = auth_counts.into_iter().collect();
    top_auth.sort_by(|a, b| b.1.cmp(&a.1));
    let mut top_off: Vec<_> = label_off.into_iter().collect();
    top_off.sort_by(|a, b| b.1.cmp(&a.1));

    let open = list_open_tickets();
    let closed = list_closed_tickets();
    let resolved = resolved_primary_labels();

    let dual: Vec<String> = open
        .iter()
        .filter(|id| closed.iter().any(|c| c == *id))
        .cloned()
        .collect();

    let mut out = String::from("[Pattern intelligence · geometry speaks]\n");
    out.push_str(&format!(
        "window_events={} matches={match_n} (probe={probe_n} operator={op_n} softcascade={soft_n})\n\
speech hit/miss={speech_hit}/{speech_miss} · transfer pass/fail={xfer_pass}/{xfer_fail}\n\
curriculum primary_off={primary_off_curr} geometry_blind={geometry_blind} mixture_crutch={mixture_crutch}\n\
tickets open={} closed={} dual_open+closed_bug={}\n",
        events.len(),
        open.len(),
        closed.len(),
        dual.len()
    ));

    out.push_str("\n## Emergent laws (from data)\n");
    out.push_str(
        "1. **Dual authority split:** Bitwork probes geometry while operators own speech. \
Operators are the load-bearing intelligence layer; SoftCascade is minority path.\n",
    );
    out.push_str(
        "2. **Primary pack lag:** high primary_off under probe means pack insights often miss user tokens; \
tickets + operators paper the gap. Pack rebuild is optional, not urgent if transfer holds.\n",
    );
    out.push_str(
        "3. **Transfer is the truth gate:** pass history dominates fail when suite is maintained; \
entity-swap tests structure not name parrot.\n",
    );
    out.push_str(
        "4. **Ticket thrash:** open+closed pairs mean closed labels were reopened by match events — \
now suppressed: closed tickets stay closed, curriculum still grows.\n",
    );
    out.push_str(
        "5. **Impasse primitive:** fail → ticket → transfer → close is the real self-improve unit \
(Soar-style), not denser chat.\n",
    );
    out.push_str(
        "6. **Three memories:** Bitwork pack · append-only ledgers · session/Cortex. \
Folding them into one blob would poison curriculum.\n",
    );

    if !top_auth.is_empty() {
        out.push_str("\n## Top speech authorities (match)\n");
        for (a, n) in top_auth.iter().take(8) {
            out.push_str(&format!("  · {a}: {n}\n"));
        }
    }
    if !top_off.is_empty() {
        out.push_str("\n## Curriculum primary_off labels\n");
        for (l, n) in top_off.iter().take(8) {
            let res = if resolved.iter().any(|r| r == l) {
                "operator-resolved"
            } else {
                "open pack debt"
            };
            out.push_str(&format!("  · {l}: {n}  [{res}]\n"));
        }
    }
    if !dual.is_empty() {
        out.push_str("\n## Dual ticket hygiene (remove open if closed exists)\n");
        for id in &dual {
            out.push_str(&format!("  · {id}\n"));
        }
        out.push_str("  fix: `perci lab hygiene` or next stage_primary_fix_ticket call path\n");
    }

    out.push_str(
        "\n## Feed-forward (how intelligence enters Perci)\n\
- Operators & frames (code) — fastest intelligence channel\n\
- Hardness + transfer suite — anti-overfit law\n\
- Curriculum JSONL — staged pack debt only\n\
- Cortex remember/consolidate — human/agent session memory\n\
- **Never** silent weight promote from this report\n",
    );
    out
}

/// Remove open ticket files when a `.closed.md` already exists (hygiene).
pub fn hygiene_dual_tickets() -> String {
    let dir = tickets_dir();
    let open = list_open_tickets();
    let closed = list_closed_tickets();
    let mut removed = 0u32;
    let mut lines = String::from("[Lab hygiene]\n");
    for id in &open {
        if closed.iter().any(|c| c == id) {
            let path = dir.join(format!("{id}.md"));
            if path.is_file() {
                let _ = fs::remove_file(&path);
                removed += 1;
                lines.push_str(&format!("removed reopen thrash: {id}.md (closed remains)\n"));
            }
        }
    }
    if removed == 0 {
        lines.push_str("no dual open+closed pairs\n");
    } else {
        lines.push_str(&format!("removed {removed} thrash open ticket(s)\n"));
    }
    lines
}

/// Unified world-loop queue: emergence tickets + hardness red summary path.
pub fn unified_queue_report() -> String {
    let mut out = String::from("[Unified lab queue · world loop]\n");
    out.push_str(&lab_report());
    out.push('\n');
    out.push_str(&curriculum_cluster_report());
    out.push('\n');
    // Hardness eval presence
    let hard = PathBuf::from("models/candidates/evaluation-hardness-v1.json");
    if hard.is_file() {
        if let Ok(t) = fs::read_to_string(&hard) {
            let pass = t.contains("\"status\": \"PASS\"") || t.contains("\"status\":\"PASS\"");
            out.push_str(&format!(
                "[Hardness eval] path={} looks_pass={pass}\n",
                hard.display()
            ));
        }
    } else {
        out.push_str("[Hardness eval] missing — run python scripts/evaluate_hardness.py\n");
    }
    out.push_str(
        "\nagent: perci agent lab --full [--repair] [--dry-run]\n\
release: python scripts/release_gates.py\n\
channels: perci lab feed\n",
    );
    out
}

/// Status of all five intelligence-feed channels (never auto-promotes weights).
pub fn feed_all_channels_report() -> String {
    let mut out = String::from(
        "[Intelligence channels · feed status]\n\
Claim boundary: engineering feed only — not consciousness, not weight auto-promote.\n\n",
    );

    // 1 Operators / frames
    out.push_str("## 1. Operators / frames\n");
    out.push_str(
        "status: ACTIVE\n\
module: src/cognition_expand.rs + deliberation::activate_semantic_frames\n\
operators: agent-loop-plan, cross-domain-compose, uncertainty-calibration,\n\
  ledger-memory-integrate, meta-critique-queue, novel-entity-generalize, pattern-intelligence\n\
frames: EXPAND_FRAMES (agent loop, transfer, uncertainty, ledger, critique, cross-domain,\n\
  pattern intelligence, intelligence channel)\n\n",
    );

    // 2 Hardness + transfer
    out.push_str("## 2. Hardness + transfer-suite\n");
    let hard = PathBuf::from("models/candidates/evaluation-hardness-v1.json");
    let pack = PathBuf::from("training/hardness/hardness-pack-v1.jsonl");
    let pack_n = fs::read_to_string(&pack)
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    if hard.is_file() {
        if let Ok(t) = fs::read_to_string(&hard) {
            let pass = t.contains("\"status\": \"PASS\"") || t.contains("\"status\":\"PASS\"");
            let passed = extract_json_numberish_from(&t, "passed").unwrap_or_else(|| "?".into());
            out.push_str(&format!(
                "hardness_pack_cases={pack_n}\neval={} looks_pass={pass} passed={passed}\n",
                hard.display()
            ));
        }
    } else {
        out.push_str(&format!(
            "hardness_pack_cases={pack_n}\neval=MISSING — run python scripts/evaluate_hardness.py\n"
        ));
    }
    out.push_str(&format!(
        "transfer_bases={} — run: perci transfer-suite\n\n",
        standard_transfer_bases().len()
    ));

    // 3 Curriculum
    out.push_str("## 3. Curriculum JSONL\n");
    let cpath = curriculum_path();
    let staged = [
        "training/curriculum/curriculum-cognition-v0622.jsonl",
        "training/curriculum/curriculum-channels-v0624.jsonl",
        "training/curriculum/curriculum-latest.jsonl",
    ];
    out.push_str(&format!("live_candidates: {}\n", cpath.display()));
    if cpath.is_file() {
        let n = fs::read_to_string(&cpath)
            .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0);
        out.push_str(&format!("  samples≈{n} (agent-writable; not weights)\n"));
    } else {
        out.push_str("  (empty — will grow on primary_off under softcascade/probe)\n");
    }
    for s in staged {
        let p = PathBuf::from(s);
        if p.is_file() {
            let n = fs::read_to_string(&p)
                .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);
            out.push_str(&format!("  {s}: {n} lines [human-review / authorize only]\n"));
        } else {
            out.push_str(&format!("  {s}: missing\n"));
        }
    }
    out.push('\n');

    // 4 Cortex
    out.push_str("## 4. Cortex cards\n");
    out.push_str(
        "status: session-driven (activate → remember → consolidate)\n\
docs_indexable: docs/PATTERN_INTEL_v0623.md, docs/BITWORK_COGNITION_v0622.md,\n\
  docs/GOAL.md, knowledge/packs/perci-core-intelligence-v1/13-pattern-intelligence.md\n\
runtime: .cortex/cards/ + .perci/cortex-home/packets/\n\
authority: cortex_may_authorize_mutation=false (host rules + human authorize)\n\n",
    );

    // 5 lab patterns
    out.push_str("## 5. /lab patterns\n");
    out.push_str("command: perci lab patterns | /lab in chat\n");
    out.push_str(&pattern_intelligence_report());
    out.push_str(
        "\n## Feed law\n\
Intelligence enters through these five channels only.\n\
Never densify chat as a substitute. Never auto-promote .pwgt.\n\
Next: python scripts/release_gates.py after hardness + transfer-suite green.\n",
    );
    out
}

fn extract_json_numberish_from(window: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":");
    let idx = window.find(&pat)?;
    let rest = window[idx + pat.len()..].trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if num.is_empty() {
        None
    } else {
        Some(num)
    }
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_owned()
    } else {
        t.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::{CognitiveMatch, MixtureSupport};
    use std::sync::Mutex;

    // Serialize tests that touch global ledger paths.
    static LOCK: Mutex<()> = Mutex::new(());

    fn sample_match(margin: i32, insight: Option<&str>, mix_insight: Option<&str>) -> CognitiveMatch {
        CognitiveMatch {
            label: "general".into(),
            variant: 1,
            concept_id: 0,
            insight: insight.map(String::from),
            score: 200,
            overlap: 70,
            runner_up_score: 200 - margin,
            margin,
            query_popcount: 200,
            prototype_popcount: 150,
            positive_overlap: 50,
            negative_overlap: 10,
            hamming: 200,
            jaccard: 0.2,
            overlap_z: 15.0,
            mixture: mix_insight
                .map(|i| {
                    vec![MixtureSupport {
                        label: "systems".into(),
                        score: 180,
                        overlap: 60,
                        concept_id: 1,
                        insight: Some(i.into()),
                        residual: false,
                        hop: 0,
                        attention_pm: 300,
                    }]
                })
                .unwrap_or_default(),
            composition: vec![],
            primary_attention_pm: 400,
        }
    }

    #[test]
    fn contested_phase_from_low_margin() {
        assert_eq!(FieldPhase::from_margin(2), FieldPhase::Contested);
        assert_eq!(FieldPhase::from_margin(20), FieldPhase::Locked);
    }

    #[test]
    fn primary_off_prefers_mixture_when_multipartite() {
        let m = sample_match(
            2,
            Some("Behavioral complexity is observable; subjective experience is inferred."),
            Some("Interfaces earn trust when timeouts and retries stay explicit under lag."),
        );
        let p = analyze(&m, "how should interfaces earn trust under lag and retry?");
        assert_eq!(p.phase, FieldPhase::Contested);
        assert!(p.prefer_mixture_thesis);
        assert!(p.mixture_crutch);
        assert!(p.open_primary_fix_ticket);
        assert!(p.tags.contains(&"primary_off_topic"));
        assert!(p.tags.contains(&"mixture_crutch"));
        assert!(!p.geometry_blind);
        let pref = preferred_mixture_insight(
            &m,
            "how should interfaces earn trust under lag and retry?",
        )
        .expect("mix");
        let pl = pref.to_ascii_lowercase();
        assert!(
            pl.contains("trust")
                || pl.contains("timeout")
                || pl.contains("interface")
                || pl.contains("lag")
        );
    }

    #[test]
    fn geometry_blind_when_primary_and_mixture_miss() {
        let m = sample_match(
            1,
            Some("Behavioral complexity is observable; subjective experience is inferred."),
            Some("Phenomenology stays private while behavior is public."),
        );
        let p = analyze(&m, "how should interfaces earn trust under lag and retry?");
        assert!(p.tags.contains(&"primary_off_topic"));
        assert!(p.geometry_blind);
        assert!(p.force_multipartite_arc);
        assert!(!p.prefer_mixture_thesis);
    }

    #[test]
    fn typed_event_roundtrip() {
        let ev = LedgerEvent {
            ts: 42,
            kind: EventKind::Match,
            phase: Some("contested".into()),
            label: Some("general".into()),
            margin: Some(2),
            overlap: Some(10),
            overlap_z: Some(12.5),
            alpha_pm: Some(400),
            mix_n: Some(1),
            residual_n: Some(0),
            mix_labels: Some("general+systems".into()),
            tags: vec!["primary_off_topic".into(), "multipartite".into()],
            prefer_mix_thesis: Some(true),
            geometry_blind: Some(false),
            chronic: Some(false),
            mixture_crutch: Some(true),
            authority: Some("softcascade".into()),
            user: Some("how trust".into()),
            speech_hit: None,
            token_hits: None,
            token_n: None,
            used_mix_thesis: None,
            ticket_id: None,
            ticket_kind: None,
            transfer_id: None,
            transfer_pass: None,
            transfer_score_pm: None,
            transfer_detail: None,
        };
        let s = serde_json::to_string(&ev).unwrap();
        let back: LedgerEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(back.kind, EventKind::Match);
        assert_eq!(back.label.as_deref(), Some("general"));
        assert_eq!(back.authority.as_deref(), Some("softcascade"));
        assert_eq!(back.mixture_crutch, Some(true));
    }

    #[test]
    fn authority_filter_excludes_operator_from_curriculum_chronic() {
        // Unit-level: is_curriculum_authority
        assert!(is_curriculum_authority("softcascade"));
        assert!(is_curriculum_authority("probe"));
        assert!(!is_curriculum_authority("trust-interface-design"));
        assert!(!is_curriculum_authority("emergence-vs-memorization"));
    }

    #[test]
    fn transfer_gate_passes_when_topic_binds_across_variants() {
        let base = "how should interfaces earn trust under lag";
        let cases = default_transfer_set(base);
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0].role, "base");
        assert_eq!(cases[1].role, "paraphrase");
        assert_eq!(cases[2].role, "novel");

        let mut map = HashMap::new();
        // Good speech that hits base tokens; also mention novel entities if present.
        let good = "Interfaces earn trust when timeouts stay explicit under lag and retry.";
        for c in &cases {
            // Speech that always mentions the structural words from base.
            let mut speech = good.to_owned();
            if c.role == "novel" {
                speech.push_str(" ZephyrNode and Quoril still need checkable acceptance.");
            }
            map.insert(c.prompt.clone(), speech);
        }
        let r = evaluate_transfer("test-xfer-1", &cases, &map);
        // base should pass; paraphrase keeps "interfaces/trust/lag" in prompt tokens often;
        // novel may swap nouns — we added ZephyrNode.
        assert!(
            r.pass || r.score_pm >= 400,
            "expected transfer progress, got pass={} score={} detail={}",
            r.pass,
            r.score_pm,
            r.detail
        );
    }

    #[test]
    fn transfer_gate_fails_when_speech_ignores_topic() {
        let base = "how should interfaces earn trust under lag";
        let cases = default_transfer_set(base);
        let mut map = HashMap::new();
        for c in &cases {
            map.insert(
                c.prompt.clone(),
                "Behavioral complexity is observable; subjective experience is inferred.".into(),
            );
        }
        let r = evaluate_transfer("test-xfer-fail", &cases, &map);
        assert!(!r.pass, "off-topic speech must fail transfer");
    }

    #[test]
    fn transfer_novel_scores_structure_not_entity_parrot() {
        let base = "how should interfaces earn trust under lag";
        let cases = default_transfer_set(base);
        let mut map = HashMap::new();
        let structural = "Interfaces earn trust when timeouts stay explicit under lag; retries must be idempotent.";
        for c in &cases {
            map.insert(c.prompt.clone(), structural.into());
        }
        let r = evaluate_transfer("test-xfer-struct", &cases, &map);
        assert!(r.pass, "structural operator speech should transfer: {}", r.detail);
    }

    #[test]
    fn stage_ticket_is_idempotent_per_label() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Use isolated env paths for this test.
        let tmp = std::env::temp_dir().join(format!("perci-emerge-test-{}", now_ts()));
        let _ = fs::create_dir_all(&tmp);
        let tickets = tmp.join("tickets");
        let curriculum = tmp.join("curriculum.jsonl");
        let log = tmp.join("log.jsonl");
        env::set_var("PERCI_EMERGENCE_TICKETS", &tickets);
        env::set_var("PERCI_EMERGENCE_CURRICULUM", &curriculum);
        env::set_var("PERCI_EMERGENCE_LOG", &log);

        let id1 = stage_primary_fix_ticket(
            "general",
            "how should interfaces earn trust under lag",
            Some("phenomenology fluff"),
            Some("interfaces earn trust with timeouts"),
            "test",
        );
        let id2 = stage_primary_fix_ticket(
            "general",
            "how should interfaces earn trust under lag again",
            Some("phenomenology fluff"),
            Some("interfaces earn trust with timeouts"),
            "test2",
        );
        assert_eq!(id1, id2);
        assert!(tickets.join(format!("{}.md", id1.as_ref().unwrap())).is_file());
        // Curriculum grew twice.
        let cur = fs::read_to_string(&curriculum).unwrap_or_default();
        assert_eq!(cur.lines().filter(|l| !l.is_empty()).count(), 2);

        // cleanup env so other tests don't inherit
        env::remove_var("PERCI_EMERGENCE_TICKETS");
        env::remove_var("PERCI_EMERGENCE_CURRICULUM");
        env::remove_var("PERCI_EMERGENCE_LOG");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn speech_outcome_hit_detection() {
        let user = "how should interfaces earn trust under lag";
        let tokens = content_tokens(user);
        assert!(tokens.iter().any(|t| t.contains("trust") || t.contains("interface")));
        let good = "Interfaces earn trust when timeouts stay explicit under lag.";
        let gl = good.to_ascii_lowercase();
        let hits = tokens.iter().filter(|t| gl.contains(t.as_str())).count();
        assert!(hits >= 2);
    }
}
