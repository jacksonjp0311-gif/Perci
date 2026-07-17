//! Governed interaction learning.
//!
//! Every bounded, non-sensitive exchange becomes a pending evidence record.
//! Safe presentation preferences can adapt immediately; facts, procedures, and
//! cognitive weights never auto-promote from conversation alone.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FIELD_CHARS: usize = 1200;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DialogueProfile {
    pub interaction_count: u64,
    pub feedback_count: u64,
    pub teaching_candidate_count: u64,
    pub prefer_concise: bool,
    pub avoid_structured_chat: bool,
    pub prefer_warmth: bool,
    pub prefer_direct_answers: bool,
    pub prefer_explanations: bool,
}

/// Counts from the append-only event log (may exceed profile counters).
#[derive(Clone, Debug, Default)]
pub struct EventLogStats {
    pub total: u64,
    pub observations: u64,
    pub teaching: u64,
    pub other: u64,
}

#[derive(Clone, Debug)]
pub struct InteractionLearner {
    events_path: PathBuf,
    profile_path: PathBuf,
    profile: DialogueProfile,
}

impl InteractionLearner {
    pub fn discover() -> Self {
        let events_path = env::var_os("PERCI_LEARNING")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("memory/interaction-learning.jsonl"));
        let profile_path = env::var_os("PERCI_DIALOGUE_PROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("memory/dialogue-profile.json"));
        let mut learner = Self::new(events_path, profile_path);
        // Best-effort reconcile so status never under-reports vs the log.
        let _ = learner.reconcile_counters();
        learner
    }

    pub fn new(events_path: impl Into<PathBuf>, profile_path: impl Into<PathBuf>) -> Self {
        let events_path = events_path.into();
        let profile_path = profile_path.into();
        let profile = fs::read_to_string(&profile_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self {
            events_path,
            profile_path,
            profile,
        }
    }

    pub fn profile(&self) -> &DialogueProfile {
        &self.profile
    }

    pub fn events_path(&self) -> &Path {
        &self.events_path
    }

    pub fn status_label(&self) -> String {
        let stats = self.event_log_stats();
        format!(
            "active · interactions={} · event_log={} (obs={} teach={} other={}) · feedback={} · teach_candidates={} · direct={} · explain={} · concise={} · structured_chat={} · pending review",
            self.profile.interaction_count,
            stats.total,
            stats.observations,
            stats.teaching,
            stats.other,
            self.profile.feedback_count,
            self.profile.teaching_candidate_count,
            self.profile.prefer_direct_answers,
            self.profile.prefer_explanations,
            self.profile.prefer_concise,
            !self.profile.avoid_structured_chat,
        )
    }

    /// Count JSONL event kinds so profile vs log drift is visible (and reparable).
    pub fn event_log_stats(&self) -> EventLogStats {
        let mut stats = EventLogStats::default();
        if !self.events_path.is_file() {
            return stats;
        }
        let Ok(file) = fs::File::open(&self.events_path) else {
            return stats;
        };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
                stats.other = stats.other.saturating_add(1);
                stats.total = stats.total.saturating_add(1);
                continue;
            };
            stats.total = stats.total.saturating_add(1);
            match event.get("signal").and_then(|v| v.as_str()).unwrap_or("") {
                "explicit_teaching" => stats.teaching = stats.teaching.saturating_add(1),
                // Turn rows (observe()) always carry a user field; teaching does not.
                _ if event.get("user").is_some() => {
                    stats.observations = stats.observations.saturating_add(1);
                }
                _ => stats.other = stats.other.saturating_add(1),
            }
        }
        stats
    }

    /// Align profile.interaction_count with observation-like events when log is ahead.
    /// Does not rewrite history; only lifts the profile counter so status is honest.
    pub fn reconcile_counters(&mut self) -> io::Result<bool> {
        let stats = self.event_log_stats();
        let mut changed = false;
        if stats.observations > self.profile.interaction_count {
            self.profile.interaction_count = stats.observations;
            changed = true;
        }
        if stats.teaching > self.profile.teaching_candidate_count {
            self.profile.teaching_candidate_count = stats.teaching;
            changed = true;
        }
        if changed {
            write_profile(&self.profile_path, &self.profile)?;
        }
        Ok(changed)
    }

    pub fn observe(
        &mut self,
        user: &str,
        assistant: &str,
        previous: Option<&(String, String)>,
    ) -> io::Result<()> {
        let sensitive = looks_sensitive(user) || looks_sensitive(assistant);
        let signal = feedback_signal(user);
        self.profile.interaction_count = self.profile.interaction_count.saturating_add(1);
        if signal != "observation" {
            self.profile.feedback_count = self.profile.feedback_count.saturating_add(1);
            apply_safe_preference(&mut self.profile, user, signal);
        }

        let prior_user = previous.map(|turn| turn.0.as_str()).unwrap_or("");
        let prior_assistant = previous.map(|turn| turn.1.as_str()).unwrap_or("");
        let event = serde_json::json!({
            "schema": "perci.interaction-learning.v1",
            "recorded_at_unix_ms": now_ms(),
            "signal": signal,
            "user": if sensitive { "[redacted-sensitive]".to_owned() } else { bounded(user) },
            "assistant": if sensitive { "[redacted-sensitive]".to_owned() } else { bounded(assistant) },
            "prior_user": if sensitive { "[redacted-sensitive]".to_owned() } else { bounded(prior_user) },
            "prior_assistant": if sensitive { "[redacted-sensitive]".to_owned() } else { bounded(prior_assistant) },
            "profile_applied": signal != "observation",
            "candidate_status": "pending_review",
            "automatic_fact_promotion": false,
            "automatic_weight_mutation": false,
        });
        append_jsonl(&self.events_path, &event)?;
        write_profile(&self.profile_path, &self.profile)
    }

    pub fn stage_teaching(&mut self, claim: &str) -> io::Result<String> {
        let claim = claim.trim();
        if claim.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "teaching claim is empty",
            ));
        }
        if looks_sensitive(claim) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sensitive-looking content cannot become a teaching candidate",
            ));
        }
        let recorded_at = now_ms();
        let candidate_id = format!("teach-{recorded_at}");
        let event = serde_json::json!({
            "schema": "perci.interaction-learning.v1",
            "recorded_at_unix_ms": recorded_at,
            "signal": "explicit_teaching",
            "candidate_id": candidate_id,
            "claim": bounded(claim),
            "candidate_status": "pending_review",
            "automatic_fact_promotion": false,
            "automatic_weight_mutation": false,
        });
        append_jsonl(&self.events_path, &event)?;
        self.profile.teaching_candidate_count =
            self.profile.teaching_candidate_count.saturating_add(1);
        write_profile(&self.profile_path, &self.profile)?;
        Ok(candidate_id)
    }

    pub fn recent_teaching_claims(&self, limit: usize) -> io::Result<Vec<String>> {
        if limit == 0 || !self.events_path.is_file() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.events_path)?;
        let mut claims = BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(&line).ok())
            .filter(|event| {
                event.get("signal").and_then(|value| value.as_str()) == Some("explicit_teaching")
            })
            .filter_map(|event| {
                event
                    .get("claim")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        claims.reverse();
        claims.dedup();
        claims.truncate(limit);
        Ok(claims)
    }
}

fn feedback_signal(user: &str) -> &'static str {
    let text = user.to_ascii_lowercase();
    if [
        "not smooth",
        "isn't smooth",
        "isnt smooth",
        "doesn't seem smooth",
        "doesnt seem smooth",
        "lacking smoothness",
        "too stiff",
        "too robotic",
        "too procedural",
        "too formal",
        "too verbose",
        "too long",
        "be more concise",
        "say it naturally",
        "less formal",
        "more natural",
        "speak more smart",
        "speak smarter",
        "more smart",
        "talk smarter",
        "sound smarter",
        "less robotic",
        "generic",
        "non direct",
        "not direct",
        "too vague",
        "lead with the direct",
        "lead with direct",
        "same answer",
        "same response",
        "repeating yourself",
        "keep repeating",
        "why do you repeat",
        "repeat sayings",
        "not working correctly",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        "negative_style_feedback"
    } else if [
        "need more",
        "go deeper",
        "one level deeper",
        "more detail",
        "expand on that",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        "request_elaboration"
    } else if [
        "love the style",
        "that was good",
        "perfect",
        "much better",
        "much smoother",
        "chat seems smoother",
        "your system seems smoother",
        "system seems smoother",
        "seems smoother",
        "feels smoother",
        "your system feels smoother",
        "smoother now",
        "that helped",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        "positive_feedback"
    } else if [
        "wrong",
        "incorrect",
        "not what i meant",
        "that's not true",
        "thats not true",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        "correction"
    } else {
        "observation"
    }
}

fn apply_safe_preference(profile: &mut DialogueProfile, user: &str, signal: &str) {
    let text = user.to_ascii_lowercase();
    if signal == "negative_style_feedback" {
        profile.avoid_structured_chat = true;
        profile.prefer_warmth = true;
        if text.contains("generic") || text.contains("direct") {
            profile.prefer_direct_answers = true;
        }
        if text.contains("verbose")
            || text.contains("too long")
            || text.contains("concise")
            || text.contains("smooth")
            || text.contains("smart")
            || text.contains("natural")
            || text.contains("robotic")
        {
            profile.prefer_concise = true;
            profile.prefer_direct_answers = true;
            profile.avoid_structured_chat = true;
        }
    }
    if signal == "positive_feedback" {
        profile.prefer_warmth = true;
    }
    if signal == "request_elaboration" {
        profile.prefer_explanations = true;
        profile.prefer_direct_answers = true;
    }
}

fn append_jsonl(path: &Path, value: &serde_json::Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{value}")
}

fn write_profile(path: &Path, profile: &DialogueProfile) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(profile).map_err(json_error)?,
    )?;
    if path.is_file() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)
}

fn bounded(text: &str) -> String {
    text.chars().take(MAX_FIELD_CHARS).collect()
}

fn looks_sensitive(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "api key",
        "apikey",
        "password",
        "secret=",
        "token=",
        "authorization: bearer",
        "private key",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_feedback_updates_profile_and_writes_pending_record() {
        let root = env::temp_dir().join(format!("perci_learn_{}", now_ms()));
        let events = root.join("events.jsonl");
        let profile = root.join("profile.json");
        let mut learner = InteractionLearner::new(&events, &profile);
        learner
            .observe("that was too procedural and not smooth", "I agree.", None)
            .unwrap();
        assert!(learner.profile().prefer_concise);
        assert!(learner.profile().avoid_structured_chat);
        let raw = fs::read_to_string(events).unwrap();
        assert!(raw.contains("pending_review"));
        assert!(raw.contains("automatic_weight_mutation\":false"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn directness_and_depth_feedback_become_active_preferences() {
        let root = env::temp_dir().join(format!("perci_direct_{}", now_ms()));
        let mut learner =
            InteractionLearner::new(root.join("events.jsonl"), root.join("profile.json"));
        learner
            .observe("that answer was generic and non direct", "Fair.", None)
            .unwrap();
        learner
            .observe("good premise but I need more", "Going deeper.", None)
            .unwrap();
        assert!(learner.profile().prefer_direct_answers);
        assert!(learner.profile().prefer_explanations);
        assert_eq!(learner.profile().feedback_count, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn positive_smoothness_feedback_is_counted_and_persisted() {
        let root = env::temp_dir().join(format!("perci_positive_{}", now_ms()));
        let events = root.join("events.jsonl");
        let profile = root.join("profile.json");
        let mut learner = InteractionLearner::new(&events, &profile);
        learner
            .observe(
                "Your system seems smoother",
                "I'm glad it feels smoother.",
                None,
            )
            .unwrap();
        assert_eq!(learner.profile().feedback_count, 1);
        assert!(learner.profile().prefer_warmth);
        let raw = fs::read_to_string(events).unwrap();
        assert!(raw.contains("positive_feedback"));
        assert!(raw.contains("\"profile_applied\":true"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sensitive_turns_are_redacted() {
        let root = env::temp_dir().join(format!("perci_redact_{}", now_ms()));
        let events = root.join("events.jsonl");
        let mut learner = InteractionLearner::new(&events, root.join("profile.json"));
        learner
            .observe("my api key is abc123", "do not store it", None)
            .unwrap();
        let raw = fs::read_to_string(events).unwrap();
        assert!(!raw.contains("abc123"));
        assert!(raw.contains("redacted-sensitive"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_teaching_is_pending_and_never_auto_promoted() {
        let root = env::temp_dir().join(format!("perci_teach_{}", now_ms()));
        let events = root.join("events.jsonl");
        let mut learner = InteractionLearner::new(&events, root.join("profile.json"));
        let id = learner
            .stage_teaching("A claim needs provenance and a falsifiable check")
            .unwrap();
        assert!(id.starts_with("teach-"));
        assert_eq!(learner.profile().teaching_candidate_count, 1);
        let raw = fs::read_to_string(events).unwrap();
        assert!(raw.contains("explicit_teaching"));
        assert!(raw.contains("pending_review"));
        assert!(raw.contains("automatic_fact_promotion\":false"));
        assert!(raw.contains("automatic_weight_mutation\":false"));
        assert_eq!(
            learner.recent_teaching_claims(2).unwrap(),
            vec!["A claim needs provenance and a falsifiable check"]
        );
        let _ = fs::remove_dir_all(root);
    }
}
