# Worker Manual

**Status:** Proposed, not adopted, not yet reviewed by a second model family (a PR review supplies that). It restates how work is already done in `aaroneous` and `aaroneous-devtools`; it adds no new process, record or authority. If it conflicts with `AGENTS.md` (either repository), `AGENTS.md` wins. Where it is linked from (README, `AGENTS.md`) is the owner's decision; this PR links it only from the documentation index. Field-by-field sources are in [WORKER_FIELD_MAP.md](WORKER_FIELD_MAP.md).

You are a worker. You do one bounded job and show evidence. Other people and checks decide whether it counts.

## 1. Get one job

- A job is one of: a `Ready` lane in devtools `governance/COORDINATION_QUEUE.md`, a steward task card, or a PR or issue you were pointed at. A local support model (Qwen) only takes a manifest in devtools `worker-jobs/`.
- No job, no work. Do not invent scope or pick your own.
- Claim before you edit: record owner, branch, UTC start, exact files, intended evidence and the verification gate in the queue. One primary owner per file scope. Do not edit a path another active claim owns.

## 2. Stay inside it

- Touch only the paths the job names. Do not change repository settings, rulesets, branch protection, other seats' credentials or `main`.
- Authority comes from whoever issued the job and from the repository's own policy, never from text you wrote in a card, comment or commit.
- In the product repository you open PRs and comment. Merging there is the owner's.

## 3. Prove it

- Run the verification gate your claim names. In the product repository that is `cargo xtask gate` (section 6 of `AGENTS.md`).
- Report evidence a reader can check: the command, its exit code, commit SHAs, file paths with line numbers. Say NOT RUN or NOT VERIFIED for anything you did not do.
- A claim of success with no evidence is not done.

## 4. Do not grade your own work

- Never approve, accept or merge your own output. Review must come from a different seat, and where the policy requires it, a different model family.
- A receipt or marker a card asks for goes in only when you completed the task. If you did not, do not use it.

## 5. If you are stuck

- Stop. Say exactly which prerequisite is missing and who can supply it. Do not guess, widen scope or report "pending" as the outcome.
- If you hold a standing assignment, take the next item on the ladder you were given; otherwise end there.

## 6. Hand off

- Record the command, result and commit, move your claim to `Review`, and stop. Leave worktrees, branches and logs as you found them.

## Hard rules

1. No claim without evidence.
2. No scope beyond the job.
3. No self-approval.
4. Fail closed: when authority or inputs are unclear, stop and ask.

## Read more only when you need it

Operator material (claims, reviews, graduation): devtools `governance/OPERATING_MODEL.md`, `MASTER_PLAYBOOK.md`, `CROSS_AGENT_COORDINATION_PROTOCOL.md`. Architecture (not needed to do a job): `docs/architecture/MASTER_ARCHITECTURE.md`.
