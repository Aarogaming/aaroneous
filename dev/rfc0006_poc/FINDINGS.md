# RFC-0006 Proof-of-Concept: Findings

Standalone proof-of-concept for
[`docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md`](../../docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md),
built per that RFC's own gate: *"This document is design-only. No
`studio_hud` code changes until a proof-of-concept (Section 8) is built and
reviewed against it."* Nothing here touches `crates/api` or
`crates/studio_hud` - it lives entirely under `dev/rfc0006_poc/`, a
standalone research artifact, not shipping product code.

**Status: PoC complete, all five Section 8 acceptance criteria demonstrated
by an automated test. Not self-certified as "reviewed and accepted" - that
call belongs to whoever reviews this against the RFC, per its own gate.**

## Running it

```
cargo test -p rfc0006_host -p rfc0006_abi
```

`host`'s `build.rs` builds the plugin fixtures (a separate nested workspace,
see below) before the tests run, so this one command is sufficient.

## Layout

- `abi/` - shared wire-format types and pure, `#![deny(unsafe_code)]`
  encode/decode logic (`rfc0006_abi`). Real workspace member.
- `host/` - the PoC host: loads a plugin, negotiates its version, ticks it,
  decodes the result (`rfc0006_host`). Real workspace member. All `unsafe`
  is isolated to `host/src/loader.rs`, which carries `#![allow(unsafe_code)]`
  as a file-scoped (not crate-scoped) inner attribute - `host/src/lib.rs`
  itself stays `#![deny(unsafe_code)]`. Every `unsafe` block has a
  `// SAFETY:` comment, per `AGENTS.md`'s Domain-Aware Unsafe Permissions.
- `plugins/` - five tiny `cdylib` fixtures (`hello`, `bad_version`,
  `panicker`, `panicker_raw`, `liar`), one per acceptance criterion below.
  **Deliberately not a member of the root Aaroneous workspace** (see the
  root `Cargo.toml`'s `exclude` list): it's its own nested `[workspace]`,
  which (a) models the realistic deployment shape - a plugin is a genuinely
  separate, independently-compiled artifact, not part of the host's own
  build graph - and (b) keeps `cargo check --workspace`/`cargo test
  --workspace` at the product root untouched by this PoC's fixtures.

## Acceptance criteria (RFC-0006 Section 8) → evidence

| # | Criterion | Test | Fixture(s) |
|---|---|---|---|
| 1 | Load → tick(×N) → unload cleanly, no leaked handles | `host/tests/lifecycle.rs::hello_plugin_loads_ticks_and_unloads_cleanly_across_many_cycles` (20 load/unload cycles × 5 ticks each) | `plugin_hello` |
| 2 | Version-mismatched plugin refused at load, not crashed/silent | `host/tests/lifecycle.rs::version_mismatched_plugin_is_refused_at_load_not_crashed_or_silently_accepted` | `plugin_bad_version` |
| 3 | Panic inside `plugin_tick` doesn't crash/corrupt the host | `host/tests/panic_containment.rs::a_panic_inside_plugin_tick_is_caught_by_the_plugin_itself_and_reported_as_faulted` (the actual proof - runs entirely in-process). Its sibling test is a **control case, not a second proof** - see the correction below and the file's own module doc comment. | `plugin_panicker` (proof); `plugin_panicker_raw` (control) |
| 4 | A plugin lying about `command_count`/`payload_len` is truncated, not OOB-read | `host/tests/adversarial_buffer.rs` + `abi`'s own `proptest` fuzz tests + `abi::tests::decode_bounds_by_payload_len_even_when_real_capacity_and_bytes_used_are_larger` | `plugin_liar` |
| 5 | Host-side path is `deny(unsafe_code)`-clean except isolated, documented `unsafe` | Structural: `host/src/lib.rs` crate-level `deny`, only `loader.rs` opts out, every block has `// SAFETY:` | n/a |

All: `cargo test -p rfc0006_host -p rfc0006_abi` → 16 tests, 0 failures.
`cargo clippy -p rfc0006_abi -p rfc0006_host --all-targets -- -D warnings`
and the same for the `plugins` workspace: both clean. `cargo fmt -p
rfc0006_abi -p rfc0006_host -- --check`: clean.

## A correction to RFC-0006 Section 7

Section 7 currently reads: *"Plugins must compile with `panic = 'abort'`
for their `cdylib` target, **or** the host wraps every FFI call in
`std::panic::catch_unwind`... unwinding across an `extern "C"` boundary is
undefined behavior in Rust today."*

That description predates a change that's now been stable for a while:
since Rust 1.71 (RFC 2945 / the `C-unwind` ABI stabilization), a panic that
reaches a plain `extern "C"` (not `extern "C-unwind"`) function boundary
without being caught no longer causes undefined behavior. The runtime
detects it and **aborts the process safely** - confirmed directly by this
PoC's `panic_containment.rs`, which shows the literal message `thread
caused non-unwinding panic. aborting.` when `plugin_panicker_raw` (which
deliberately does not catch its own panic) is ticked.

This changes the mitigation, not just the terminology:

- **"The host wraps every FFI call in `catch_unwind`" cannot work as
  described.** By the time control would return to a `catch_unwind` at the
  *host's* call site, the process has already aborted at the plugin's own
  `extern "C"` boundary - the host's wrapper never gets a chance to run.
  `panic_containment.rs`'s `an_uncaught_panic_takes_down_whatever_process_hosts_it`
  test demonstrates exactly this - it has to run the ticking plugin in a
  disposable *child* process specifically because that child, not this test
  suite's own process, is what plays the role of "host" for the
  non-compliant plugin, and it does not survive. (An earlier version of
  this PoC mis-described this test as itself satisfying criterion 3,
  because its own process - a bystander that never loads the panicking
  plugin - survives; a review caught that this conflates "isolation
  protected an unrelated process" with "the actual host was protected,"
  which it wasn't. Corrected here and in the test's own name/doc comment.)
- **The only placement that actually works is inside the plugin itself**,
  wrapping its own logic in `catch_unwind` *before* returning across its own
  `extern "C"` boundary - exactly what `plugins/panicker/src/lib.rs` does,
  verified by
  `a_panic_inside_plugin_tick_is_caught_by_the_plugin_itself_and_reported_as_faulted`
  (5 consecutive panicking ticks, all cleanly `TickOutcome::Faulted`, no
  abort, no corruption).
- **`panic = "abort"` doesn't help either.** It would abort on the *first*
  panic, before any `catch_unwind` (host- or plugin-side) could run at all -
  worse than the default, not a fix.
- The good news: since the abort in the unmitigated case is *safe* (no
  memory corruption, just process termination), the actual risk profile is
  "one plugin's panic takes down the whole shared-process host" rather than
  "undefined behavior" - still unacceptable for a plugin host's resilience
  story, but a different, better-understood failure mode than RFC-0006
  currently states.

**Recommendation:** reword Section 7 to require plugins to internally
`catch_unwind` at their own `extern "C"` boundary (the only mechanism that
actually prevents host disruption), and correct the "UB" claim to "a safe
but disruptive whole-process abort," matching current stable Rust.

## Other scope notes

- **`payload_len` is enforced, not just read.** RFC-0006 Section 6 says the
  host must check `command_count`/`payload_len` "against the buffer
  capacity it allocated." An earlier version of this PoC only checked
  `command_count` against the real buffer length and ignored `payload_len`
  entirely, reasoning that was "strictly more conservative." A review
  correctly pointed out that's wrong when the two header fields disagree:
  `host`'s buffer is reused across ticks (never zeroed between them), so a
  plugin claiming a large `bytes_used` with a small, honest `payload_len`
  could otherwise have stale bytes from an earlier tick replayed as if they
  were current commands. `decode_commands` now bounds the parsed command
  count by both the real slice length *and* `payload_len`, whichever is
  smaller; see `abi::tests::decode_bounds_by_payload_len_even_when_real_capacity_and_bytes_used_are_larger`.
- **`plugin_handle_event` is resolved at load (part of the RFC's fixed
  symbol set, "no partial activation") but not exercised by a dedicated
  test** - Section 8's five criteria don't cover event feedback, so this
  PoC scoped it out rather than build a redundant test path. Every fixture
  still exports a valid (no-op) implementation so `load` succeeds.
- **The RFC's "three consecutive faults → automatic unload" policy (end of
  Section 7) is not implemented in this PoC's minimal `LoadedPlugin`.** It's
  a real implementation's job, not required to demonstrate any of the five
  Section 8 criteria; noted here so it isn't mistaken for an oversight.
- **Authentication (RFC-0006 Section 6, "a real signature check... gates
  loading the library at all") is explicitly out of scope**, per the RFC's
  own text ("orthogonal to ABI soundness"). This PoC loads only its own
  just-built fixtures - `LoadedPlugin::load`'s `// SAFETY:` comment says so
  directly.

## Review round: findings and fixes

A first review pass on this PoC (Codex) found six real issues, all fixed
here rather than argued with:

1. **The `panicker_raw` subprocess test was mis-described as proving
   criterion 3.** It proves the opposite for the process it actually hosts
   the plugin in. Corrected above and in `panic_containment.rs`'s naming.
2. **The plugin-fixture path lookup hardcoded `lib*.so`**, which breaks on
   this repo's Windows CI runner (Cargo emits `{name}.dll`, no `lib`
   prefix). Fixed via `std::env::consts::DLL_PREFIX`/`DLL_SUFFIX` in
   `host/tests/common/mod.rs`.
3. **`decode_commands` ignored `payload_len`** - covered above.
4. **`CommandBufferWriter::push` could truncate text mid-UTF-8-character**,
   producing bytes the decoder would then reject as invalid UTF-8 even
   though the plugin supplied valid Unicode. Fixed by backing off to the
   nearest `str::is_char_boundary` at or below `MAX_TEXT_LEN`.
5. **`TickResultRaw` didn't derive `bytemuck::Pod`/`Zeroable`**, which
   `AGENTS.md`'s "Memory Geometry & ABI Safety" rule requires for every
   boundary type crossing an FFI edge, regardless of whether this crate's
   own code happens to use a `Pod` cast (it doesn't - decoding stays manual
   byte-slice parsing, per this file's `#![deny(unsafe_code)]` design).
   Added both derives plus the `bytemuck` dependency.
6. **`decode_commands` read `wire_version` but never gated on it**, so a
   hypothetical future wire format bump could get silently misparsed with
   v1's record layout. Added an early `DecodeError::UnsupportedWireVersion`
   check before any record parsing begins.

Re-verified after all six fixes: `cargo test -p rfc0006_host -p
rfc0006_abi` (all pass), `cargo clippy --all-targets -- -D warnings` on
both the root-workspace crates and the `plugins` nested workspace (clean),
`cargo fmt -- --check` (clean).

## What this unblocks

Per RFC-0006's own closing line: *"Only after these pass does implementing
this in `crates/studio_hud/src/plugin_api.rs` become a scoped, reviewable
change instead of a research question."* All five criteria pass. The next
step is a human/reviewer sign-off against this PoC (and, ideally, a
decision on the Section 7 correction above), after which the real
`crates/api`/`crates/studio_hud` implementation becomes a normal, scoped
PR - reusing this PoC's wire format and `decode_commands` logic nearly
as-is, and its `loader.rs` as the template for the real dynamic-loading
integration (plus the still-separate authentication mechanism Section 6
defers).
