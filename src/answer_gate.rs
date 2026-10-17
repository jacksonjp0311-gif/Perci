//! Final answer gate for chat turns.
//!
//! Perci's open path assembles prose from routing templates, Bitwork concept
//! sentences and the native n-gram fields. None of those sources hold world
//! knowledge, so for a factual, how-to, code, creative, planning or
//! troubleshooting request the draft is ungrounded, however fluent it sounds.
//! The same is true of the critic's meta repairs ("That repeats the last
//! answer…", "I'm tracking X. The last answer missed your point…"), of stock
//! filler sentences, and of leaked internal scaffolding
//! (`[Perci dialogue workspace: …]`, B2E-PERCI-008).
//!
//! The gate replaces those drafts with something the user can act on:
//! * a matching saved note when there is one (quoted, marked unverified);
//! * a general procedure where one genuinely helps (troubleshooting order,
//!   migration / design / project plans), framed as general, not as knowledge
//!   of the user's system;
//! * an accurate description of Perci itself for questions about Perci;
//! * otherwise a short, specific statement of what Perci can't do for this
//!   request and which of its real tools do work.
//!
//! It never invents facts. Exact-tool answers pass through untouched apart
//! from hint stripping.

use crate::util::text::{normalize_quotes, truncate};

/// What kind of request the user made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestKind {
    /// "what is / who was / where is / define …"
    Factual,
    /// "how do I / how to / steps to …"
    HowTo,
    /// Asks for code or a code-level explanation.
    Code,
    /// "write me a poem / story / email …"
    Creative,
    /// "plan / design / migrate / structure …"
    Plan,
    /// Something of the user's is failing ("not responding", "error", …).
    Troubleshoot,
    /// Opinion, reflection, conversation, or anything else.
    Open,
}

/// Where a draft came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftSource {
    /// Exact arithmetic / geometry / proof / memory / teaching / security
    /// boundary. Authoritative; only hint stripping applies.
    Exact,
    /// A named, hand-written operator or reviewed dialogue case.
    Operator,
    /// The associative open path (Bitwork + native language fields).
    Open,
    /// A configured local language model produced the text.
    Model,
}

/// A saved note that matched the request.
#[derive(Clone, Debug)]
pub struct NoteHit {
    pub text: String,
}

/// Everything [`review`] needs about one turn.
#[derive(Clone, Copy, Debug)]
pub struct Turn<'a> {
    pub input: &'a str,
    pub draft: &'a str,
    pub source: DraftSource,
    /// The previous (user, answer) turn, if any.
    pub previous: Option<&'a (String, String)>,
    pub notes: &'a [NoteHit],
    /// A local model is configured (changes the "how to get more" hint).
    pub model_configured: bool,
}

/// Whole-answer deflections: text that talks about the conversation instead
/// of answering it. Matched on lower-cased, ASCII-quoted text.
const META_DEFLECTIONS: &[&str] = &[
    "repeats the last answer",
    "did not produce a usable draft",
    "no usable draft for this turn",
    "lost the thread while carrying",
    "rephrase the request and i will",
    "the last answer missed your point",
    "i can take it toward mechanism, evidence, or the next test",
    "[perci ",
    "[recent dialogue]",
];

/// Stock filler sentences the open path splices into drafts. A sentence
/// containing one of these carries no answer content.
const FILLER: &[&str] = &[
    "i'll stay concrete",
    "the last answer missed your point",
    "i'm tracking ",
    "what's the claim we can check directly",
    "that keeps the answer anchored to",
    "carrying that thread forward",
    "back to your question",
    "for your point about",
    "the short answer is name the thing",
    "i can take it toward mechanism",
    "anchor: ",
];

/// Words that mark a question about Perci itself; Perci's own operators and
/// self-description answer these.
const PERCI_TERMS: &[&str] = &[
    "perci",
    "bitwork",
    "softcascade",
    "popcount",
    "prototype pack",
    "cognitive pack",
    "your weights",
    "your memory",
];

fn has(lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| lower.contains(n))
}

fn starts(lower: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| lower.starts_with(p))
}

fn lowered(text: &str) -> String {
    normalize_quotes(text).to_ascii_lowercase()
}

/// True when the request is about Perci itself.
pub fn is_about_perci(input: &str) -> bool {
    has(&lowered(input), PERCI_TERMS)
}

/// Requests addressed to Perci, or that invite its opinion, are answered by
/// Perci's own operators/associative voice and are not gated as "unknown".
fn is_about_perci_or_opinion(lower: &str) -> bool {
    let padded = format!(" {lower} ");
    has(&padded, PERCI_TERMS)
        || has(
            &padded,
            &[
                " you ",
                " you?",
                " your ",
                " yourself",
                " you'",
                "do you think",
                "in your view",
                "original thought",
                "imagine ",
                "what if ",
                " we ",
                " us ",
            ],
        )
}

/// Second-person complaint that the last answer missed the question
/// (B2E-PERCI-003). Deliberately anchored to "you"/"that" so "why is my
/// server not answering requests" is *not* feedback.
pub fn is_feedback_complaint(input: &str) -> bool {
    let lower = lowered(input);
    let lower = lower.trim();
    has(
        lower,
        &[
            "you're not answering",
            "you are not answering",
            "youre not answering",
            "you aren't answering",
            "you didn't answer",
            "you did not answer",
            "you didnt answer",
            "you're not listening",
            "you aren't listening",
            "you are not listening",
            "you missed my question",
            "you missed the question",
            "that's not what i asked",
            "thats not what i asked",
            "that is not what i asked",
            "that doesn't answer",
            "that does not answer",
            "that didn't answer",
            "that did not answer",
            "answer my question",
            "answer the question",
        ],
    )
}

/// Classify a (typo-repaired) request.
pub fn classify(input: &str) -> RequestKind {
    let lower = lowered(input);
    let lower = lower.trim().trim_end_matches(['?', '!', '.']).trim();
    if lower.is_empty() {
        return RequestKind::Open;
    }
    let padded = format!(" {lower} ");
    let troubled = has(
        lower,
        &[
            "not working",
            "not answering",
            "not responding",
            "stopped responding",
            "stopped working",
            "no response",
            "won't start",
            "wont start",
            "doesn't work",
            "does not work",
            "doesnt work",
            "keeps crashing",
            "crashes",
            "crashing",
            "error",
            "exception",
            "failing",
            "fails",
            "failed",
            "broken",
            "timing out",
            "times out",
            "timeout",
            "refused",
            "refusing",
            "rejecting",
            "won't connect",
            "wont connect",
            "keeps disconnecting",
            "keeps dropping",
            "not loading",
            "can't connect",
            "cannot connect",
            "cant connect",
            "won't load",
            "wont load",
            "hangs",
            "freezes",
            "runs all the time",
            "run all the time",
            "running all the time",
            "overheating",
            "too slow",
            "so slow",
        ],
    );
    // Feedback aimed at Perci ("you're not answering") is a dialogue act,
    // not a support request.
    if troubled
        && has(
            &padded,
            &[" my ", " our ", " the ", " this ", " it ", " its "],
        )
        && !has(&padded, &[" you ", " you're ", " youre ", " your "])
    {
        return RequestKind::Troubleshoot;
    }
    if is_about_perci_or_opinion(lower) {
        return RequestKind::Open;
    }
    let creative_forms = [
        "haiku",
        "poem",
        "story",
        "song",
        "lyrics",
        "essay",
        "letter",
        "email",
        "joke",
        "limerick",
        "speech",
        "toast",
        "cover letter",
        "tweet",
        "slogan",
    ];
    if (starts(
        lower,
        &[
            "write",
            "compose",
            "draft",
            "make up",
            "tell me a",
            "give me a",
        ],
    ) || lower.contains("can you write"))
        && has(lower, &creative_forms)
    {
        return RequestKind::Creative;
    }
    if starts(
        lower,
        &[
            "plan ",
            "help me plan",
            "design ",
            "architect ",
            "how should i structure",
            "how do i structure",
            "how should i organize",
            "how should i organise",
            "how do i migrate",
            "how should i migrate",
            "migrate ",
            "make a plan",
            "create a plan",
            "outline a plan",
            "draft a plan",
        ],
    ) || has(
        lower,
        &[
            "migration plan",
            "plan a migration",
            "roadmap for",
            "schema for",
            "data model for",
        ],
    ) {
        return RequestKind::Plan;
    }
    let code_words = [
        "rust",
        "python",
        "javascript",
        "typescript",
        "java ",
        "c++",
        "c#",
        "golang",
        " go ",
        "sql",
        "regex",
        "function",
        " code",
        "script",
        "algorithm",
        "linked list",
        "binary tree",
        "hash map",
        "hashmap",
        "hash table",
        "compiler",
        "borrow checker",
        "lifetimes",
        " api",
        "class ",
        "struct",
        "powershell",
        "bash",
        " css",
        " html",
        "mutex",
        "async",
        "pointer",
        "recursion",
    ];
    if has(&padded, &code_words)
        && (starts(
            lower,
            &[
                "how",
                "write",
                "show",
                "implement",
                "give me",
                "example",
                "explain",
                "what is the best way",
            ],
        ) || has(lower, &["how do i", "how to", "example of", "implement"]))
    {
        return RequestKind::Code;
    }
    if starts(
        lower,
        &[
            "how do i ",
            "how can i ",
            "how to ",
            "how should i ",
            "how would i ",
            "what's the best way to ",
            "what is the best way to ",
            "steps to ",
            "what are the steps",
            "teach me how",
            "show me how",
        ],
    ) {
        return RequestKind::HowTo;
    }
    if starts(
        lower,
        &[
            "what is ",
            "what's ",
            "what are ",
            "what was ",
            "what were ",
            "what does ",
            "what causes ",
            "what should i ",
            "which ",
            "who ",
            "whom ",
            "whose ",
            "where ",
            "when ",
            "how many ",
            "how much ",
            "how far ",
            "how old ",
            "how long ",
            "how tall ",
            "how big ",
            "how does ",
            "how do ",
            "why do ",
            "why does ",
            "why is ",
            "why are ",
            "define ",
            "explain ",
            "describe ",
            "tell me about ",
            "list ",
            "name ",
            "translate ",
            "summarize ",
            "summarise ",
            "compare ",
            "recommend ",
            "suggest ",
            "give me a recipe",
            "give me some ",
            "is it worth ",
        ],
    ) || has(
        lower,
        &[
            "capital of",
            "population of",
            "meaning of",
            "definition of",
            "difference between",
            "recipe for",
        ],
    ) {
        return RequestKind::Factual;
    }
    RequestKind::Open
}

/// True for critic/meta text that talks about the conversation instead of
/// answering it, and for leaked internal scaffolding.
pub fn is_meta_deflection(answer: &str) -> bool {
    has(&lowered(answer), META_DEFLECTIONS)
}

/// Drop internal hint fragments such as `[Perci dialogue workspace: …]` from
/// user-facing text (B2E-PERCI-008).
pub fn strip_internal_hints(text: &str) -> String {
    if !text.contains("[Perci ") && !text.contains("[Recent dialogue]") {
        return text.trim().to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = [rest.find("[Perci "), rest.find("[Recent dialogue]")]
        .into_iter()
        .flatten()
        .min()
    {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        match tail.find(']') {
            // Cut at the first closing bracket on the same line.
            Some(end) if !tail[..end].contains('\n') => rest = &tail[end + 1..],
            // A truncated hint (no closing bracket) is dropped to the end of
            // its sentence, including a trailing ellipsis.
            _ => {
                rest = tail
                    .find(['.', '\n'])
                    .map(|i| &tail[i..])
                    .unwrap_or("")
                    .trim_start_matches(['…', '.']);
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" .", ".")
        .replace("Practical angle:.", "")
        .replace("Practical angle: .", "")
        .trim()
        .to_owned()
}

fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if matches!(b, b'.' | b'?' | b'!') && bytes.get(i + 1).map_or(true, |n| *n == b' ') {
            let s = text[start..=i].trim();
            if !s.is_empty() {
                out.push(s);
            }
            start = i + 1;
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Remove stock filler sentences; what remains is the draft's real content.
pub fn strip_filler(text: &str) -> String {
    sentences(text)
        .into_iter()
        .filter(|s| !has(&lowered(s), FILLER))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The question the user is actually asking: for a complaint about the last
/// answer, the previous question; otherwise the input itself.
pub fn effective_question<'a>(input: &'a str, previous: Option<&'a (String, String)>) -> &'a str {
    match previous {
        Some((prev_user, _))
            if is_feedback_complaint(input) && !is_feedback_complaint(prev_user) =>
        {
            prev_user.as_str()
        }
        _ => input,
    }
}

const STOP: &[&str] = &[
    "the", "and", "for", "are", "was", "were", "what", "where", "when", "who", "why", "how",
    "does", "did", "that", "this", "with", "from", "into", "about", "your", "you", "have", "has",
    "can", "could", "should", "would", "will", "is", "my", "our", "its", "it's", "there", "their",
    "them", "they", "tell", "me", "please", "explain", "give", "some", "any", "work", "works",
    "make", "get", "use", "need", "want", "know", "think", "like", "just", "all", "one",
];

fn content_words(text: &str) -> Vec<String> {
    lowered(text)
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\''))
        .filter(|w| w.len() >= 3 && !STOP.contains(w))
        .map(str::to_owned)
        .collect()
}

/// Keep only notes that share most of the request's content words, so a note
/// that merely contains "work" is not quoted for "how does a hash map work".
pub fn relevant_notes(query: &str, candidates: &[String]) -> Vec<NoteHit> {
    let terms = content_words(query);
    if terms.is_empty() {
        return Vec::new();
    }
    let need = terms.len().div_ceil(2).max(1);
    candidates
        .iter()
        .filter_map(|c| {
            // Context items may carry a "[source] " prefix.
            let text = c
                .strip_prefix('[')
                .and_then(|r| r.split_once("] ").map(|(_, b)| b))
                .unwrap_or(c)
                .trim();
            let lower = text.to_ascii_lowercase();
            let hits = terms.iter().filter(|t| lower.contains(t.as_str())).count();
            (hits >= need && !text.is_empty()).then(|| NoteHit {
                text: text.to_owned(),
            })
        })
        .take(3)
        .collect()
}

/// The request, trimmed for quoting back.
fn subject(input: &str) -> String {
    truncate(input.trim().trim_end_matches(['?', '!', '.']), 90)
}

const CAPABILITIES: &str = "What I can do offline: exact math (arithmetic, fractions, \
percentages, geometry), keep and recall notes (\"remember that …\"), and reason over premises you \
give me (\"if A raises B and B lowers C, what happens to C?\").";

fn model_note(model_configured: bool) -> &'static str {
    if model_configured {
        "Your local model didn't return an answer that passed Perci's checks this time; `perci doctor` shows its status."
    } else {
        "Open questions like this need a local model (Ollama or LM Studio); `perci doctor` shows how to connect one."
    }
}

fn notes_block(notes: &[NoteHit]) -> Option<String> {
    if notes.is_empty() {
        return None;
    }
    let quoted = notes
        .iter()
        .take(3)
        .map(|n| format!("\u{201c}{}\u{201d}", truncate(&n.text, 220)))
        .collect::<Vec<_>>()
        .join("; ");
    Some(format!(
        "From your saved notes: {quoted}. That's what you told me; I haven't verified it."
    ))
}

fn troubleshoot_reply(input: &str) -> String {
    let lower = lowered(input);
    let padded = format!(" {lower} ");
    let service = has(
        &padded,
        &[
            "server",
            "service",
            " api",
            " port",
            "request",
            "connect",
            "endpoint",
            "website",
            " site",
            "daemon",
            " host",
            "network",
            "socket",
            "http",
            " dns",
            " ssh",
            "database",
            " db ",
            "nginx",
            "apache",
            "container",
            "docker",
        ],
    );
    let build = has(
        &padded,
        &[
            " build",
            "compile",
            "compiler",
            "cargo",
            " npm",
            " pip ",
            "install",
            "linker",
            "dependency",
            "dependencies",
            " update",
            "updated",
            "upgrade",
        ],
    );
    let device = has(
        &padded,
        &[
            "laptop", "computer", " pc ", " fan", "battery", "overheat", "printer", "phone",
            "wifi", "wi-fi", "monitor", "keyboard",
        ],
    );
    let what = subject(input);
    if service {
        format!(
            "I can't see your machine, so I can't say why \"{what}\" is happening, but this order usually finds it fast:\n\
1. Is the process actually running? Check the service manager or task list, and whether it restarted or ran out of memory.\n\
2. Is it listening where you think? `ss -ltnp` (Linux) or `netstat -ano | findstr :<port>` (Windows).\n\
3. Does a request from the same machine work? `curl -v http://127.0.0.1:<port>/`. If it does, the problem is between client and server (firewall, proxy, DNS, TLS, wrong host or port). If it doesn't, it's the app.\n\
4. Read the server log at the first failure, not the last line.\n\
5. What changed last: a deploy, config, certificate, disk space, or a dependency?\n\
Paste the error or the log line and we can narrow it down from there."
        )
    } else if build {
        format!(
            "I can't see your build, so I won't guess at the cause of \"{what}\", but this sequence usually isolates it:\n\
1. Re-run with verbose output (for example `cargo build -v`, `npm install --verbose`) and find the first error, not the last.\n\
2. Record the exact toolchain and dependency versions now versus before the update (`rustc -V`, `rustup show`, the lock file diff).\n\
3. Do a clean build (`cargo clean`, or delete the build/output folder) to rule out stale artifacts.\n\
4. If the first error is from the linker or a native dependency, check that the platform build tools and SDK the toolchain expects are installed.\n\
5. Pin back to the previous version to confirm the update is the trigger, then move forward one step at a time.\n\
Paste the first error message and I'll help you read it."
        )
    } else if device {
        format!(
            "I can't inspect your hardware, so I can't tell you why \"{what}\" is happening, but a quick order to check:\n\
1. Look at what's using the resource right now (Task Manager on Windows, Activity Monitor on macOS, `top` on Linux) and note the top process.\n\
2. Check whether it happens at idle, or only under a specific app or task.\n\
3. Check for recent changes: updates, new startup apps, drivers, or a changed power plan.\n\
4. Rule out the physical side: dust, blocked vents, a failing part, or a soft surface under the device.\n\
Tell me what the top process is or when it started, and we can narrow it down."
        )
    } else {
        format!(
            "I can't diagnose \"{what}\" without the details, and I won't guess. What usually gets to the cause fastest:\n\
1. Reproduce it with the smallest input or command you can.\n\
2. Copy the first error message exactly (the first one, not the last warning).\n\
3. Check what changed since it last worked.\n\
4. Change one thing at a time and re-run.\n\
Paste the exact error text and what you expected, and we can work from that."
        )
    }
}

/// Text after " to " / " into " / " onto " (the migration target), if any.
fn target_after_to(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    for marker in [" to ", " into ", " onto "] {
        if let Some(i) = lower.rfind(marker) {
            let t = input[i + marker.len()..]
                .trim()
                .trim_end_matches(['?', '!', '.']);
            if !t.is_empty() && t.split_whitespace().count() <= 4 {
                return Some(t.to_owned());
            }
        }
    }
    None
}

fn plan_reply(input: &str) -> String {
    let lower = lowered(input);
    let what = subject(input);
    let footer = "This is a general procedure, not knowledge of your system. Tell me the sizes, \
deadlines and constraints and I'll keep them in this session, check any numbers exactly, and \
save decisions you want kept (\"remember that …\").";
    if has(
        &lower,
        &["migrat", "move from", "switch to", "port to", "moving to"],
    ) {
        let target = target_after_to(input)
            .map(|t| format!(" to {t}"))
            .unwrap_or_default();
        format!(
            "A migration{target} plan that keeps every step reversible:\n\
1. Inventory: what moves (schemas, data volume, queries, jobs, integrations) and who depends on it.\n\
2. Pick the strategy: one cutover in a maintenance window if the data is small and downtime is acceptable; otherwise run old and new side by side (dual writes or change capture) and backfill.\n\
3. Build the target and a repeatable migration script, and run it against a copy of production first.\n\
4. Verify with numbers: row counts and checksums per table, and your most important queries returning the same results on both sides.\n\
5. Cut over behind a switch (a config flag or connection string) and keep the old system read-only so rollback is one change.\n\
6. Watch errors and latency, then decommission the old system after an agreed quiet period.\n\
{footer}"
        )
    } else if has(
        &lower,
        &["schema", "data model", "database design", "tables"],
    ) {
        format!(
            "A way to design the schema for \"{what}\" that holds up:\n\
1. List the nouns (entities) and, for each, the fields you actually need to show or filter on.\n\
2. Give every table a stable primary key; decide which relations are one-to-many and which need a join table.\n\
3. Add constraints early: NOT NULL, UNIQUE, foreign keys, and allowed values. They catch bugs for free.\n\
4. Write the three or four queries the app runs most, and add indexes for those, not before.\n\
5. Plan for change: timestamps (created/updated), soft deletes if you need history, and migrations from day one.\n\
{footer}"
        )
    } else if has(
        &lower,
        &["structure", "organize", "organise", "layout", "project"],
    ) {
        format!(
            "A structure for \"{what}\" that stays easy to change:\n\
1. Keep a thin entry point that only parses input and calls into the core.\n\
2. Put the core logic in modules with one responsibility each and no I/O, so it's easy to test.\n\
3. Keep I/O (files, network, terminal) at the edges, behind small interfaces.\n\
4. Put tests next to the code they cover, plus a few end-to-end tests that run the real binary.\n\
5. Add formatting, linting and tests to CI from the first commit.\n\
{footer}"
        )
    } else {
        format!(
            "A plan skeleton for \"{what}\":\n\
1. Goal and \"done when\": one sentence each, with a number you can check.\n\
2. Constraints: deadline, budget, what must not break.\n\
3. Risks first: do the step most likely to fail or change the plan earliest.\n\
4. Milestones that each leave something usable and can be rolled back.\n\
5. A check after every milestone, and a rollback path written down before you start.\n\
{footer}"
        )
    }
}

/// Accurate self-description for questions about how Perci works.
fn perci_self_reply(input: &str) -> Option<String> {
    let lower = lowered(input);
    if lower.contains("bitwork") || lower.contains("popcount") {
        return Some(
            "Bitwork is Perci's classifier. Each prototype in the cognitive pack is a 4,096-bit pattern. \
Your message is encoded into the same kind of pattern and compared with every prototype using bitwise AND \
plus a popcount (counting shared 1-bits), all in integer math. The best matches pick one of 16 domains and \
the concept cards that shape the wording. It routes and ranks; it doesn't know facts, and numbers always \
come from the exact tools."
                .to_owned(),
        );
    }
    let asks_how = has(
        &lower,
        &[
            "how does perci",
            "how do you decide",
            "how do you work",
            "how does it decide",
            "how do you answer",
            "how do you choose",
            "how are you built",
            "what are you made of",
            "how does perci work",
        ],
    );
    if asks_how {
        return Some(
            "Each message goes through a fixed order. Exact tools come first: arithmetic, geometry and proofs \
give checked answers. Then memory commands (\"remember that …\", recall). Then named operators, which are \
hand-written reasoning handlers for specific kinds of request. Then reviewed dialogue cases. Last is the open \
path, where the Bitwork classifier picks a domain and concept cards and the language fields word the reply. \
A final answer gate checks the draft: if it's filler, off-topic, or a knowledge question the offline packs \
can't ground, it's replaced with an honest answer, a saved note, or a general procedure. The same input \
gives the same output, and `/trace` shows which path answered."
                .to_owned(),
        );
    }
    None
}

fn open_reply(input: &str) -> String {
    let lower = lowered(input);
    if starts(
        lower.trim(),
        &[
            "i feel",
            "i'm feeling",
            "im feeling",
            "i am feeling",
            "i'm stuck",
            "i am stuck",
            "i'm so",
            "i'm tired",
        ],
    ) {
        return "That sounds frustrating. If you tell me what exactly is stuck (the step you're on, the error, or the \
decision you can't make), I can help break it into the next concrete step, keep track of it in this session, and \
save anything you want kept with \"remember that …\"."
            .to_owned();
    }
    format!(
        "I don't have a grounded answer to \"{}\", and I'd rather say so than pad it out. If you add what you're \
trying to decide or do, I can work through it with you.\n\n{CAPABILITIES}",
        subject(input)
    )
}

/// Build the honest reply for a request Perci can't ground.
pub fn honest_reply(
    kind: RequestKind,
    input: &str,
    notes: &[NoteHit],
    model_configured: bool,
) -> String {
    if let Some(block) = notes_block(notes) {
        // A saved note is the only grounded content Perci has for open
        // questions; lead with it.
        return block;
    }
    if let Some(text) = perci_self_reply(input) {
        return text;
    }
    let what = subject(input);
    let model = model_note(model_configured);
    match kind {
        RequestKind::Troubleshoot => troubleshoot_reply(input),
        RequestKind::Plan => plan_reply(input),
        RequestKind::Factual => format!(
            "I can't answer \"{what}\" reliably offline. Perci has no general knowledge base, only exact \
tools, your notes and a routing classifier, and I'd rather say that than make something up.\n\n{CAPABILITIES} {model}"
        ),
        RequestKind::HowTo => format!(
            "I don't have reliable step-by-step knowledge for \"{what}\", and a made-up procedure would be \
worse than none. If you tell me what you've tried and where it stops, I can keep track of it with you and \
check any numbers exactly.\n\n{CAPABILITIES} {model}"
        ),
        RequestKind::Code => format!(
            "I can't explain or write code for \"{what}\" reliably yet: Perci's offline packs have no code \
model, and something that looks like code without being correct would waste your time. If you paste an error \
or a failing command, I can help you work through it step by step.\n\n{CAPABILITIES} {model}"
        ),
        RequestKind::Creative => format!(
            "I can't write that well yet: my offline language fields produce fragments, not finished writing, \
and I'd rather not hand you filler.\n\n{CAPABILITIES} {model}"
        ),
        RequestKind::Open => open_reply(input),
    }
}

/// Outcome of [`review`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Keep the draft (possibly with internal hints and filler stripped).
    Keep(String),
    /// Replace the draft; `reason` goes into the audit trace.
    Replace { text: String, reason: &'static str },
}

impl Verdict {
    pub fn text(&self) -> &str {
        match self {
            Verdict::Keep(t) => t,
            Verdict::Replace { text, .. } => text,
        }
    }
}

const MIN_SUBSTANTIVE_WORDS: usize = 8;

/// Review one draft.
pub fn review(turn: Turn<'_>) -> Verdict {
    let Turn {
        input,
        draft,
        source,
        previous,
        notes,
        model_configured,
    } = turn;
    let cleaned = strip_internal_hints(draft);
    if source == DraftSource::Exact {
        return Verdict::Keep(cleaned);
    }

    // "You're not answering" → answer the previous question properly.
    if is_feedback_complaint(input) {
        let prev_question = previous
            .map(|(u, _)| u.as_str())
            .filter(|u| !is_feedback_complaint(u));
        // Re-answer only a question Perci can say something specific about:
        // a knowledge/plan/troubleshooting request, a saved note, or Perci itself.
        let answerable = prev_question.filter(|u| {
            classify(u) != RequestKind::Open || !notes.is_empty() || perci_self_reply(u).is_some()
        });
        return Verdict::Replace {
            text: match answerable {
                Some(prev_user) => format!(
                    "You're right, that didn't answer \"{}\". {}",
                    subject(prev_user),
                    honest_reply(classify(prev_user), prev_user, notes, model_configured)
                ),
                None => {
                    "Sorry, I missed it. Ask me the question again in one line and I'll answer it \
directly, or tell you plainly what I can't do with it."
                        .to_owned()
                }
            },
            reason: "answer gate: feedback re-answers the previous question",
        };
    }

    let kind = classify(input);
    if is_meta_deflection(draft) {
        // The same question again: answer it again instead of talking about
        // the repetition.
        if let Some((prev_user, prev_answer)) = previous {
            let same = lowered(prev_user).trim() == lowered(input).trim();
            if same && !is_meta_deflection(prev_answer) && !prev_answer.trim().is_empty() {
                return Verdict::Replace {
                    text: prev_answer.trim().to_owned(),
                    reason: "answer gate: repeated question re-answered instead of meta text",
                };
            }
        }
        return Verdict::Replace {
            text: honest_reply(kind, input, notes, model_configured),
            reason: "answer gate: meta deflection replaced",
        };
    }
    if source == DraftSource::Model {
        return Verdict::Keep(cleaned);
    }
    // The user's own saved note beats a generic operator answer for a
    // question about their world ("where is the spare key").
    if !notes.is_empty() && kind != RequestKind::Open && !is_about_perci(input) {
        return Verdict::Replace {
            text: honest_reply(kind, input, notes, model_configured),
            reason: "answer gate: saved note answers the question",
        };
    }
    let substance = strip_filler(&cleaned);
    let words = substance.split_whitespace().count();
    if source == DraftSource::Operator {
        // Named operators own their prose; only drop filler, and replace the
        // draft only if nothing is left.
        return if words == 0 {
            Verdict::Replace {
                text: honest_reply(kind, input, notes, model_configured),
                reason: "answer gate: operator draft was only filler",
            }
        } else {
            Verdict::Keep(substance)
        };
    }
    // Open path.
    if kind != RequestKind::Open || !notes.is_empty() {
        return Verdict::Replace {
            text: honest_reply(kind, input, notes, model_configured),
            reason: "answer gate: ungrounded knowledge request answered honestly",
        };
    }
    if is_about_perci(input) {
        if let Some(text) = perci_self_reply(input) {
            return Verdict::Replace {
                text,
                reason: "answer gate: self-description",
            };
        }
    }
    if words < MIN_SUBSTANTIVE_WORDS {
        return Verdict::Replace {
            text: honest_reply(kind, input, notes, model_configured),
            reason: "answer gate: empty or filler-only draft replaced",
        };
    }
    Verdict::Keep(substance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn<'a>(input: &'a str, draft: &'a str, source: DraftSource) -> Turn<'a> {
        Turn {
            input,
            draft,
            source,
            previous: None,
            notes: &[],
            model_configured: false,
        }
    }

    // Prompts here are deliberately not copied from Perci's rule tables.
    #[test]
    fn classifies_request_kinds() {
        use RequestKind::*;
        for (prompt, kind) in [
            ("who painted the night watch?", Factual),
            ("how many moons does neptune have", Factual),
            ("how can i renew a passport by mail", HowTo),
            ("show me a python function that dedupes a list", Code),
            ("explain rust lifetimes", Code),
            ("compose a limerick about a lazy cat", Creative),
            ("plan a migration to postgres", Plan),
            ("design a schema for a recipe site", Plan),
            (
                "my nginx box keeps timing out after the update",
                Troubleshoot,
            ),
            ("why is my server not answering requests", Troubleshoot),
            ("what do you think about memory", Open),
            ("you're not answering my question", Open),
            ("I like rainy mornings", Open),
            ("what is bitwork", Open),
        ] {
            assert_eq!(classify(prompt), kind, "{prompt}");
        }
    }

    #[test]
    fn feedback_is_anchored_to_second_person() {
        // B2E-PERCI-003: the over-broad pattern caught support questions.
        assert!(is_feedback_complaint("you're not answering me"));
        assert!(is_feedback_complaint("that’s not what i asked"));
        assert!(!is_feedback_complaint(
            "why is my server not answering requests"
        ));
        assert!(!is_feedback_complaint(
            "the api is not answering on port 80"
        ));
    }

    #[test]
    fn feedback_re_answers_the_previous_question() {
        let prev = (
            "why is my vps refusing connections".to_owned(),
            "Meaning emerges where a mind and a world meet.".to_owned(),
        );
        let t = Turn {
            previous: Some(&prev),
            ..turn(
                "you didn't answer my question",
                "I hear you.",
                DraftSource::Operator,
            )
        };
        let out = review(t);
        assert!(out.text().starts_with("You're right"), "{}", out.text());
        assert!(out.text().contains("curl -v"), "{}", out.text());
    }

    #[test]
    fn scaffold_hints_are_stripped() {
        // B2E-PERCI-008
        let leaked = "I hear you. Practical angle: [Perci dialogue workspace: act=answer goal=explain topic=ser…. Next.";
        let out = strip_internal_hints(leaked);
        assert!(!out.contains("[Perci"), "{out}");
        assert!(out.starts_with("I hear you."), "{out}");
        assert_eq!(
            strip_internal_hints("Start [Perci context card: x=y] end"),
            "Start end"
        );
        assert_eq!(
            strip_internal_hints("A [Recent dialogue] B [Perci x: y] C"),
            "A B C"
        );
    }

    #[test]
    fn filler_sentences_are_removed() {
        let draft = "Carrying that thread forward. Yeah, that friction is real. That keeps the answer anchored to feel / stuck.";
        assert_eq!(strip_filler(draft), "Yeah, that friction is real.");
    }

    #[test]
    fn ungrounded_factual_question_gets_an_honest_reply() {
        let draft = "The cleanest read: keep the invariant explicit. That keeps the answer anchored to painter / night.";
        match review(turn(
            "who painted the night watch",
            draft,
            DraftSource::Open,
        )) {
            Verdict::Replace { text, .. } => {
                assert!(text.contains("reliably offline"), "{text}");
                assert!(text.contains("perci doctor"), "{text}");
                assert!(!text.contains("invariant"));
            }
            other => panic!("expected replace, got {other:?}"),
        }
    }

    #[test]
    fn plans_get_a_real_procedure() {
        let out = review(turn(
            "plan a move from mysql onto cockroachdb",
            "Each milestone should leave a usable state.",
            DraftSource::Open,
        ));
        let text = out.text();
        assert!(
            text.contains("onto cockroachdb") || text.contains("cockroachdb"),
            "{text}"
        );
        assert!(text.contains("checksums"), "{text}");
        assert!(text.contains("rollback"), "{text}");
    }

    #[test]
    fn matching_note_leads_the_reply() {
        let notes = relevant_notes(
            "where is the router admin password",
            &["[note] the router admin password is on the yellow card".into()],
        );
        assert_eq!(notes.len(), 1);
        let t = Turn {
            notes: &notes,
            ..turn(
                "where is the router admin password",
                "filler",
                DraftSource::Open,
            )
        };
        let text = review(t).text().to_owned();
        assert!(text.starts_with("From your saved notes:"), "{text}");
        assert!(text.contains("yellow card"));
    }

    #[test]
    fn weakly_related_notes_are_not_quoted() {
        let notes = relevant_notes(
            "how does a hash map work",
            &["finish the work report on friday".into()],
        );
        assert!(notes.is_empty());
    }

    #[test]
    fn repeated_question_is_answered_again_not_deflected() {
        let prev = (
            "how tall is the eiffel tower".to_owned(),
            "About 330 m.".to_owned(),
        );
        let draft = "That repeats the last answer without adding a new layer to eiffel tower.";
        let t = Turn {
            previous: Some(&prev),
            ..turn("how tall is the eiffel tower", draft, DraftSource::Operator)
        };
        assert_eq!(review(t).text(), "About 330 m.");
    }

    #[test]
    fn troubleshooting_gets_concrete_steps() {
        let out = honest_reply(
            RequestKind::Troubleshoot,
            "the api on my vps stopped responding",
            &[],
            false,
        );
        assert!(out.contains("curl -v"));
        assert!(out.contains("log"));
        let build = honest_reply(
            RequestKind::Troubleshoot,
            "my project fails to compile since i upgraded node",
            &[],
            false,
        );
        assert!(build.contains("first error"), "{build}");
    }

    #[test]
    fn questions_about_perci_get_a_self_description() {
        let out = review(turn(
            "how does perci pick a reply",
            "I'm tracking perci reply. The last answer missed your point.",
            DraftSource::Open,
        ));
        assert!(
            out.text().contains("4,096") || out.text().contains("fixed order"),
            "{}",
            out.text()
        );
    }

    #[test]
    fn exact_operator_and_model_answers_are_kept() {
        let draft = "Paris is the capital of France.";
        for source in [
            DraftSource::Model,
            DraftSource::Exact,
            DraftSource::Operator,
        ] {
            assert_eq!(
                review(turn("capital of france?", draft, source)),
                Verdict::Keep(draft.into())
            );
        }
    }

    #[test]
    fn substantive_open_conversation_is_kept() {
        let draft = "Rainy mornings slow everything down, which is part of the appeal: the light is soft and nobody expects much.";
        assert_eq!(
            review(turn("I like rainy mornings", draft, DraftSource::Open)),
            Verdict::Keep(draft.into())
        );
    }
}
