# Worker Field Map: what each existing record already carries

**Status:** Working draft, not yet reviewed by a second model family. Not part of the governing document hierarchy. Proposes no code and no schema.
**Written:** 2026-11-19. Product at `origin/main` `cc794cb3`; devtools read from the local checkout (not pinned to a revision), with `contracts/guild-operation-v1.md` at the status it declares itself: PROPOSED, NOT IMPLEMENTED, NOT ADOPTED.
**Author:** Claude seat. The same seat wrote [WORKER_MANUAL.md](WORKER_MANUAL.md), so this is a contributor's map, not an independent review.
**Why it exists:** a pasted proposal (from Gemini, via the owner) defined a worker "job card" in TOML with intent, authority, constraints and evidence. Before adding anything, this map shows which existing record already answers each question, so a new card, if any, extends an existing record. AGENTS.md section 4 forbids a second implementation of an existing component.

## 1. Method

Read for this map: devtools `AGENTS.md`; `governance/COORDINATION_QUEUE.md` (structure only); `governance/LOCAL_WORKER_SERVICE.md` and `worker-jobs/*.toml`; `governance/LOCAL_VERIFICATION_GATE.md`; `ci/suites/*.toml`; `contracts/{intake-proposal,contribution-envelope,virtual-patch-set,local-proposal}-v1.schema.json` (field lists extracted by script); `contracts/guild-operation-v1.md` sections 1 to 4; and in `foundation/crates/steward/src/` the definitions of `cards.rs` `card_for`, `verification.rs` `Request` and `Outcome`, `local_verification.rs` `Claim` and `Runner`, `local_authority.rs` `Principal` and `Policy`, `board.rs` `AppendReceipt`. Field names below are copied from those files. Not read in full: the wire codec, job service, and the other schemas under `contracts/`. A cell says "no" only when the field is absent from the record's schema or struct as read.

## 2. The questions a worker needs answered

From the pasted proposal, with the Guild contract's own words where it names the same thing:

| # | Question | Pasted card field | Guild contract name (proposed) |
| :--- | :--- | :--- | :--- |
| Q1 | What is the job and what counts as done? | `intent.objective`, `success_criteria`, `failure_criteria` | desired outcome, acceptance |
| Q2 | What may I read, write and run? | `authority.read`, `write`, `execute` | read/write scope |
| Q3 | What may I not do, and what authority is withheld? | `authority.merge`, `publish`, `constraints.forbidden` | prohibited effects, authorized effects |
| Q4 | What must I hand back? | `intent.deliverables` | artifact references |
| Q5 | What proves I finished? | `evidence_required.*` | verification association, evidence references |
| Q6 | What limits my cost and time? | none | budget |
| Q7 | Who issued this and how is that checkable? | none | admission reference |
| Q8 | What do I do when blocked? | none (manual text) | none |

## 3. Matrix: existing records against the questions

Legend: **field** = a named field carries it; **prose** = only free text in a field or document; **rule** = enforced by code or policy outside the record; **no** = absent.

| Question | Queue row (claim) | Steward task card (JSON) | Verification request and suite (C132) | Local worker manifest (TOML) | Intake proposal | Contribution envelope | Guild intent revision (proposed) | Pasted job card (proposed) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| Q1 intent and done | prose (`Scope`) | **field** `task`, `acceptance`, `why_now` | no (one fixed suite) | **field** `instruction` | **field** `decision` | **field** `instruction` | **field** | **field** |
| Q2 read, write, run | prose (exact paths, gate named in text) | no | **rule** (argv fixed by the suite at `base_sha`) | **rule** (`mode`; product read-only; no execution; 1 to 8 files, 32 KiB each) | **field** `scope[]` | **field** `target_crate`, `target_file` | **field** | **field** (globs) |
| Q3 withheld authority | **rule** (process: claim only `Ready`; owner merges product) | no (`role` is a label) | **rule** (trusted-base policy) | **rule** (devtools `AGENTS.md`: cannot claim queue work, edit a shared branch, execute commands or declare verification successful) | no (text is "not a grant") | no | **field** | **field** (bools) |
| Q4 deliverable | prose (`Evidence / handoff`) | prose in `acceptance`; `marker` receipt | **rule** (signed receipts, logs outside the checkout) | **rule** (result JSON, 64 KiB cap) | no | **field** (the change itself plus `patch_sha256`) | **field** | **field** |
| Q5 evidence required | prose | prose in `acceptance` | **rule** (every step must pass; missing evidence never passes) | no (result carries an input snapshot) | **field** `verification[]` (proposed checks) | **field** `verification_commands[{command, exit_code}]`, `evidence_key` (recorded after the fact) | **field** | **field** (self-reported in a repo file) |
| Q6 budget | no | no | **field** `timeout_s` per step; log size cap | **rule** (size caps; 120 s model timeout) | no | no | **field** | no |
| Q7 issuer checkable | prose (`Owner` column) | no | **rule** (card issued from the trusted base; node signature checked) | **rule** (reviewed manifest in an approved directory) | no | partly (`independent_review.actor`, claimed not authenticated) | **field** (admission reference with policy digest) | no |
| Q8 when blocked | no | **field** `if_blocked` | **rule** (timeout, failed start, corrupt files fail closed) | **rule** (failures retained, no unbounded retry) | no | no | partly (holds) | no |

Other facts the matrix does not show:

- **A claim is already typed in one place.** `devtools_ops::coordination::LaneClaim` holds `lane_id`, `owner_seat`, `branch`, `start_timestamp_utc` and `owned_paths` (the same facts devtools `AGENTS.md` says a claim records, minus the intended evidence and gate), refuses a claim with no paths, and has `conflicts_with_path`. By grep it is defined and re-exported from `devtools_ops/src/lib.rs` and used nowhere else in `foundation/` except its own tests. It matches exact path strings only, not globs.
- **A task card is a JSON object, not a Rust type.** `card_for` builds a `Map` with `agent`, `role`, `tick`, `priority`, `task`, `why_now`, `inputs{pr, kind}`, `marker`, `acceptance`, `if_blocked`. `enrich_card` adds `lens`, `lens_text` and `attempt`. This agrees with section 7.4 of the Atlas document, which is not on `main` (PR 153).
- **Authority is never carried by a card.** It comes from policy supplied by the trusted base (`Policy` with `minimum_reviewers`, `minimum_families`, `principals`), from the worker service's own code, or from process. The Guild contract proposes carrying admission in the intent revision, issued by an authorized admitter.
- **Evidence is mostly produced after the fact.** The contribution envelope and the verification receipts record what happened. Only the suite manifest and the steward `acceptance` text say beforehand what must be shown.

## 4. Reading the matrix

1. **The pasted card is not a new concept.** Its fields line up almost one to one with the Guild contract's proposed intent revision (desired outcome, acceptance, read/write scope, prohibited effects, authorized effects). It would be a serialization of that revision for a worker, not a fifth schema. The Guild contract says to reuse existing records and not introduce a universal trait, and that is the constraint a card would work inside.
2. **A TOML manifest already exists for support models.** The `worker-jobs/*.toml` format has five fields and enforces its limits in code. An earlier note in this session said a TOML card would introduce a second format. That was wrong in that narrow respect: TOML job manifests are already in use, so format is not an objection. The objection that remains is that the existing manifest has no authority, success, evidence or budget fields.
3. **Four questions have no machine-checkable answer in any record today:** what the worker is authorized to write (Q2 and Q3 outside the verification path), what evidence is required before starting (Q5), budget (Q6 outside the verification suite), and who issued the job (Q7 outside verification and the worker manifest). These are exactly the fields the Guild contract proposes, and they are the part of the pasted card that cannot safely live in a repo file the worker can edit.
4. **The only enforceable piece of the pasted proposal with an existing home is path scope.** `LaneClaim.owned_paths` plus `conflicts_with_path` already expresses "this worker may write these paths". A check that every path in a PR diff falls inside a claim's owned paths would extend an existing type instead of adding one. It is a hypothesis that this check would have caught real scope creep; no case is documented here.
5. **Evidence the worker reports is a claim.** The pasted `evidence_required.metrics = exit_code` would be worker-reported. The verification path already refuses that: the node runs base-defined commands and signs receipts kept outside the checkout. A worker card should name evidence to be reproduced by a verifier, not a number to be asserted.

## 5. What the manual draws on

[WORKER_MANUAL.md](WORKER_MANUAL.md) uses only what exists today. Its sections map as follows:

| Manual section | Source today |
| :--- | :--- |
| 1 Get one job | Devtools `AGENTS.md` "Before selecting work" and "Claiming a lane"; `LaneClaim` |
| 2 Stay inside it | Claims own exact paths; product `AGENTS.md` and the owner-gated merge rule; local worker service limits |
| 3 Prove it | Product `AGENTS.md` section 6; task card `acceptance`; contribution envelope `verification_commands` |
| 4 Do not grade your own work | Independence rules and `Policy.minimum_families`; the `marker` receipt rule in `card_for` |
| 5 If you are stuck | Task card `if_blocked`; fail-closed behavior in the verification gate |
| 6 Hand off | Devtools `AGENTS.md` "Completion and handoff" |

## 6. Gaps and decisions for the owner

None of these is started.

| # | Gap or decision | Note |
| :--- | :--- | :--- |
| G1 | No per-job authority record a worker cannot edit | The Guild contract proposes it; adoption is the owner's and needs review (its section 8) |
| G2 | No success or failure criteria field outside the steward card's free-text `acceptance` | Could be structured inside the intent revision rather than a new card |
| G3 | `LaneClaim` has no consumers and no glob support | Either wire it into a PR-diff check or record why it is unused; do not add a parallel claim type |
| G4 | The worker manifest (`worker-jobs/*.toml`) has no authority or evidence fields | Extending it is possible, but it serves local support models only |
| G5 | The worker manual is not linked from either repository's README | Needs a docs PR in each repository; the product repository is owner-gated |
| G6 | Product `AGENTS.md` is 285 lines and the manual's three-minute target is the pasted text's test, not a measured result | Measure before claiming the manual fixes skimming |

## 7. Status of this map's claims

| Claim | Status | Basis |
| :--- | :--- | :--- |
| Field names in the matrix | Observed | Copied from the files listed in section 1 |
| "no" cells | Observed within the files read; not exhaustive | Other contracts and the wire codec were not read in full |
| `LaneClaim` unused | Observed by grep over `foundation/` | Other repositories and scripts were not searched |
| Reading of the pasted card as a view of the intent revision | Hypothesized | Comparison of field lists, not a mapping tested against a real job |
| Path-scope check would catch scope creep | Hypothesized | No case documented |
