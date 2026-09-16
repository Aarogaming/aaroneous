//! Independent "finder" angles that each scan the same diff for a different
//! class of problem, mirroring a multi-reviewer human process: several
//! reviewers look at the same change with different concerns in mind, then
//! their raw suspicions get checked before anyone acts on them (see
//! `crate::verify`).
//!
//! Each angle is a fixed system prompt plus a request for a strict JSON
//! array of candidates. Angles never talk to each other and never see one
//! another's output — that independence is what makes pooling their results
//! meaningful instead of one pass rediscovering another's blind spots.

use anyhow::{Context, Result};
use llm_gateway::LLMClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Angle {
    LineByLine,
    RemovedBehavior,
    Reuse,
    Simplification,
    Efficiency,
    Altitude,
    /// Not run by `run_all_angles` — driven separately by `crate::conventions`
    /// only when a governing CLAUDE.md exists. Kept as an `Angle` variant so
    /// its findings carry the same ranking/reporting shape as the others.
    Conventions,
}

impl Angle {
    pub const ALL: [Angle; 6] = [
        Angle::LineByLine,
        Angle::RemovedBehavior,
        Angle::Reuse,
        Angle::Simplification,
        Angle::Efficiency,
        Angle::Altitude,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Angle::LineByLine => "line_by_line",
            Angle::RemovedBehavior => "removed_behavior",
            Angle::Reuse => "reuse",
            Angle::Simplification => "simplification",
            Angle::Efficiency => "efficiency",
            Angle::Altitude => "altitude",
            Angle::Conventions => "conventions",
        }
    }

    /// Whether findings from this angle are correctness bugs (outrank
    /// cleanup findings when a report has to cut down to its cap).
    pub fn is_correctness(self) -> bool {
        matches!(self, Angle::LineByLine | Angle::RemovedBehavior | Angle::Conventions)
    }

    fn system_prompt(self) -> &'static str {
        match self {
            Angle::LineByLine => {
                "You are a line-by-line diff reviewer. Read every hunk. For each changed \
                 line, ask what input, state, timing, or platform makes it wrong: inverted \
                 conditions, off-by-one errors, null/None/unwrap-on-None panics, missing \
                 await or blocking calls in async contexts, falsy-zero treated as missing, \
                 copy-paste with the wrong variable, silently swallowed errors, unescaped \
                 format-string or regex metacharacters, integer overflow, unsafe blocks with \
                 broken invariants, and (this is a Rust workspace with shared-memory/IPC and \
                 async orchestration code) concurrency races: torn reads, missing memory \
                 ordering, unchecked buffer bounds. Ignore pure whitespace/line-ending hunks."
            }
            Angle::RemovedBehavior => {
                "You are a removed-behavior auditor. For every line the diff deletes or \
                 replaces, name the invariant, guard, validation, or error path it used to \
                 enforce, then check whether the new code re-establishes it anywhere. If you \
                 cannot find it re-established, that is a candidate: a removed bounds/null \
                 check, a dropped error path that now panics or silently continues, narrowed \
                 validation, or weakened test coverage on a security- or correctness-relevant \
                 path."
            }
            Angle::Reuse => {
                "You are a reuse reviewer. Flag new code the diff adds that re-implements \
                 something the same codebase plausibly already has: parsing, retry/backoff, \
                 serialization, a data structure, validation. Only report a candidate when \
                 you can point at what's duplicated concretely enough for someone to go look \
                 for the existing equivalent — don't guess at file paths you haven't seen."
            }
            Angle::Simplification => {
                "You are a simplification reviewer. Flag unnecessary complexity the diff \
                 adds: redundant or derivable state, copy-paste blocks with a slight \
                 variation that should be one parameterized function, needless nesting, dead \
                 code left alongside a new path, or an abstraction (a trait, a generic \
                 parameter) with no real polymorphism behind it. Name the simpler form."
            }
            Angle::Efficiency => {
                "You are an efficiency reviewer. Flag wasted work the diff introduces: \
                 redundant computation or repeated I/O/lock acquisition in a loop, \
                 independent operations made sequential where they could stay concurrent, \
                 blocking work added to a startup or hot path, or a long-lived \
                 closure/struct now capturing more of its enclosing scope than it needs. \
                 Weigh the actual data sizes/call frequency involved before flagging \
                 something as a real cost, not just a theoretical one."
            }
            Angle::Altitude => {
                "You are an altitude reviewer. For each substantive change, check whether it \
                 fixes the root cause at the right depth, or instead patches a symptom with a \
                 narrow special case bolted onto shared infrastructure (an `if this_specific_id \
                 == X` carve-out inside a generic dispatcher/router/supervisor, a retry policy \
                 hand-rolled for one caller instead of using a general one). Name the deeper, \
                 more general fix."
            }
            Angle::Conventions => {
                unreachable!("Conventions findings are produced by crate::conventions::run, not run_angle")
            }
        }
    }

    fn output_contract(self) -> &'static str {
        if matches!(self, Angle::Conventions) {
            unreachable!("Conventions findings are produced by crate::conventions::run, not run_angle");
        }
        "Respond with ONLY a JSON array (no prose, no markdown fences) of up to 6 objects, \
         each shaped exactly as: \
         {\"file\": \"path/from/diff\", \"line\": <int or null>, \"summary\": \"one line\", \
         \"failure_scenario\": \"concrete input/state/timing that triggers it, or the concrete \
         cost for a cleanup finding\"}. \
         Only include candidates you can point at specific lines in the diff below. If you \
         find nothing for this angle, respond with an empty array []."
    }
}

/// A raw, unverified candidate finding from one angle. Nothing in this
/// struct should be trusted as fact yet — see `crate::verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub angle: Angle,
    pub file: String,
    pub line: Option<u32>,
    pub summary: String,
    pub failure_scenario: String,
}

/// Best-effort extraction of a JSON array from a model response that may
/// still be wrapped in prose or a markdown fence despite instructions.
fn extract_json_array(raw: &str) -> Option<&str> {
    let start = raw.find('[')?;
    let end = raw.rfind(']')?;
    if end < start {
        return None;
    }
    Some(&raw[start..=end])
}

#[derive(Debug, Deserialize)]
struct RawCandidate {
    file: String,
    line: Option<u32>,
    summary: String,
    failure_scenario: String,
}

/// Run a single angle against the given diff text and return its parsed
/// candidates. A malformed or empty model response is treated as "no
/// findings from this angle" rather than an error — one angle's confusion
/// should not sink the whole review.
pub async fn run_angle(client: &LLMClient, angle: Angle, diff_text: &str) -> Result<Vec<Candidate>> {
    let system = format!("{}\n\n{}", angle.system_prompt(), angle.output_contract());
    let user = format!("Diff under review:\n```diff\n{diff_text}\n```");

    let response = client
        .chat(&system, &user, "compliance_audit")
        .await
        .with_context(|| format!("angle {:?} chat call failed", angle))?;

    let Some(json) = extract_json_array(&response) else {
        return Ok(Vec::new());
    };

    let raw: Vec<RawCandidate> = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    Ok(raw
        .into_iter()
        .map(|r| Candidate {
            angle,
            file: r.file,
            line: r.line,
            summary: r.summary,
            failure_scenario: r.failure_scenario,
        })
        .collect())
}

/// Run every angle concurrently against the same diff text and pool the
/// results. Angles are independent by construction (separate prompts,
/// separate calls), so they're issued together and polled concurrently —
/// a slow or failing angle never blocks the others from finishing.
pub async fn run_all_angles(client: &LLMClient, diff_text: &str) -> Vec<Candidate> {
    let pending = Angle::ALL
        .iter()
        .map(|&angle| run_angle(client, angle, diff_text));
    let results = futures::future::join_all(pending).await;

    results
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect()
}

/// Drop near-duplicate candidates: same file, same rounded line, same angle.
/// Keeps the review from reporting the same defect twice because two angles
/// (or the same angle re-run) happened to notice it independently.
pub fn dedup(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in candidates {
        let key = (c.file.clone(), c.line.unwrap_or(0) / 5, c.summary.to_lowercase());
        if seen.insert(key) {
            out.push(c);
        }
    }
    out
}
