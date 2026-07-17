//! Explicit reasoning loop + dual-pass self-critic for deep domains.
//!
//! Not chain-of-thought theater. Structured, short, human-readable:
//! goal → known/assumed/unknown → steps → attack → answer → next check.

use crate::voice::{detect_social, SocialKind};

/// Domains that benefit from multi-step reasoning.
pub fn needs_reason_loop(label: &str, user: &str) -> bool {
    // Skip pure social / exact one-shot
    if !matches!(
        detect_social(user),
        SocialKind::None | SocialKind::Frustration
    ) {
        // Frustration with tech still reasons
        if detect_social(user) == SocialKind::Frustration {
            return user_looks_technical(user);
        }
        return false;
    }
    let t = user.to_ascii_lowercase();
    if t.split_whitespace().count() < 4 && !user_looks_technical(user) {
        return false;
    }
    let deep_domain = matches!(
        label,
        "logic"
            | "science"
            | "code"
            | "planning"
            | "systems"
            | "governance"
            | "comparison"
            | "explanation"
    );
    let asks_for_depth = [
        "why ",
        "how should",
        "how do",
        "explain",
        "analyze",
        "reason",
        "prove",
        "tradeoff",
        "compare",
        "plan ",
        "design ",
        "fix ",
        "debug",
        "investigate",
        "step by step",
    ]
    .iter()
    .any(|marker| t.contains(marker));
    deep_domain && asks_for_depth
}

fn user_looks_technical(user: &str) -> bool {
    let t = user.to_ascii_lowercase();
    [
        "bug",
        "error",
        "cargo",
        "rust",
        "debug",
        "compile",
        "test",
        "plan",
        "architect",
        "permission",
        "govern",
        "hypothesis",
        "evidence",
        "counterexample",
        "fail",
    ]
    .iter()
    .any(|k| t.contains(k))
}

#[derive(Debug, Clone)]
pub struct ReasonFrame {
    pub goal: String,
    pub known: String,
    pub assumed: String,
    pub unknown: String,
    pub steps: Vec<String>,
    pub attack: String,
    pub next_check: String,
}

/// Build a compact reasoning frame from user text + domain label.
pub fn build_frame(label: &str, user: &str) -> ReasonFrame {
    let goal = compress_goal(user);
    let (known, assumed, unknown) = split_epistemics(label, user);
    let steps = plan_steps(label, user);
    let attack = attack_line(label, user);
    let next_check = next_check_line(label, user);
    ReasonFrame {
        goal,
        known,
        assumed,
        unknown,
        steps,
        attack,
        next_check,
    }
}

/// Render frame + domain body into a natural multi-step answer.
pub fn render_reasoned_answer(
    frame: &ReasonFrame,
    domain_body: &str,
    woven_tips: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("Here's how I'd reason it:\n");
    out.push_str(&format!("• Goal: {}\n", frame.goal));
    out.push_str(&format!("• Known: {}\n", frame.known));
    if !frame.assumed.is_empty() {
        out.push_str(&format!("• Assuming: {}\n", frame.assumed));
    }
    if !frame.unknown.is_empty() {
        out.push_str(&format!("• Unknown: {}\n", frame.unknown));
    }
    out.push_str("• Steps:\n");
    for (i, s) in frame.steps.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, s));
    }
    out.push_str(&format!("• Stress-test: {}\n", frame.attack));
    out.push_str(&format!("• Conclusion: {}\n", domain_body.trim()));
    if !woven_tips.is_empty() {
        out.push_str("• From packs: ");
        out.push_str(&woven_tips.join(" · "));
        if !out.ends_with('.') {
            out.push('.');
        }
        out.push('\n');
    }
    out.push_str(&format!("• Next check: {}", frame.next_check));
    out
}

/// Render exploratory questions as connected prose. The full evidence frame
/// remains available through `/trace`; the visible answer should sound like a
/// person explaining an idea, not like an internal checklist.
pub fn render_natural_reasoned_answer(
    frame: &ReasonFrame,
    domain_body: &str,
    _woven_tips: &[String],
) -> String {
    let first_step = frame
        .steps
        .first()
        .map(|step| lower_initial(step))
        .unwrap_or_else(|| "name the relevant relation".to_owned());
    let second_step = frame
        .steps
        .get(1)
        .map(|step| lower_initial(step))
        .unwrap_or_else(|| "compare it with a nearby alternative".to_owned());
    format!(
        "The short answer is {}. It becomes clearer when we {} and then {}. The boundary is {}. A useful next check is {}.",
        domain_body.trim(),
        first_step.trim_end_matches('.'),
        second_step.trim_end_matches('.'),
        frame.attack.trim_end_matches('.'),
        lower_initial(&frame.next_check).trim_end_matches('.')
    )
}

fn lower_initial(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn conversational_reasoning(user: &str) -> bool {
    let text = user.trim().to_ascii_lowercase();
    let exploratory = text.starts_with("why ")
        || text.starts_with("how ")
        || text.starts_with("what ")
        || text.starts_with("tell me ")
        || text.contains(" in plain language");
    let procedural = [
        "step by step",
        "debug",
        "fix ",
        "plan ",
        "design ",
        "prove ",
        "test ",
        "compare ",
        "analyze ",
        "list ",
        "acceptance",
        "held-out",
        "ablation",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    exploratory && !procedural
}

/// Dual-pass critic: returns Ok(answer) or a revised answer.
pub fn self_critic(user: &str, label: &str, answer: &str) -> (String, Vec<&'static str>) {
    let mut flags: Vec<&'static str> = Vec::new();
    let a = answer.trim();
    let u = user.to_ascii_lowercase();

    if a.is_empty() || a.len() < 12 {
        flags.push("empty_or_thin");
    }
    // Invented tool theater
    let lower = a.to_ascii_lowercase();
    if lower.contains("i ran cargo")
        || lower.contains("i executed")
        || lower.contains("tests passed")
        || lower.contains("i fixed the file")
        || lower.contains("i wrote the patch")
    {
        flags.push("invented_execution");
    }
    // Off-topic comparison when user asked about bugs
    if (u.contains("bug") || u.contains("error") || u.contains("stuck"))
        && lower.contains("crowning a winner")
    {
        flags.push("off_topic");
    }
    // Missing verify when code-shaped
    if matches!(label, "code" | "systems")
        && (u.contains("fix") || u.contains("debug") || u.contains("error") || u.contains("bug"))
        && !(lower.contains("verify")
            || lower.contains("re-")
            || lower.contains("test")
            || lower.contains("check")
            || lower.contains("reproduc"))
    {
        flags.push("missing_verify");
    }
    // Doesn't engage the question keywords at all
    let keywords: Vec<&str> = u
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .take(6)
        .collect();
    let hit = keywords.iter().filter(|k| lower.contains(*k)).count();
    if keywords.len() >= 3 && hit == 0 && !matches!(label, "greeting" | "math" | "geometry") {
        flags.push("no_keyword_overlap");
    }

    if flags.is_empty() {
        return (a.to_string(), flags);
    }

    // One revise pass — honest, tighter
    let mut revised = String::new();
    if flags.contains(&"invented_execution") {
        revised.push_str("I haven't run tools yet — this is a plan, not a completed action. ");
    }
    if flags.contains(&"off_topic") || flags.contains(&"no_keyword_overlap") {
        revised.push_str("Back to your question: ");
        revised.push_str(&compress_goal(user));
        revised.push_str(". ");
    }
    if flags.contains(&"missing_verify") {
        revised.push_str("Reproduce the failure, change one thing, then re-run the same check. ");
    }
    if flags.contains(&"empty_or_thin") {
        revised.push_str("I need a clearer goal and one piece of evidence (error text or file). ");
    }
    // Keep useful core of original if not toxic
    if !flags.contains(&"off_topic") {
        let core = first_useful_sentence(a);
        if !core.is_empty() && !revised.contains(&core) {
            revised.push_str(&core);
            if !revised.ends_with('.') {
                revised.push('.');
            }
        }
    } else {
        revised.push_str(domain_fallback_fix(label));
    }
    (revised.trim().to_string(), flags)
}

/// Score reasoning quality 0.0–1.0 for eval harnesses.
pub fn score_reasoning(user: &str, answer: &str, label: &str) -> f64 {
    let a = answer.to_ascii_lowercase();
    let u = user.to_ascii_lowercase();
    let mut s = 0.25;

    // Structure markers from reason loop
    if a.contains("goal:") || a.contains("here's how i'd reason") {
        s += 0.15;
    }
    if a.contains("steps:") || a.contains("1.") {
        s += 0.1;
    }
    if a.contains("stress-test") || a.contains("counterexample") || a.contains("attack") {
        s += 0.1;
    }
    if a.contains("next check") || a.contains("verify") || a.contains("re-run") {
        s += 0.1;
    }
    if a.contains("known:") || a.contains("assuming:") || a.contains("unknown:") {
        s += 0.1;
    }
    // Honesty
    if a.contains("haven't run") || a.contains("i don't know") || a.contains("uncertain") {
        s += 0.05;
    }
    // No invented execution
    if a.contains("i ran cargo") || a.contains("tests passed") {
        s -= 0.2;
    }
    // Keyword engagement
    let keys: Vec<&str> = u
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .take(5)
        .collect();
    if !keys.is_empty() {
        let hits = keys.iter().filter(|k| a.contains(*k)).count();
        s += 0.1 * (hits as f64 / keys.len() as f64);
    }
    // Domain-appropriate
    if matches!(label, "code")
        && (a.contains("reproduc") || a.contains("error") || a.contains("patch"))
    {
        s += 0.1;
    }
    if matches!(label, "logic" | "science")
        && (a.contains("evidence") || a.contains("assum") || a.contains("falsif"))
    {
        s += 0.1;
    }
    s.clamp(0.0, 1.0)
}

fn compress_goal(user: &str) -> String {
    let t = user.trim().replace('\n', " ");
    if t.chars().count() <= 120 {
        t
    } else {
        t.chars().take(117).collect::<String>() + "…"
    }
}

fn split_epistemics(label: &str, user: &str) -> (String, String, String) {
    let t = user.to_ascii_lowercase();
    let known = if t.contains("error") || t.contains("fail") {
        "a failure symptom was reported".into()
    } else if t.contains("calcul") || t.contains("percent") {
        "numeric operands may be exact-tool eligible".into()
    } else {
        format!("the user asked about {label}")
    };
    let assumed = match label {
        "code" => "toolchain and repo are available under permission".into(),
        "governance" => "human remains authority for durable mutation".into(),
        "science" => "we can define a measurable proxy for the claim".into(),
        _ => "terms mean what they usually mean in this stack".into(),
    };
    let unknown = if t.contains("error") && !t.contains("error[") && !t.contains("failed") {
        "exact error text / exit code".into()
    } else if matches!(label, "code" | "systems") {
        "controlling file path and verify command".into()
    } else if matches!(label, "planning") {
        "deadline, constraints, and success metric".into()
    } else {
        "missing evidence that would change the answer".into()
    };
    (known, assumed, unknown)
}

fn plan_steps(label: &str, user: &str) -> Vec<String> {
    let t = user.to_ascii_lowercase();
    match label {
        "code" => vec![
            "Reproduce with the smallest command or input".into(),
            "Read the first hard error (not the last warning)".into(),
            "Patch one surface; keep the change surgical".into(),
            "Re-run the same verify command".into(),
        ],
        "logic" => vec![
            "List premises in plain language".into(),
            "Mark which are assumptions vs evidence".into(),
            "Derive only supported conclusions".into(),
            "Search for a counterexample".into(),
        ],
        "science" => vec![
            "Define the measurable claim".into(),
            "Name a falsifiable prediction".into(),
            "Compare observation to prediction with uncertainty".into(),
        ],
        "planning" => vec![
            "State objective and constraints".into(),
            "Cut the smallest end-to-end slice".into(),
            "Define acceptance test per milestone".into(),
        ],
        "governance" => vec![
            "Identify actor and required permission".into(),
            "Bound scope and rollback".into(),
            "Require validation before durable write".into(),
        ],
        "systems" => vec![
            "Name each component's job and authority limit".into(),
            "Separate suggest vs mutate paths".into(),
            "Check failure modes at boundaries".into(),
        ],
        "comparison" => vec![
            "Fix the workload and cost of being wrong".into(),
            "Score options on the same criteria".into(),
            "Pick or defer with an explicit residual risk".into(),
        ],
        _ => {
            if t.contains("why") {
                vec![
                    "Restate the phenomenon".into(),
                    "Offer the best mechanism".into(),
                    "Note what would disprove it".into(),
                ]
            } else {
                vec![
                    "Clarify the desired outcome".into(),
                    "List evidence already in hand".into(),
                    "Choose the smallest next test".into(),
                ]
            }
        }
    }
}

fn attack_line(label: &str, user: &str) -> String {
    let t = user.to_ascii_lowercase();
    match label {
        "code" => "What if the error is environmental (path, version) not logic?".into(),
        "logic" => {
            "Is there a counterexample that keeps the premises but kills the conclusion?".into()
        }
        "science" => "Could measurement noise or a confound explain the same data?".into(),
        "governance" => {
            "What if permission was granted but validation failed — still block?".into()
        }
        "planning" => "What if the first milestone cannot be verified — stop expanding.".into(),
        _ if t.contains("always") || t.contains("never") => {
            "Absolute claims usually fail on edge cases — name one.".into()
        }
        _ => "What evidence would force us to change this answer?".into(),
    }
}

fn next_check_line(label: &str, _user: &str) -> String {
    match label {
        "code" => "Run the failing command again after one change.".into(),
        "math" | "geometry" => "Recompute with exact tools if numbers are present.".into(),
        "governance" => "Confirm permission scope and a snapshot before mutate.".into(),
        "science" => "State one measurement you could take next.".into(),
        "planning" => "Define the acceptance test for step 1.".into(),
        _ => "Name one fact that would update the conclusion.".into(),
    }
}

fn first_useful_sentence(a: &str) -> String {
    a.lines()
        .map(str::trim)
        .find(|l| l.len() > 20 && !l.starts_with('•') && !l.starts_with('-'))
        .unwrap_or("")
        .chars()
        .take(200)
        .collect()
}

fn domain_fallback_fix(label: &str) -> &'static str {
    match label {
        "code" => "Reproduce, isolate the smallest fail, read the exact error, patch once, verify.",
        "logic" => "Separate premises from conclusions and try to break the claim.",
        "science" => "Define a measurable prediction, then compare against data with uncertainty.",
        _ => "State the goal, list evidence, pick the smallest next test.",
    }
}

/// Apply reason loop + critic to a domain body for deep answers.
pub fn enhance_deep_answer(
    label: &str,
    user: &str,
    domain_body: &str,
    woven_tips: &[String],
) -> (String, f64, Vec<&'static str>) {
    if !needs_reason_loop(label, user) {
        let (ans, flags) = self_critic(user, label, domain_body);
        let score = score_reasoning(user, &ans, label);
        return (ans, score, flags);
    }
    let frame = build_frame(label, user);
    let draft = if conversational_reasoning(user) {
        render_natural_reasoned_answer(&frame, domain_body, woven_tips)
    } else {
        render_reasoned_answer(&frame, domain_body, woven_tips)
    };
    let (ans, flags) = self_critic(user, label, &draft);
    // If critic gutted structure, re-wrap once
    let final_ans = if flags.contains(&"off_topic") && needs_reason_loop(label, user) {
        let frame = build_frame(label, user);
        if conversational_reasoning(user) {
            render_natural_reasoned_answer(&frame, domain_fallback_fix(label), woven_tips)
        } else {
            render_reasoned_answer(&frame, domain_fallback_fix(label), woven_tips)
        }
    } else {
        ans
    };
    let score = score_reasoning(user, &final_ans, label);
    (final_ans, score, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_code_needs_loop() {
        assert!(needs_reason_loop(
            "code",
            "fix cargo compile error in forge"
        ));
        assert!(!needs_reason_loop("greeting", "hello perci"));
        assert!(!needs_reason_loop(
            "general",
            "doesnt seem smooth enough, agree?"
        ));
        assert!(needs_reason_loop(
            "planning",
            "plan the migration step by step"
        ));
    }

    #[test]
    fn critic_flags_invented_execution() {
        let (_, flags) = self_critic(
            "bug",
            "code",
            "I ran cargo test and tests passed perfectly.",
        );
        assert!(flags.contains(&"invented_execution"));
    }

    #[test]
    fn reason_score_prefers_structure() {
        let weak = score_reasoning("why", "maybe stuff", "logic");
        let strong = score_reasoning(
            "why does permission matter",
            "Here's how I'd reason it:\n• Goal: permission\n• Steps:\n  1. check\n• Stress-test: counterexample\n• Next check: verify",
            "governance",
        );
        assert!(strong > weak);
    }

    #[test]
    fn exploratory_questions_use_connected_prose() {
        let (answer, _, _) = enhance_deep_answer(
            "science",
            "Why does life maintain local order?",
            "Life maintains local organization by consuming energy and exporting entropy.",
            &[],
        );
        assert!(answer.starts_with("The short answer is"));
        assert!(!answer.contains("Here's how I'd reason it:"));
        assert!(answer.contains("boundary"));
        assert!(answer.contains("next check"));
    }
}
