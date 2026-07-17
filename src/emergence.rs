//! Geometry emergence ledger — the field records itself and can speak back.
//!
//! Append-only JSONL of Bitwork geometry events (margin phase, α, z-score, multipartite
//! mass). Never mutates weights. Used for:
//! - `/field` inspection
//! - SoftCascade policy (prefer mixture thesis when primary is off-topic)
//! - future hardness / agent lab clustering
//!
//! Claim boundary: engineering telemetry, not consciousness.

use crate::cognitive::{CognitiveMatch, MixtureSupport};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default path under models/candidates (agent-writable, not the weight pack).
pub fn default_path() -> PathBuf {
    env::var_os("PERCI_EMERGENCE_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/emergence-geometry.jsonl"))
}

/// Field phase from margin (locked / soft / contested).
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
    /// Lead geometry is chronically off for this label (ledger lesson).
    pub chronic_label_bias: bool,
    /// No mixture insight hits user tokens either — field is blind; lean residual/check.
    pub geometry_blind: bool,
    pub tags: Vec<&'static str>,
}

/// Rolling lessons from the emergence ledger — geometry teaching the system.
#[derive(Clone, Debug, Default)]
pub struct EvolutionHints {
    pub total_events: usize,
    pub contested_n: usize,
    pub multipartite_n: usize,
    pub primary_off_n: usize,
    pub speech_miss_n: usize,
    pub speech_hit_n: usize,
    /// Labels with ≥2 recent primary_off_topic events (need operator/curriculum).
    pub chronic_off_labels: Vec<String>,
    /// Short human lines for /field and evolve reports.
    pub recommendations: Vec<String>,
}

/// Analyze a CognitiveMatch and produce feedback policy.
///
/// Consults the ledger so chronic failures bias the current turn (geometry speaks back).
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

    // Prefer mixture when primary misses user tokens and field is multipartite,
    // or when this label has been chronically off in the ledger (learn from history).
    let prefer_mixture_thesis =
        (primary_off && multipartite && mix_on) || (chronic && multipartite && mix_on);
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
        tags,
    }
}

/// Primary insight does not touch user content tokens → geometry says "don't lead with this."
fn primary_insight_off_topic(matched: &CognitiveMatch, user: &str) -> bool {
    let Some(insight) = matched.insight.as_ref() else {
        return matched.margin < 8; // no insight + soft/contested → prefer mix
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

fn content_tokens(user: &str) -> Vec<String> {
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

/// Minimum recent primary_off_topic hits for a label to count as chronic.
const CHRONIC_OFF_THRESHOLD: usize = 2;
/// Window of ledger events used for lessons / chronic bias.
const LESSON_WINDOW: usize = 48;

/// True when this label has repeatedly led with off-topic primary insight.
fn label_is_chronic_off(label: &str) -> bool {
    let hints = lessons(LESSON_WINDOW);
    hints
        .chronic_off_labels
        .iter()
        .any(|l| l.eq_ignore_ascii_case(label))
}

/// Derive rolling lessons from the emergence ledger so geometry can teach policy.
pub fn lessons(window: usize) -> EvolutionHints {
    let rows = recent(window).unwrap_or_default();
    let mut hints = EvolutionHints {
        total_events: rows.len(),
        ..Default::default()
    };
    // label → primary_off count
    let mut off_by_label: Vec<(String, usize)> = Vec::new();
    for r in &rows {
        if r.contains("\"kind\":\"speech\"") {
            if r.contains("\"speech_hit\":true") || r.contains("\"tags\":\"speech_hit") {
                hints.speech_hit_n += 1;
            } else if r.contains("\"speech_hit\":false") || r.contains("speech_miss") {
                hints.speech_miss_n += 1;
            }
            continue;
        }
        if r.contains("\"phase\":\"contested\"") {
            hints.contested_n += 1;
        }
        if r.contains("multipartite") {
            hints.multipartite_n += 1;
        }
        if r.contains("primary_off_topic") {
            hints.primary_off_n += 1;
            if let Some(lab) = extract_json_string_field(r, "label") {
                bump_count(&mut off_by_label, &lab);
            }
        }
    }
    for (lab, n) in off_by_label {
        if n >= CHRONIC_OFF_THRESHOLD {
            hints.chronic_off_labels.push(lab);
        }
    }
    hints.chronic_off_labels.sort();
    hints.chronic_off_labels.dedup();

    if !hints.chronic_off_labels.is_empty() {
        hints.recommendations.push(format!(
            "chronic primary_off labels → prefer mixture / add operator frames: {}",
            hints.chronic_off_labels.join(", ")
        ));
    }
    if hints.contested_n * 2 > hints.total_events.max(1) && hints.total_events >= 4 {
        hints.recommendations.push(
            "majority contested field → keep multipartite arcs and residual critique".into(),
        );
    }
    if hints.speech_miss_n > hints.speech_hit_n && hints.speech_miss_n + hints.speech_hit_n >= 3 {
        hints.recommendations.push(
            "speech often misses user tokens → tighten topic bind / mixture thesis".into(),
        );
    }
    if hints.primary_off_n >= 3 && hints.recommendations.is_empty() {
        hints.recommendations.push(
            "repeated primary_off_topic → curriculum or operator for those regions".into(),
        );
    }
    if hints.recommendations.is_empty() && hints.total_events > 0 {
        hints
            .recommendations
            .push("field stable enough — keep measuring, no forced policy shift".into());
    }
    hints
}

fn bump_count(map: &mut Vec<(String, usize)>, key: &str) {
    if let Some((_, n)) = map.iter_mut().find(|(k, _)| k == key) {
        *n += 1;
    } else {
        map.push((key.to_owned(), 1));
    }
}

/// Lightweight JSON string field extract (ledger lines are flat, single-line).
fn extract_json_string_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let i = line.find(&needle)?;
    let rest = &line[i + needle.len()..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => break,
            c => out.push(c),
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Record geometry from a classify match. Best-effort; never fails chat.
pub fn record_match(user: &str, matched: &CognitiveMatch, speech_authority: &str) {
    let policy = analyze(matched, user);
    let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
    let mix_labels: Vec<&str> = {
        let mut v = vec![matched.label.as_str()];
        for m in &matched.mixture {
            if !v.contains(&m.label.as_str()) {
                v.push(m.label.as_str());
            }
        }
        v
    };
    let path = default_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let user_esc = json_escape(&truncate(user, 160));
    let label_esc = json_escape(&matched.label);
    let phase = policy.phase.as_str();
    let tags = policy.tags.join(",");
    let mix = mix_labels.join("+");
    let line = format!(
        "{{\"ts\":{ts},\"kind\":\"match\",\"phase\":\"{phase}\",\"label\":{label_esc},\"margin\":{},\"overlap\":{},\"overlap_z\":{:.3},\"alpha_pm\":{},\"mix_n\":{},\"residual_n\":{residual_n},\"mix_labels\":{},\"tags\":{},\"prefer_mix_thesis\":{},\"geometry_blind\":{},\"chronic\":{},\"authority\":{},\"user\":{user_esc}}}\n",
        matched.margin,
        matched.overlap,
        matched.overlap_z,
        matched.primary_attention_pm,
        matched.mixture.len(),
        json_escape(&mix),
        json_escape(&tags),
        if policy.prefer_mixture_thesis {
            "true"
        } else {
            "false"
        },
        if policy.geometry_blind { "true" } else { "false" },
        if policy.chronic_label_bias {
            "true"
        } else {
            "false"
        },
        json_escape(speech_authority),
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
    // Session cache for SoftCascade feedback
    LAST_POLICY.with(|c| {
        *c.borrow_mut() = Some(policy);
    });
}

/// After SoftCascade speaks, record whether the body still touches user content.
/// This closes the loop: geometry → policy → speech → measured outcome.
pub fn record_speech_outcome(user: &str, speech: &str, used_mix_thesis: bool) {
    let tokens = content_tokens(user);
    let sl = speech.to_ascii_lowercase();
    let hits = tokens
        .iter()
        .filter(|t| t.len() >= 4 && sl.contains(t.as_str()))
        .count();
    let need = tokens.len().min(2).max(1);
    let speech_hit = tokens.is_empty() || hits >= need;
    let path = default_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let tag = if speech_hit {
        "speech_hit"
    } else {
        "speech_miss"
    };
    let line = format!(
        "{{\"ts\":{ts},\"kind\":\"speech\",\"speech_hit\":{},\"token_hits\":{},\"token_n\":{},\"used_mix_thesis\":{},\"tags\":{},\"user\":{}}}\n",
        if speech_hit { "true" } else { "false" },
        hits,
        tokens.len(),
        if used_mix_thesis { "true" } else { "false" },
        json_escape(tag),
        json_escape(&truncate(user, 120)),
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

use std::cell::RefCell;

thread_local! {
    static LAST_POLICY: RefCell<Option<GeometryPolicy>> = RefCell::new(None);
}

/// Last geometry policy for this process (SoftCascade feedback).
pub fn last_policy() -> Option<GeometryPolicy> {
    LAST_POLICY.with(|c| c.borrow().clone())
}

/// Set policy without logging (tests / SoftCascade after analyze).
pub fn set_session_policy(policy: GeometryPolicy) {
    LAST_POLICY.with(|c| *c.borrow_mut() = Some(policy));
}

/// Recent emergence lines for `/field`.
pub fn recent(limit: usize) -> io::Result<Vec<String>> {
    let path = default_path();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let mut lines: Vec<String> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

/// Human-readable field summary for `/field` — geometry speaks back with lessons.
pub fn status_report(limit: usize) -> String {
    let path = default_path();
    match recent(limit) {
        Ok(rows) if rows.is_empty() => format!(
            "[Field · emergence]\nNo geometry events yet.\nLog: {}\nAfter classify/chat, contested multipartite turns will append here.\n\
Geometry will learn: primary_off_topic → mixture thesis; chronic labels → multipartite force; speech_miss → tighten bind.",
            path.display()
        ),
        Ok(rows) => {
            let mut contested = 0u32;
            let mut multipartite = 0u32;
            let mut primary_off = 0u32;
            let mut high_z = 0u32;
            let mut geometry_blind = 0u32;
            let mut speech_hit = 0u32;
            let mut speech_miss = 0u32;
            for r in &rows {
                if r.contains("\"kind\":\"speech\"") {
                    if r.contains("\"speech_hit\":true") {
                        speech_hit += 1;
                    } else if r.contains("\"speech_hit\":false") {
                        speech_miss += 1;
                    }
                    continue;
                }
                if r.contains("\"phase\":\"contested\"") {
                    contested += 1;
                }
                if r.contains("multipartite") {
                    multipartite += 1;
                }
                if r.contains("primary_off_topic") {
                    primary_off += 1;
                }
                if r.contains("high_z") {
                    high_z += 1;
                }
                if r.contains("geometry_blind") {
                    geometry_blind += 1;
                }
            }
            let hints = lessons(limit.max(LESSON_WINDOW));
            let mut out = format!(
                "[Field · emergence] last {} events · log {}\n\
counts: contested={contested} multipartite={multipartite} primary_off={primary_off} high_z={high_z} geometry_blind={geometry_blind}\n\
speech: hit={speech_hit} miss={speech_miss}\n\
policy: primary misses user tokens + multipartite → mixture thesis; chronic label → force multipartite + critique\n",
                rows.len(),
                path.display()
            );
            if !hints.chronic_off_labels.is_empty() {
                out.push_str(&format!(
                    "chronic_off_labels: {}\n",
                    hints.chronic_off_labels.join(", ")
                ));
            }
            if !hints.recommendations.is_empty() {
                out.push_str("lessons (geometry → evolve):\n");
                for rec in &hints.recommendations {
                    out.push_str(&format!("  · {rec}\n"));
                }
            }
            out.push_str("---\n");
            for r in rows.iter().rev().take(8) {
                out.push_str(r);
                out.push('\n');
            }
            out
        }
        Err(e) => format!("[Field · emergence] unavailable: {e}"),
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

fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::{CognitiveMatch, MixtureSupport};

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
        assert!(p.tags.contains(&"primary_off_topic"));
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
        assert!(p.lower_critique_threshold);
        assert!(!p.prefer_mixture_thesis); // no usable mix thesis
    }

    #[test]
    fn extract_label_from_ledger_line() {
        let line = r#"{"ts":1,"kind":"match","label":"general","tags":"primary_off_topic"}"#;
        assert_eq!(
            extract_json_string_field(line, "label").as_deref(),
            Some("general")
        );
    }

    #[test]
    fn speech_outcome_hit_detection() {
        // Unit-level: content_tokens + hit logic without writing the global log path.
        let user = "how should interfaces earn trust under lag";
        let tokens = content_tokens(user);
        assert!(tokens.iter().any(|t| t.contains("trust") || t.contains("interface")));
        let good = "Interfaces earn trust when timeouts stay explicit under lag.";
        let gl = good.to_ascii_lowercase();
        let hits = tokens
            .iter()
            .filter(|t| gl.contains(t.as_str()))
            .count();
        assert!(hits >= 2);
    }
}
