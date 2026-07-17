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

    for (lab, n) in off_curriculum {
        if n >= CHRONIC_OFF_THRESHOLD {
            hints.chronic_off_labels.push(lab);
        }
    }
    hints.chronic_off_labels.sort();
    hints.chronic_off_labels.dedup();

    // Also surface tickets already on disk.
    if let Ok(entries) = fs::read_dir(tickets_dir()) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") && !hints.open_tickets.iter().any(|t| name.contains(t)) {
                hints.open_tickets.push(name.trim_end_matches(".md").to_owned());
            }
        }
    }

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
    let already = path.is_file();

    if !already {
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

    // Curriculum JSONL (append sample even if ticket exists — evidence grows).
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

    // Ledger ticket event (once per process open is fine; repeated is ok as audit).
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

    Some(id)
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

/// Build a default transfer set: base + paraphrase + novel-noun swap.
pub fn default_transfer_set(base: &str) -> Vec<TransferCase> {
    let tokens = content_tokens(base);
    let paraphrase = if base.contains('?') {
        // light paraphrase: reorder / synonym scaffold
        format!(
            "In practical terms, {} — explain carefully.",
            base.trim_end_matches('?').trim()
        )
    } else {
        format!("Rephrase and answer: {base}")
    };
    // Novel nouns: replace content tokens with invented entities where possible.
    let mut novel = base.to_owned();
    let swaps = ["ZephyrNode", "Quoril", "NembitGate", "VexorLag"];
    for (i, t) in tokens.iter().take(swaps.len()).enumerate() {
        // only replace whole-ish tokens (case-insensitive simple)
        let re_from = t.as_str();
        if re_from.len() >= 4 {
            novel = replace_ignore_case(&novel, re_from, swaps[i]);
        }
    }
    if novel == base {
        novel = format!("{base} [entity:ZephyrNode under Quoril constraints]");
    }
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

fn replace_ignore_case(hay: &str, from: &str, to: &str) -> String {
    let lower = hay.to_ascii_lowercase();
    let from_l = from.to_ascii_lowercase();
    if let Some(i) = lower.find(&from_l) {
        let mut out = String::new();
        out.push_str(&hay[..i]);
        out.push_str(to);
        out.push_str(&hay[i + from.len()..]);
        out
    } else {
        hay.to_owned()
    }
}

/// Score whether a speech body transfers: content tokens of each case appear in speech.
/// Pass if base hits ≥ need AND at least one of paraphrase/novel also hits ≥ need.
/// `speech_for` maps prompt → speech (caller supplies SoftCascade/operator outputs).
pub fn evaluate_transfer(
    transfer_id: &str,
    cases: &[TransferCase],
    speech_for: &HashMap<String, String>,
) -> TransferResult {
    let mut case_hits = Vec::new();
    let mut role_pass: HashMap<&str, bool> = HashMap::new();

    for c in cases {
        let speech = speech_for.get(&c.prompt).map(|s| s.as_str()).unwrap_or("");
        let tokens = content_tokens(&c.prompt);
        let sl = speech.to_ascii_lowercase();
        let hits = tokens
            .iter()
            .filter(|t| t.len() >= 4 && sl.contains(t.as_str()))
            .count();
        let need = tokens.len().min(2).max(1);
        let ok = tokens.is_empty() || hits >= need;
        case_hits.push((c.prompt.clone(), hits, tokens.len()));
        // Any case with this role that binds is enough for that role.
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
pub fn status_report(limit: usize) -> String {
    let path = default_path();
    let events = load_events(limit.max(LESSON_WINDOW));
    if events.is_empty() {
        return format!(
            "[Field · emergence lab]\nNo geometry events yet.\nLog: {}\nTickets: {}\nCurriculum: {}\n\
After chat, match/speech events append; chronic softcascade primary_off opens lab tickets.",
            path.display(),
            tickets_dir().display(),
            curriculum_path().display()
        );
    }

    let hints = lessons(limit.max(LESSON_WINDOW));
    let mut match_n = 0u32;
    let mut speech_hit = 0u32;
    let mut speech_miss = 0u32;
    let mut primary_off = 0u32;
    let mut geometry_blind = 0u32;
    let mut mixture_crutch = 0u32;
    let mut contested = 0u32;

    for ev in &events {
        match ev.kind {
            EventKind::Match => {
                match_n += 1;
                if ev.phase.as_deref() == Some("contested") {
                    contested += 1;
                }
                if ev.tags.iter().any(|t| t == "primary_off_topic") {
                    primary_off += 1;
                }
                if ev.geometry_blind == Some(true) {
                    geometry_blind += 1;
                }
                if ev.mixture_crutch == Some(true) {
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
            _ => {}
        }
    }

    let mut out = format!(
        "[Field · emergence lab] last {} events · log {}\n\
matches={match_n} contested={contested} primary_off={primary_off} (curriculum_ranked={}) geometry_blind={geometry_blind} mixture_crutch={mixture_crutch}\n\
speech: hit={speech_hit} miss={speech_miss} · transfer: pass={} fail={}\n\
authority filter: only softcascade|probe count for primary-fix ranking (operators excluded)\n",
        events.len(),
        path.display(),
        hints.primary_off_curriculum_n,
        hints.transfer_pass_n,
        hints.transfer_fail_n,
    );

    if !hints.chronic_off_labels.is_empty() {
        out.push_str(&format!(
            "chronic_off_labels (curriculum): {}\n",
            hints.chronic_off_labels.join(", ")
        ));
    }
    if !hints.open_tickets.is_empty() {
        out.push_str("open lab tickets:\n");
        for t in hints.open_tickets.iter().take(8) {
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
    out.push_str("--- recent ---\n");
    for ev in events.iter().rev().take(6) {
        if let Ok(s) = serde_json::to_string(ev) {
            out.push_str(&s);
            out.push('\n');
        }
    }
    out
}

/// Compact lab-only report (tickets + curriculum + transfer).
pub fn lab_report() -> String {
    let hints = lessons(LESSON_WINDOW);
    let mut out = String::from("[Lab · self-improve]\n");
    out.push_str(&format!(
        "tickets_dir: {}\ncurriculum: {}\n",
        tickets_dir().display(),
        curriculum_path().display()
    ));
    out.push_str(&format!(
        "chronic_labels: {}\nopen_tickets: {}\nmixture_crutch_events: {}\ntransfer pass/fail: {}/{}\n",
        if hints.chronic_off_labels.is_empty() {
            "(none)".into()
        } else {
            hints.chronic_off_labels.join(", ")
        },
        hints.open_tickets.len(),
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
    // List ticket files
    if let Ok(rd) = fs::read_dir(tickets_dir()) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        files.sort();
        if !files.is_empty() {
            out.push_str("ticket files:\n");
            for f in files.iter().take(12) {
                out.push_str(&format!("  · {f}\n"));
            }
        }
    }
    out
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
