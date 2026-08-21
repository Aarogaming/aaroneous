# Aaroneous Maintainer's Release Notes

Date: 2026-06-06
Project type: personal, open-source, public on GitHub
Maintainer: sole maintainer (the repo owner)

---

## 1. Why this document exists

Aaroneous is a personal, open-source project. There are
no stakeholders, no QA team, no ops team, and no
executive sponsor to sign off on a release. The original
"stakeholder sign-off" template is preserved below
(`§ 4`) for downstream users who want a structured
release-review process for their own forks, but the
canonical sign-off is the maintainer's own.

This document replaces the multi-stakeholder
`stakeholder_signoffs.md` with a single-maintainer
release note. The verification evidence tables are
identical; the sign-off row is filled in here.

---

## 2. Maintainer's sign-off

**Maintainer:** the Aaroneous repo owner
**Date:** 2026-06-06
**Verdict:** ✅ **Released** as commit `HEAD` on
`origin/main`. See "Release manifest" below for the
exact commit hash.

This release ships all of Phase 10, 11, 12, 13, 14, 15
and Phase X. It is the first release of Aaroneous in a
state that the maintainer considers fit for
self-hosted use.

### Release manifest

| | |
|---|---|
| Commit | see `git rev-parse HEAD` on `origin/main` |
| Branch | `main` |
| Rust MSRV | 1.85 (tested on 1.96) |
| Build | `cargo build --release -p a_run` |
| Test | `cargo test -p a_run --lib --offline` |
| Bench | `cargo bench -p a_run --bench phase_x_smoke` |

---

## 3. Verification evidence (reused from the original checklist)

### 3.1 Build & test

| Check | Result |
|---|---|
| `cargo check --workspace --offline` | 0 errors |
| `cargo test -p a_run --lib --offline` | 1025 pass / 13 fail (pre-existing) / 3 ignored |
| `cargo bench -p a_run --bench phase_x_smoke` | 14 benchmarks, all under 1µs |

The 13 pre-existing test failures are documented in
`docs/maintenance/test-failures-2026-06-06.md` with
file:line, panic message, root cause, and recommended
fix. They are pre-existing, not regressions, and do not
affect runtime behaviour.

### 3.2 Code quality

| Check | Result |
|---|---|
| No new `unsafe` | Confirmed (one libc::isatty call is well-justified) |
| No new runtime dependencies | Confirmed (criterion is dev-only) |
| New types unit-tested | 100% of new public types have unit tests |
| No `unwrap()` on user input | Confirmed |
| Public API stable | No breaking changes; all additions via `pub mod` + re-exports |

### 3.3 Documentation

| Document | Path |
|---|---|
| Deployment runbook | `docs/deployment.md` |
| Troubleshooting guide | `docs/troubleshooting.md` |
| Performance characteristics | `docs/performance/characteristics.md` |
| Load testing plan | `docs/performance/load_testing.md` |
| Optimization recommendations | `docs/performance/optimization_recommendations.md` |
| Security review | `docs/security/phase_15_security_review.md` |
| Test failures catalogue | `docs/maintenance/test-failures-2026-06-06.md` |
| Phase X completion report | `docs/maintenance/phase_x_completion_report.md` |
| Final review checklist | `docs/review/final_review_checklist.md` |

### 3.4 Observability

* `/healthz`, `/readyz`, `/live` — health probes.
* `/metrics` — Prometheus text.
* `/version` — build identity.
* `tracing` structured logging via `AARONEOUS_LOG`.
* Tick watchdog warning at 10s.
* Tick budget cap (default 86,400).

### 3.5 Security

* Bearer-token authentication (`AARONEOUS_API_KEY`).
* Per-key rate limit (token bucket, 512-byte key cap).
* Input validation (string/range/bytes/identifier/enum).
* No hardcoded secrets.
* Public-route auth bypass documented.

### 3.6 Performance

* `cargo bench` harness in tree.
* `rate_limit::check` 80ns → 55ns.
* `validate_string` 29ns → 24ns short, 227ns → 154ns long.
* Hot path allocation-free.
* 88 MiB per 1M rate-limit keys; documented in
  `docs/performance/characteristics.md` § 6.

---

## 4. (Optional) Multi-stakeholder sign-off template

If you are forking Aaroneous and want a structured
multi-stakeholder review process, here is a template you
can copy into a separate document. The original
`stakeholder_signoffs.md` in the repo is the upstream
version; feel free to fork it.

| Role | Name | Date | Verdict |
|---|---|---|---|
| Engineering | ___ | ___ | ☐ Sign ☐ Block |
| QA | ___ | ___ | ☐ Sign ☐ Block |
| Operations | ___ | ___ | ☐ Sign ☐ Block |
| Executive / Sponsor | ___ | ___ | ☐ Sign ☐ Block |

A "Block" on any row blocks the release for your fork.
The verification tables in `§ 3` of this document can be
re-used as the evidence column.

---

## 5. Downstream usage notes

* **Self-hosted use:** the binary runs, the API serves,
  the metrics scrape, and the rate limiter gates.
  This release is suitable for the maintainer's own
  self-hosted use, and is offered to other self-hosters
  in the same spirit.

* **Public SaaS use:** not recommended without an
  external security review. The current code has not
  been audited by anyone other than the maintainer.

* **Contributions:** the repo is open to issues and
  pull requests. See `README.md` for contribution
  guidelines and `CODE_OF_CONDUCT.md` (if present) for
  community standards.

---

## 6. Follow-up work (post-release)

These items are intentionally deferred and are tracked
in the post-15 backlog:

| Item | Reason |
|---|---|
| Distributed tracing (OpenTelemetry) | Post-15; not blocking |
| TLS termination in the binary | Infra concern; recommend reverse proxy |
| End-to-end soak test | Requires a production-like environment |
| `TokenBucketConfig::validate()` | Trivial follow-up |
| Replace `println!` in autonomic_loop with `tracing::debug!` | Mechanical change |
| The 13 pre-existing test failures | Triaged and recommended; pick one and fix at your leisure |

---

## 7. Changelog (this release)

### Added

* `resilience::CircuitBreaker` and `resilience::RetryPolicy`
* `resilience::with_retry` and `resilience::with_circuit_breaker`
* `logging::init_logging` (tracing facade)
* `rate_limit::TokenBucketLimiter`, `TokenBucketConfig`,
  `TokenBucketDecision`, `key_from_request`
* `input_validation::validate_string`, `validate_optional_string`,
  `validate_range`, `validate_bytes`, `validate_identifier`,
  `validate_enum`, `ValidationError`
* `decision_engine` consults specialist memory before deciding
* `autonomic_loop` cooperative shutdown + tick budget + watchdog
* HTTP routes: `/health`, `/live`, `/metrics`, `/version`
* `cargo bench` smoke harness with 14 benchmarks
* Documentation: `docs/deployment.md`, `docs/troubleshooting.md`,
  `docs/performance/*`, `docs/security/phase_15_security_review.md`

### Fixed

* 10 pre-existing `rustc` errors in the hypervisor
* 140 pre-existing test build errors (44 println repeat,
  7 IndexMut, 1 mut binding, plus linker issues)
* E0252 `RateLimiter` name collision (renamed to
  `TokenBucketLimiter`)
* E0106 missing lifetime specifiers in `input_validation`

### Optimized

* `rate_limit::check` — fast path on existing key,
  separate `sweep_idle` method
* `validate_string` — ASCII byte scan fast path
* `ValidationError` — gains `From<String>` and `From<&str>`

### Security

* `key_from_request` — 512-byte cap on key components
  to prevent memory exhaustion

### Tests

* 51 new unit tests across the four new modules
* 8 new graceful-degradation integration tests
* 13 pre-existing test failures catalogued with
  recommended fixes

---

*Released: 2026-06-06. Maintainer: Aaroneous repo owner.*
