# Active Coordination Queue

> **Status:** Live coordination ledger for parallel work. The canonical priorities remain in
> [../WORKLIST.md](../WORKLIST.md); this file records ownership and file boundaries.

## Participants

| Handle | Environment | Role |
| --- | --- | --- |
| Codex | Codex desktop | Rust implementation, verification, integration |
| Gemini 3.8 | Antigravity | Independent implementation and review lane |
| Claude Sonnet 5 | Claude | Independent implementation and review lane |
| Qwen | Local, optional | Bounded code-generation or fixture tasks under a named owner |

## Claim protocol

1. Read this file and `WORKLIST.md` before editing code.
2. Claim one bounded item by adding an entry with an owner, branch, exact path scope, and UTC
   start time. Do not edit another active claim's paths.
3. Use a short claim-only commit before implementation whenever the shared branch is involved.
4. Record the commit, verification evidence, and released paths when finished or blocked.
5. A claim expires after 24 hours without an update. Take over an expired claim only after adding
   a handoff note; do not overwrite another agent's uncommitted work.
6. Qwen receives a narrow prompt and writes only to an owner-provided scratch location until the
   owning thread reviews and applies the result.

## Ready lanes

| ID | Worklist source | Scope boundary | Status | Owner | Branch | Started (UTC) | Handoff / evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C1 | B6: native text encoding | `crates/ast_auditor/**`, `scripts/check_text_encoding.py`, CI/docs references | Ready | — | — | — | Port the checker before removing Python. |
| C2 | S1: revalidate legacy security findings | `crates/studio_hud/src/plugin_api.rs` and directly related tests/docs | Ready | — | — | — | Reproduce each finding before repair. |
| C3 | S2: reproducible dependency review | Dependency-audit configuration, report format, and CI only | Ready | — | — | — | Do not update dependencies in the same task. |
| C4 | B6: interface and terminology inventory | `data/fabrication/**`, `registry/**`, `config/**`, documentation only | Ready | — | — | — | Classify contracts before deleting or renaming. |

## Active claims

_No active claim has been recorded in this queue yet. A participant must claim a ready lane before
editing its scoped files._

## Completion records

Add completed claims here with the resulting commit and verification evidence. Keep the entry
concise; detailed findings belong in the worklist, review document, or RFC.

## Historical handoff record

The completed pre-coordination handoff items remain in Git history. They are no longer an active
source of work or ownership.
