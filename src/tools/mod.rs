//! Perci as a tool: a catalog of stateless, deterministic checks that AI
//! harnesses call (MCP stdio via `perci mcp`, or one-shot `perci tool`).
//!
//! Every tool returns a [`crate::verdict::Verdict`]. Tools never go through
//! the chat engine, never read or write the chat session, and never touch the
//! learning log. The only tool that writes memory is `perci.remember` (off by
//! default); it appends to the journal through [`crate::cited_memory`].
//!
//! Boundary: nothing under `src/tools/` may use `chat`, `voice`,
//! `deliberation` or `session` (enforced by `tests/tool_boundary.rs`). That is
//! the seam along which `perci-core` will be split out as its own crate.
//!
//! # Extension points
//! * [`register`] adds a tool to the catalog at runtime (for example
//!   `perci.eval` backed by bug2eval). Registered tools override built-ins
//!   with the same name.
//! * `perci.recall` / `perci.remember` go through
//!   [`crate::cited_memory::CitedMemory`] (AgentSearch-backed, substring
//!   fallback); [`Ctx::with_memory`] injects a different journal.
//! * `perci.eval` goes through [`crate::b2e`] (bug2eval subprocess).

pub mod agent;
pub mod check_math;
pub mod eval;
pub mod exact;
pub mod ledger;
pub mod mcp;
pub mod policy;
pub mod recall;
pub mod relation;
pub mod route;
pub mod status;
pub mod temporal;
pub mod verify;

use crate::verdict::{Outcome, Verdict};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Largest single text argument a tool accepts (32 KiB).
pub const MAX_TEXT_BYTES: usize = 32 * 1024;

/// One tool in the catalog.
pub trait Tool: Send + Sync {
    /// Canonical id, e.g. `perci.exact` (used in verdicts and receipts).
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// JSON Schema of `args`.
    fn input_schema(&self) -> Value;
    fn read_only(&self) -> bool {
        true
    }
    fn idempotent(&self) -> bool {
        true
    }
    fn destructive(&self) -> bool {
        false
    }
    /// Whether the tool is exposed when `PERCI_TOOLS` is unset.
    fn default_enabled(&self) -> bool {
        true
    }
    /// Output depends only on args and the output config, so the verdict
    /// may be served from the per-process cache.
    fn cacheable(&self) -> bool {
        false
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome;
}

/// Shared, lazily built state for one server process. Weights and the policy
/// file load once; every call after that is warm.
#[derive(Default)]
pub struct Ctx {
    pub(crate) packs: Mutex<std::collections::BTreeMap<String, Arc<route::LoadedPack>>>,
    pub(crate) policy: OnceLock<policy::LoadedPolicy>,
    /// Cited memory, opened on first use (`CitedMemory::open_default`).
    memory: Arc<Mutex<Option<crate::cited_memory::CitedMemory>>>,
    /// The run this process records verdicts into (see [`ledger`]).
    run: Mutex<Option<ledger::RunLedger>>,
    /// Verdicts of deterministic tools by input hash + output config.
    cache: Mutex<VerdictCache>,
    /// Learned step-transition model (`perci ledger learn`), loaded once.
    temporal: OnceLock<(temporal::Model, String)>,
    /// Transitions of the current run, updated on every append (so
    /// `next_step` never rebuilds it).
    live: Mutex<temporal::Model>,
}

/// Bounded FIFO map of sealed verdicts.
#[derive(Default)]
pub struct VerdictCache {
    map: std::collections::HashMap<String, Verdict>,
    order: std::collections::VecDeque<String>,
    pub hits: u64,
    pub misses: u64,
}

const CACHE_CAPACITY: usize = 4096;

impl VerdictCache {
    fn get(&mut self, key: &str) -> Option<Verdict> {
        let v = self.map.get(key).cloned();
        if v.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        v
    }
    fn put(&mut self, key: String, v: Verdict) {
        if self.map.insert(key.clone(), v).is_none() {
            self.order.push_back(key);
            while self.order.len() > CACHE_CAPACITY {
                if let Some(old) = self.order.pop_front() {
                    self.map.remove(&old);
                }
            }
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl Ctx {
    pub fn new() -> Self {
        Self::default()
    }

    /// Use this memory instead of `CitedMemory::open_default()`.
    pub fn with_memory(memory: crate::cited_memory::CitedMemory) -> Self {
        let ctx = Self::default();
        *ctx.memory.lock().unwrap_or_else(|e| e.into_inner()) = Some(memory);
        ctx
    }

    /// Load the routing pack and policy now instead of on the first call.
    pub fn prewarm(&self) {
        let _ = route::pack(self, None);
        let _ = policy::loaded(self);
    }

    /// Sync notes and start AgentSearch on a background thread, so the first
    /// `perci.recall` is warm. A recall issued meanwhile waits for it.
    pub fn prewarm_memory(&self) {
        let slot = Arc::clone(&self.memory);
        std::thread::spawn(move || {
            let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            let mem = guard.get_or_insert_with(crate::cited_memory::CitedMemory::open_default);
            let _ = mem.sync();
            mem.warm_up();
        });
    }

    /// Run `f` on the cited memory, opening it on first use.
    pub fn memory<R>(&self, f: impl FnOnce(&mut crate::cited_memory::CitedMemory) -> R) -> R {
        let mut guard = self.memory.lock().unwrap_or_else(|e| e.into_inner());
        f(guard.get_or_insert_with(crate::cited_memory::CitedMemory::open_default))
    }

    /// Record into run `run_id` (opened or resumed; an existing ledger is
    /// verified first and is never appended to if its chain is broken).
    pub fn set_run(&self, run_id: &str) -> std::io::Result<()> {
        let l = ledger::RunLedger::open(run_id)?;
        let mut live = temporal::Model::new();
        if l.broken.is_none() {
            for e in &l.entries {
                live.push(e.token(), &e.status);
            }
        }
        *self.live.lock().unwrap_or_else(|e| e.into_inner()) = live;
        *self.run.lock().unwrap_or_else(|e| e.into_inner()) = Some(l);
        Ok(())
    }

    /// `PERCI_RUN_ID` when set (and `PERCI_LEDGER` is not `off`).
    pub fn set_run_from_env(&self) -> Option<String> {
        if std::env::var("PERCI_LEDGER").is_ok_and(|v| v == "off" || v == "0") {
            return None;
        }
        let id = std::env::var("PERCI_RUN_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())?;
        match self.set_run(id.trim()) {
            Ok(()) => Some(id),
            Err(e) => {
                eprintln!("perci: ledger disabled: {e}");
                None
            }
        }
    }

    pub fn run_id(&self) -> Option<String> {
        self.run
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|r| r.run_id.clone())
    }

    /// Verified entries of `run_id` (default: the current run). `None` when
    /// there is no such run context or its chain is broken.
    pub fn ledger_entries(&self, run_id: Option<&str>) -> Option<Vec<ledger::Entry>> {
        self.with_ledger_entries(run_id, |e| e.to_vec())
    }

    /// [`Self::ledger_entries`] without copying the current run. `f` runs
    /// under the ledger lock: it must not call back into the run methods.
    pub fn with_ledger_entries<R>(
        &self,
        run_id: Option<&str>,
        f: impl FnOnce(&[ledger::Entry]) -> R,
    ) -> Option<R> {
        let guard = self.run.lock().unwrap_or_else(|e| e.into_inner());
        match (run_id, guard.as_ref()) {
            (None, Some(r)) => r.broken.is_none().then(|| f(&r.entries)),
            (Some(id), Some(r)) if r.run_id == id => r.broken.is_none().then(|| f(&r.entries)),
            (Some(id), _) => {
                drop(guard);
                ledger::verify_run(id)
                    .ok()
                    .filter(|v| v.ok)
                    .map(|v| f(&v.parsed))
            }
            (None, None) => None,
        }
    }

    /// Run `f` on the current run's live transition model.
    pub fn live_model<R>(&self, f: impl FnOnce(&temporal::Model) -> R) -> R {
        f(&self.live.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Run `f` on the saved temporal model and its sha256.
    pub fn temporal<R>(&self, f: impl FnOnce(&temporal::Model, &str) -> R) -> R {
        let (m, sha) = self.temporal.get_or_init(temporal::load_saved);
        f(m, sha)
    }

    pub fn cache_contains(&self, input_sha256: &str) -> bool {
        let c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        c.map.keys().any(|k| k.starts_with(input_sha256))
    }

    /// `(entries, hits, misses)` of the verdict cache.
    pub fn cache_stats(&self) -> (usize, u64, u64) {
        let c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        (c.len(), c.hits, c.misses)
    }

    /// `(backend, why unavailable)` without blocking or starting anything.
    pub fn memory_backend(&self) -> (String, Option<String>) {
        match self.memory.try_lock() {
            Ok(guard) => match guard.as_ref() {
                Some(m) => (
                    m.backend_name().to_owned(),
                    m.unavailable_reason().map(str::to_owned),
                ),
                None => ("not started".to_owned(), None),
            },
            Err(_) => ("warming up".to_owned(), None),
        }
    }
}

type Registered = Arc<dyn Tool>;

fn extensions() -> &'static Mutex<Vec<Registered>> {
    static EXT: OnceLock<Mutex<Vec<Registered>>> = OnceLock::new();
    EXT.get_or_init(|| Mutex::new(Vec::new()))
}

/// Add (or override) a tool in the catalog. Call before serving.
pub fn register(tool: Arc<dyn Tool>) {
    let mut ext = extensions().lock().unwrap_or_else(|e| e.into_inner());
    ext.retain(|t| t.name() != tool.name());
    ext.push(tool);
}

/// Every tool, built-ins first, with registered extensions overriding by name.
pub fn catalog() -> Vec<Registered> {
    let mut tools: Vec<Registered> = vec![
        Arc::new(exact::Exact),
        Arc::new(check_math::CheckMath),
        Arc::new(relation::Relation),
        Arc::new(route::Route),
        Arc::new(policy::PolicyCheck),
        Arc::new(recall::Recall),
        Arc::new(recall::Remember),
        Arc::new(verify::Verify),
        Arc::new(eval::Eval),
        Arc::new(status::Status),
        Arc::new(agent::VerifyPatch),
        Arc::new(agent::CheckClaims),
        Arc::new(agent::GateStep),
        Arc::new(ledger::LedgerAppend),
        Arc::new(ledger::LedgerVerify),
        Arc::new(ledger::LedgerShow),
        Arc::new(temporal::NextStep),
    ];
    let ext = extensions().lock().unwrap_or_else(|e| e.into_inner());
    for t in ext.iter() {
        tools.retain(|b| b.name() != t.name());
        tools.push(Arc::clone(t));
    }
    tools
}

/// `perci.agent.verify_patch` → `agent_verify_patch`. MCP clients (and the
/// model APIs behind them) restrict function names to `[A-Za-z0-9_-]`, so
/// the wire name drops the `perci.` prefix and uses underscores.
pub fn wire_name(name: &str) -> String {
    name.strip_prefix("perci.")
        .unwrap_or(name)
        .replace('.', "_")
}

/// Accept the canonical id, the wire name, or the bare short name.
pub fn resolve(tools: &[Registered], requested: &str) -> Option<Registered> {
    let req = requested.trim();
    tools
        .iter()
        .find(|t| {
            t.name() == req || wire_name(t.name()) == req || t.name() == format!("perci.{req}")
        })
        .cloned()
}

/// Tools exposed under `PERCI_TOOLS` (comma list of short or wire names,
/// `all`, or unset for the default set).
pub fn enabled(tools: Vec<Registered>, filter: Option<&str>) -> Vec<Registered> {
    let Some(filter) = filter.map(str::trim).filter(|f| !f.is_empty()) else {
        return tools.into_iter().filter(|t| t.default_enabled()).collect();
    };
    if filter.eq_ignore_ascii_case("all") {
        return tools;
    }
    let wanted: Vec<String> = filter.split(',').map(|s| s.trim().to_owned()).collect();
    tools
        .into_iter()
        .filter(|t| {
            wanted.iter().any(|w| {
                w == t.name() || *w == wire_name(t.name()) || format!("perci.{w}") == t.name()
            })
        })
        .collect()
}

/// `PERCI_TOOLS`, else the profile's `tools.enabled` list, else the
/// default set.
pub fn enabled_from_env() -> Vec<Registered> {
    let from_env = std::env::var("PERCI_TOOLS")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let filter = from_env.or_else(|| {
        crate::profile::config()
            .pointer("/tools/enabled")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .filter(|s| !s.is_empty())
    });
    enabled(catalog(), filter.as_deref())
}

/// Run one tool and seal its verdict. Panics inside a tool become an
/// `error` verdict instead of killing the server.
pub fn run(ctx: &Ctx, tool: &dyn Tool, args: &Value) -> Verdict {
    let t0 = Instant::now();
    let outcome = if !args.is_object() {
        Outcome::error("BAD_ARGS", "arguments must be a JSON object")
    } else {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tool.call(ctx, args))) {
            Ok(o) => o,
            Err(_) => Outcome::error("INTERNAL", "tool panicked; this is a Perci bug"),
        }
    };
    let elapsed = t0.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    Verdict::seal(tool.name(), args, outcome, elapsed)
}

fn cache_enabled() -> bool {
    !std::env::var("PERCI_TOOL_CACHE").is_ok_and(|v| v == "0" || v == "off")
}

/// Meta tools that read the ledger are not recorded in it.
fn recorded(name: &str) -> bool {
    !matches!(
        name,
        "perci.ledger_show" | "perci.ledger_verify" | "perci.next_step" | "perci.status"
    )
}

/// [`run`] plus the verdict cache (deterministic tools, same input hash and
/// output config) and the run ledger (every call is appended when the
/// context has a run; the receipt then names the entry).
pub fn run_recorded(ctx: &Ctx, tool: &dyn Tool, args: &Value) -> Verdict {
    let name = tool.name();
    let t0 = Instant::now();
    let key = (tool.cacheable() && cache_enabled() && args.is_object()).then(|| {
        format!(
            "{}|{}",
            crate::verdict::input_hash(name, args),
            crate::verdict::canonical_json(
                &serde_json::to_value(crate::verdict::output_config()).unwrap_or(Value::Null)
            )
        )
    });
    let cached = key
        .as_ref()
        .and_then(|k| ctx.cache.lock().unwrap_or_else(|e| e.into_inner()).get(k));
    let mut verdict = match cached {
        Some(mut v) => {
            v.receipt.cache_hit = true;
            v.receipt.elapsed_us = t0.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
            v.receipt.ledger = None;
            v
        }
        None => {
            let v = run(ctx, tool, args);
            if let Some(k) = key {
                if v.status != crate::verdict::Status::Error {
                    ctx.cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .put(k, v.clone());
                }
            }
            v
        }
    };
    if !recorded(name) {
        return verdict;
    }
    let mut guard = ctx.run.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(l) = guard.as_mut() {
        let (phase, step, status, note) = if name == "perci.ledger_append" {
            if verdict.status != crate::verdict::Status::Pass {
                return verdict;
            }
            let r = &verdict.result;
            (
                r["phase"].as_str().map(str::to_owned),
                r["step"].as_str().unwrap_or("").to_owned(),
                r["status"].as_str().unwrap_or("pass").to_owned(),
                r["note"].as_str().map(str::to_owned),
            )
        } else {
            let (phase, step) = ledger::step_of(name, args);
            (phase, step, verdict.status.as_str().to_owned(), None)
        };
        match l.append(name, phase.as_deref(), &step, &status, &verdict, note) {
            Ok(e) => {
                verdict.receipt.ledger = Some(e.reference());
                let tok = e.token();
                self_live_push(ctx, tok, &status);
            }
            Err(e) => eprintln!("perci: ledger append failed: {e}"),
        }
    }
    verdict
}

fn self_live_push(ctx: &Ctx, tok: String, status: &str) {
    ctx.live
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(tok, status);
}

/// Dispatch by any accepted name over the enabled catalog.
pub fn dispatch(ctx: &Ctx, name: &str, args: &Value) -> Verdict {
    let tools = enabled_from_env();
    match resolve(&tools, name) {
        Some(t) => run_recorded(ctx, t.as_ref(), args),
        None => Verdict::seal(
            name,
            args,
            Outcome::error("UNKNOWN_TOOL", format!("no enabled tool named {name:?}")),
            0,
        ),
    }
}

// ---- argument helpers -------------------------------------------------------

pub(crate) fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, Outcome> {
    match args.get(key) {
        Some(Value::String(s)) if s.len() > MAX_TEXT_BYTES => Err(Outcome::error(
            "BAD_ARGS",
            format!("{key} is longer than {MAX_TEXT_BYTES} bytes"),
        )),
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s),
        Some(Value::String(_)) => Err(Outcome::error("BAD_ARGS", format!("{key} is empty"))),
        Some(_) => Err(Outcome::error(
            "BAD_ARGS",
            format!("{key} must be a string"),
        )),
        None => Err(Outcome::error(
            "BAD_ARGS",
            format!("missing required argument {key}"),
        )),
    }
}

pub(crate) fn opt_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, Outcome> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => arg_str(args, key).map(Some),
    }
}

pub(crate) fn opt_str_list(args: &Value, key: &str) -> Result<Vec<String>, Outcome> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::String(s) if s.len() <= MAX_TEXT_BYTES => Ok(s.clone()),
                _ => Err(Outcome::error(
                    "BAD_ARGS",
                    format!("{key} must be an array of strings"),
                )),
            })
            .collect(),
        Some(_) => Err(Outcome::error(
            "BAD_ARGS",
            format!("{key} must be an array of strings"),
        )),
    }
}

/// Small helper so tools can write `json!` results with a summary line.
pub(crate) fn with_summary(mut result: Value, summary: impl Into<String>) -> Value {
    if let Some(obj) = result.as_object_mut() {
        obj.insert("summary".into(), json!(summary.into()));
    }
    result
}

macro_rules! try_arg {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(outcome) => return outcome,
        }
    };
}
pub(crate) use try_arg;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_are_function_safe() {
        for t in catalog() {
            let w = wire_name(t.name());
            assert!(
                w.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
                "{w}"
            );
            assert!(w.len() <= 64);
        }
        assert_eq!(wire_name("perci.agent.verify_patch"), "agent_verify_patch");
    }

    #[test]
    fn resolve_accepts_all_name_forms() {
        let tools = catalog();
        for n in ["perci.exact", "exact"] {
            assert_eq!(resolve(&tools, n).unwrap().name(), "perci.exact");
        }
        assert_eq!(
            resolve(&tools, "agent_check_claims").unwrap().name(),
            "perci.agent.check_claims"
        );
    }

    #[test]
    fn default_set_is_read_only() {
        // The only default write is ledger_append: append-only, to Perci's
        // own run ledger, never to user files or memory.
        for t in enabled(catalog(), None) {
            assert!(
                t.read_only() || t.name() == "perci.ledger_append",
                "{} is in the default set but writes",
                t.name()
            );
            assert!(!t.destructive(), "{} is destructive", t.name());
        }
        assert!(enabled(catalog(), Some("exact,remember")).len() == 2);
        assert_eq!(enabled(catalog(), Some("all")).len(), catalog().len());
    }

    #[test]
    fn non_object_args_are_an_error_verdict() {
        let ctx = Ctx::new();
        let v = run(&ctx, &exact::Exact, &json!("2+2"));
        assert_eq!(v.status, crate::verdict::Status::Error);
    }
}
