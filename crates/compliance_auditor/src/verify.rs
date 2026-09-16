//! Adversarial verification of finder-angle candidates.
//!
//! A finder angle is deliberately recall-biased — it's told to surface
//! anything with a nameable failure scenario rather than self-censor. That
//! means its raw output is not yet a finding; each candidate gets checked by
//! a separate call that has no stake in having "found" anything and is
//! instructed to refute unless the defect is actually real. Only
//! `Confirmed`/`Plausible` verdicts survive into the report.

use anyhow::{Context, Result};
use llm_gateway::LLMClient;
use serde::{Deserialize, Serialize};

use crate::angles::Candidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Confirmed,
    Plausible,
    Refuted,
}

impl Verdict {
    fn parse(word: &str) -> Option<Verdict> {
        let w = word.trim().trim_matches(|c: char| !c.is_alphabetic()).to_uppercase();
        match w.as_str() {
            "CONFIRMED" => Some(Verdict::Confirmed),
            "PLAUSIBLE" => Some(Verdict::Plausible),
            "REFUTED" => Some(Verdict::Refuted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub candidate: Candidate,
    pub verdict: Verdict,
    pub justification: String,
}

const VERIFIER_SYSTEM_PROMPT: &str = "\
You are verifying one candidate finding from a code review against the diff it was raised \
against. Be recall-biased but honest: a candidate is PLAUSIBLE by default when the failure \
state is realistic (a concurrency race, a rare-but-reachable nil/error path, an off-by-one on \
a boundary the code doesn't exclude, a retry/partial-failure interaction) even if it depends on \
runtime conditions you can't observe directly. Only answer REFUTED when you can point at why \
it's factually wrong, provably impossible given the code's actual types/constants/invariants, \
already handled elsewhere in the diff, or is pure style with no observable effect. Answer \
CONFIRMED when you can trace the exact mechanism end to end. \
Respond with exactly one verdict word first — CONFIRMED, PLAUSIBLE, or REFUTED — on its own, \
then a newline, then a one-paragraph justification.";

/// Verify one candidate. On a malformed or unparseable model response, the
/// candidate is kept as `Plausible` rather than silently dropped — an
/// inconclusive verifier run is not evidence the finding is wrong.
pub async fn verify_one(client: &LLMClient, candidate: &Candidate) -> Result<Finding> {
    let user = format!(
        "File: {}\nLine: {:?}\nAngle: {:?}\nSummary: {}\nFailure scenario: {}\n\n\
         Verify this against the actual file/diff content you can retrieve; do not just take \
         the summary's word for it.",
        candidate.file, candidate.line, candidate.angle, candidate.summary, candidate.failure_scenario
    );

    let response = client
        .chat(VERIFIER_SYSTEM_PROMPT, &user, "compliance_audit")
        .await
        .context("verifier chat call failed")?;

    let mut lines = response.lines();
    let verdict = lines
        .next()
        .and_then(Verdict::parse)
        .unwrap_or(Verdict::Plausible);
    let justification = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    let justification = if justification.is_empty() {
        response.trim().to_string()
    } else {
        justification
    };

    Ok(Finding {
        candidate: candidate.clone(),
        verdict,
        justification,
    })
}

/// Verify every candidate concurrently, then drop anything Refuted.
/// Verification runs one call per candidate rather than batching, so each
/// verdict is reached without seeing (and being anchored by) the others.
pub async fn verify_all(client: &LLMClient, candidates: &[Candidate]) -> Vec<Finding> {
    let pending = candidates.iter().map(|c| verify_one(client, c));
    let results = futures::future::join_all(pending).await;

    results
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(|f| f.verdict != Verdict::Refuted)
        .collect()
}

/// Rank findings most-severe-first (correctness angles ahead of cleanup
/// angles, Confirmed ahead of Plausible within a tier) and cap the list —
/// mirrors how a human reviewer would triage a long list down to what's
/// worth acting on first.
pub fn rank_and_cap(mut findings: Vec<Finding>, cap: usize) -> Vec<Finding> {
    findings.sort_by_key(|f| {
        let angle_rank = if f.candidate.angle.is_correctness() { 0 } else { 1 };
        let verdict_rank = match f.verdict {
            Verdict::Confirmed => 0,
            Verdict::Plausible => 1,
            Verdict::Refuted => 2,
        };
        (angle_rank, verdict_rank)
    });
    findings.truncate(cap);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angles::{Angle, Candidate};

    fn finding(angle: Angle, verdict: Verdict) -> Finding {
        Finding {
            candidate: Candidate {
                angle,
                file: "a.rs".to_string(),
                line: Some(1),
                summary: "s".to_string(),
                failure_scenario: "f".to_string(),
            },
            verdict,
            justification: "j".to_string(),
        }
    }

    #[test]
    fn verdict_parse_is_case_and_punctuation_tolerant() {
        assert_eq!(Verdict::parse("CONFIRMED"), Some(Verdict::Confirmed));
        assert_eq!(Verdict::parse("confirmed."), Some(Verdict::Confirmed));
        assert_eq!(Verdict::parse("  Plausible  "), Some(Verdict::Plausible));
        assert_eq!(Verdict::parse("REFUTED!"), Some(Verdict::Refuted));
        assert_eq!(Verdict::parse("not a verdict"), None);
    }

    #[test]
    fn rank_and_cap_puts_confirmed_correctness_first_and_truncates() {
        let findings = vec![
            finding(Angle::Simplification, Verdict::Confirmed),
            finding(Angle::LineByLine, Verdict::Plausible),
            finding(Angle::LineByLine, Verdict::Confirmed),
        ];

        let ranked = rank_and_cap(findings, 2);

        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].candidate.angle, Angle::LineByLine);
        assert_eq!(ranked[0].verdict, Verdict::Confirmed);
        assert_eq!(ranked[1].candidate.angle, Angle::LineByLine);
        assert_eq!(ranked[1].verdict, Verdict::Plausible);
    }
}
