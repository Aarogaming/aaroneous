# Decision review: chrono (RUSTSEC-2026-0543)

**Status:** Options review by the Claude seat, 2026-11-18. On the owner's instruction ("perform recommended") the recommendation in section 6 is being carried out: steps 1 and 2 are the PR that carries this file, step 3 is the lane below, step 4 (the ratchet) is tracked in section 8. Nothing here is a migration; chrono stays in the lockfile until the lane in section 8 lands.
**Repo state examined:** `main` at `cc794cb3`.

## 1. What is actually failing

- `cargo deny --all-features check` fails with one error: `error[unmaintained]: chrono is unmaintained` (RUSTSEC-2026-0543). `cargo xtask gate` stops at gate 3.6, so inside `Check & Test` the tests, audit (3.7) and semver (3.8) never run. Until this is resolved no product PR can get a green `Check & Test`.
- The advisory is informational ("unmaintained"), dated 2026-09-14, with no vulnerability and no patched version.
- **Correction to my earlier note:** I said both `deny.toml` and `.cargo/audit.toml` need an entry. Only `deny.toml` is needed to pass CI. `cargo audit` treats unmaintained crates as warnings: on the `quinn-udp` branch (#154) it exits 0 with the chrono warning printed. `audit.toml` would only be a mirror for the sync convention in both files' headers.

## 2. Upstream facts (GitHub issue chronotope/chrono#1768)

- The maintainer announced a wind-down of `chrono` and `chrono-tz` in January 2026 ("soft-deprecated") and filed the advisory himself. On 2026-09-15 he said they are "unlikely to merge any more changes". On 2026-11-17 he said the signal to move off chrono was given in January.
- No successor took over; the issue closed as completed on 2026-11-17. Last release 0.4.45 on 2026-06-04; last push 2026-09-07. The repository is not archived.
- The recommended alternatives are `jiff` and `time`. `jiff` is still 0.2.x in our lock (0.2.35); the maintainer had wanted to wait for jiff 1.0 to avoid two ecosystem migrations.

So "wait for a fix" is not a real option: there is no fix coming.

## 3. How much we depend on it (Observed)

- Direct dependents: 8 crates (`hypervisor`, `adaptation_engine`, `adaptation_plane`, `capabilities`, `coordinator`, `omni`, `orchestrator`, `studio_hud`). The only other path in the lock is `rusqlite`'s `chrono` feature enabled in `core/hypervisor/Cargo.toml`.
- **That `rusqlite` feature is free to drop.** The two hypervisor files that use `rusqlite` never mention chrono. I removed the feature in a scratch worktree: `cargo check -p hypervisor -p coordinator --all-targets` exit 0, and `rusqlite` no longer depends on `chrono`. (Hypervisor tests NOT RUN; reverted.)
- Footprint: 44 files use `chrono::`. API in use is small: `Utc::now` (about 92 call sites in 7 crates, 31 in `coordinator`, 20 in `hypervisor`), `DateTime<Utc>` in serde-derived structs (hypervisor, adaptation_plane and others), `to_rfc3339` (11), `parse_from_rfc3339` (4, all in tests), `timestamp_millis` (9), one `Local::now`, one `Duration::seconds`.
- Profiles: `hypervisor` is `kernel`; `coordinator`, `adaptation_*`, `capabilities`, `omni`, `orchestrator` are `control`; `studio_hud` is `presentation`.

## 4. The compatibility trap that makes migration riskier than it looks (Observed in source)

`chrono` produces two different RFC 3339 spellings from the same type:
- `to_rfc3339()` writes the offset as `+00:00` (`use_z = false`, chrono 0.4.45 `datetime/mod.rs`). `cluster_db` stores these strings in TEXT columns and runs `ORDER BY updated_at / created_at / timestamp DESC` on them.
- `serde` serialization writes `Z` (`use_z = true`). Several structs persist or send JSON this way (for example `artifact_pruning` writes persona and genome JSON with `serde_json`).

Consequences for any replacement: (a) it must reproduce both spellings exactly or old data stops matching; (b) mixing `Z` into columns that already hold `+00:00` rows changes the within-second sort order, because `Z` (0x5A) sorts after the fractional-second `.` (0x2E) while `+` (0x2B) sorts before it; (c) `time` and `jiff` do not serialize with `serde` in RFC 3339 form by default, so each field needs an explicit format attribute or feature. `cluster_db::ClockSource` is also a public type alias over `chrono::DateTime<Utc>`.

## 5. Options

| # | Option | Unblocks CI | Cost | Main risk |
| :--- | :--- | :--- | :--- | :--- |
| 1 | **Documented ignore** of RUSTSEC-2026-0543 in `deny.toml` (optionally mirrored in `audit.toml`) | Immediately | One file, a few lines, following the existing pattern (`bincode`, `cgmath`, `paste` unmaintained entries) | Becomes permanent if no exit is recorded. `cargo-deny` 0.21 adds ignore expiry, but local runs use 0.20.2 and I did not verify that 0.20.2 accepts the new keys, so use a dated comment and a tracking lane rather than expiry fields. |
| 2 | **Migrate to `time` 0.3** (already locked via `async-nats` and `tracing-appender`) | Only when finished | 8 crates, 44 files, about 92 clock sites, serde and SQL-format work | Section 4. Touches the `kernel` crate `hypervisor`. Needs dependency-admission scoring (AGENTS section 4) for direct use. |
| 3 | **Migrate to `jiff`** (the advisory's first suggestion) | Only when finished | Same scope as 2 | Pre-1.0 (0.2.35), so a likely second migration at 1.0; default features read the system time zone (ambient authority), so admission would need `default-features = false`. |
| 4 | **Project-owned timestamp type** behind an injected clock, library chosen later | Only when finished | Largest; touches the same files plus contracts | Most design work. Best fit with the architecture: AGENTS section 2 bans ambient clocks in library code, and `Utc::now()` is exactly that (about 92 sites). I found no auditor rule that catches it (the only `now()` hit in `cratify_core` is its own `SystemTime::now`). |
| 5 | Leave it red | No | None | Product CI cannot pass. |
| 6 | Loosen the policy globally (`unmaintained = "none"`, `"workspace"` or `"transitive"`) | Partly or wholly | One line | Hides every future unmaintained crate. `"transitive"` would not cover chrono at all, because it is a direct dependency. Not recommended. |

## 6. Recommendation (the owner decides)

Do 1 now, plan 4 with a library choice deferred, and ratchet in between:

1. **Now:** option 1 in `deny.toml`, with an evidence block in the file's existing style (informational, no vulnerability, wound down in January 2026, last release 2026-06-04, no successor) and a named exit. Mirror it in `audit.toml` only to honour the header convention.
2. **Free cleanup in the same or a nearby PR:** drop `"chrono"` from `rusqlite`'s features in `core/hypervisor/Cargo.toml` (compiles; tests not run).
3. **Plan, do not start:** a lane that introduces a thin project-owned timestamp type with golden tests for both exact spellings (`+00:00` for stored text, `Z` for serde), then moves the 8 crates over one at a time. Pick `time` or `jiff` behind that type later; this avoids a second migration at jiff 1.0.
4. **Ratchet:** an `ast_auditor` rule with a baseline (8 crates, about 92 clock sites) that may only shrink, matching the existing ratchet doctrine. Only a hard block once a crate reaches zero.

## 7. Limits

- Call-site counts come from `git grep` regexes, not the compiler.
- I did not run the hypervisor or `coordinator` test suites in the no-`rusqlite`-chrono experiment, and did not price option 4 beyond counting files.
- I did not verify how a `time` or `jiff` replacement would serialize each `DateTime<Utc>` field; section 4 states the hazard, not a measured breakage.
- Dependency-admission scores (AGENTS section 4) are not computed here; they belong in the PR that adds a direct dependency.

## 8. Lane: leave chrono (planned, not started)

Exit condition for the `deny.toml` / `audit.toml` entries: `chrono` no longer appears in `Cargo.lock`. Until then the entries stay and each carries this file as its evidence.

Phases, each its own PR with the full `cargo xtask gate`:

1. **Golden tests first (no behavior change).** Pin today's exact output: `to_rfc3339()` gives `+00:00`, serde JSON gives `Z`, `timestamp_millis` values, and the `cluster_db` `ORDER BY` result for rows that differ only inside one second. These tests must stay green through every later phase.
2. **Project-owned timestamp type.** A thin type in a `kernel`-compatible crate (candidate: `core-contracts`, which needs `no_std`-friendly deps, so the type must not assume a clock) with explicit `to_stored_text()` (`+00:00`) and serde (`Z`) forms, and a clock injected as a tick or trait input instead of `Utc::now()` (AGENTS.md section 2). No chrono types in its public API.
3. **Move crates one at a time**, lowest-risk first (`studio_hud`, `omni`, `capabilities`, `orchestrator`, `adaptation_*`), then `coordinator` (31 clock sites, owns `cluster_db`) and `hypervisor` (`kernel` profile) last. Drop `cluster_db::ClockSource`'s chrono alias in the same step as `cluster_db`.
4. **Choose the backing library behind the type** (`time` is already in the lock; `jiff` is pre-1.0). Run the dependency-admission scoring (AGENTS.md section 4: alloc, ambient, abi, safety) in that PR, then remove `chrono` from the last manifest, delete both advisory entries, and delete this lane section.

### Ratchet (recommendation step 4)

Count `chrono` as a direct dependency per package and `Utc::now` / `Local::now` call sites per package, with a baseline that may only shrink, using the project's existing ratchet pattern. As of `cc794cb3`: 8 packages with a direct chrono dependency, about 92 clock call sites. It is not part of this change: the repo's own doctrine lands new auditor rules as warnings with a per-crate baseline, and a new gate has to be mirrored in `xtask/src/gate.rs`, `ci.yml` and the drift tests, which deserves its own reviewed PR.
