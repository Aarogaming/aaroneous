# RFC-0006: Plugin Lifecycle & Stable UI Cartridge ABI

- **RFC Number:** 0006
- **Title:** Sound Dynamic Plugin Loading for `studio_hud` via a Stable Command-Buffer ABI
- **Status:** Draft / Proposed
- **Domain:** Presentation Layer, FFI/ABI Safety, Plugin Extensibility
- **Target Crates:** crates/api, crates/studio_hud
- **Depends On:** none (design-only; no code lands until a proof-of-concept is accepted)
- **Supersedes:** the dynamic-loading mechanism removed from `crates/studio_hud/src/plugin_api.rs` (see History below)

---

## 1. Executive Summary

`crates/studio_hud/src/plugin_api.rs` used to offer `load_dynamic_plugin`/`unload_dynamic_plugin`, hot-loading a `.dll`/`.so` and reconstructing a `Box<dyn UiCartridge>` from a raw pointer returned across an `extern "C"` boundary. A security revalidation found two independent problems and removed the mechanism rather than patch around it:

1. **SEC-01 — fake signature check.** The gate before a load hashed the file and rejected only a hash whose first two bytes happened to be zero. That passes almost any file, malicious or benign.
2. **SEC-03 — unsound ABI.** `UiCartridge::render` takes `&mut egui::Ui`, a complex, non-`repr(C)` third-party type with no stable binary layout across compiler versions or even different builds of the same `egui` version. Passing it across a real DLL boundary is undefined behavior regardless of how well-authenticated the plugin is — **authentication and ABI safety are orthogonal concerns; fixing one does not fix the other.**

`TODO.md`'s roadmap still lists "dynamic plugin swapping" (P4: Adaptive Runtime Engine) and "Hot-reload ABI plugins" (Phase 5: Adaptive Runtime & Live Patching, v0.8.0) as a real, planned capability — this isn't being abandoned, it's being redesigned to actually be sound. This RFC proposes that redesign: instead of handing a plugin a live reference to host-owned, non-FFI-safe state, **the plugin writes draw commands into a `#[repr(C)]` command buffer, and the host replays that buffer against its own `egui::Ui`.** Neither side ever needs a stable layout for `egui` types; only a small, hand-designed wire format needs to be stable, and Rust can guarantee that.

This document is design-only. No `studio_hud` code changes until a proof-of-concept (Section 8) is built and reviewed against it.

---

## 2. Problem Statement

| Requirement | Fat-pointer trait object (removed) | Command-buffer ABI (proposed) |
| :--- | :--- | :--- |
| Cross-DLL memory safety | ✗ — `*mut dyn UiCartridge` is an unspecified-layout fat pointer across compilation units | ✓ — a plain byte buffer with a fixed, versioned header |
| `egui::Ui` never crosses the boundary | ✗ — passed by `&mut` reference directly | ✓ — plugin never sees a live `Ui`; only opaque draw opcodes |
| Authentication | ✗ (SEC-01: fake check) | Addressed independently (Section 6) — orthogonal to ABI soundness |
| Plugin panics | Undefined — unwind across FFI is UB | Must abort at the boundary (Section 7) |
| Forward/backward compatibility | None — any struct-layout change breaks every existing plugin silently | Explicit version negotiation (Section 7) |

The redesign must solve **all** of these, not just the one that happened to be found first.

---

## 3. Design Goals / Non-Goals

**Goals:**
- A plugin compiled against one `studio_hud`/`api` release keeps working, or fails to load with a clear reason, against a later compatible release — never silently corrupts memory.
- The host can bound how much work a single plugin `tick` does (draw-command count, buffer size) before replay, so one plugin cannot stall the frame loop indefinitely.
- A plugin panic is contained: it cannot unwind into host code.
- The command set is small and hand-maintained, not machine-generated from `egui`'s full API — `egui` itself has no ABI stability contract, so mirroring it 1:1 would just relocate the unsoundness one layer down.

**Non-goals (explicitly out of scope for this RFC):**
- Process-level plugin sandboxing (a separate process, IPC-based design) — this RFC is in-process-DLL only, matching what `TODO.md` describes for Phase 5.
- Full `egui` widget parity on day one — Section 5 scopes a minimal initial command set.
- The generational rollback journal `TODO.md`'s Phase 5 also lists — that's a hot-reload state-management concern layered on top of a working plugin ABI, not a prerequisite for one.

---

## 4. Lifecycle

```
Load ──► Version Negotiate ──► [ Tick ──► Replay ──► Event Feedback ]* ──► Unload
```

1. **Load.** Host `dlopen`s the plugin library via `libloading` and resolves a small, fixed set of `extern "C"` symbols by name. A plugin missing any required symbol fails to load — no partial activation.
2. **Version Negotiate.** The plugin exports `plugin_abi_version() -> u32`. The host checks it against a `[MIN_SUPPORTED, MAX_SUPPORTED]` range it owns; a mismatch is a normal, logged refusal to load, not a crash. This is where a meaningful (not the removed fake) compatibility gate lives.
3. **Tick.** Once per frame the host calls `plugin_tick(buf: *mut CommandBuffer, capacity: usize) -> TickResult`, passing a buffer it owns. The plugin writes draw commands into it and returns how many bytes it used, or an overflow signal if `capacity` was too small (never a partial/torn write past `capacity`).
4. **Replay.** The host validates the returned buffer (Section 6) and replays each command against its live `egui::Ui`. This is the only place `egui` API calls happen — the plugin never touches `egui` types directly.
5. **Event Feedback.** Widgets the host draws on the plugin's behalf (e.g., a button) get a stable `u64` id the plugin chose when it emitted that command. After replay, the host calls `plugin_handle_event(events: *const HostEvent, count: usize)` with which ids fired this frame, so the plugin can react next tick — a one-frame round-trip, not a live callback into plugin code from inside `egui`'s own call stack.
6. **Unload.** Host calls `plugin_shutdown()` for the plugin to release its own resources, then drops the `libloading::Library`, unmapping the DLL.

---

## 5. Command Buffer Wire Format (initial sketch)

A plain, versioned, length-prefixed buffer — no pointers, no trait objects, nothing that isn't `Copy` and `#[repr(C)]`:

```rust
#[repr(C)]
pub struct CommandBufferHeader {
    pub wire_version: u32,   // format version, independent of plugin_abi_version()
    pub command_count: u32,
    pub payload_len: u32,    // bytes following the header
    pub _reserved: u32,      // zero; future flags
}

// Each command: a fixed-size tagged record. No variable-length payloads in v1 —
// text is passed as a fixed-capacity byte array + explicit length, not a pointer,
// so every command is Copy and the buffer can be validated by a single bounds
// check over `command_count * size_of::<Command>()`.
#[repr(C)]
pub struct Command {
    pub op: CommandOp,       // #[repr(u32)] enum — see below
    pub widget_id: u64,      // stable id the plugin assigns; 0 = non-interactive
    pub text: [u8; 64],      // UTF-8, NUL-padded; longer text is a v2 problem
    pub text_len: u16,
    pub x: f32,
    pub y: f32,
    pub color_rgba: [u8; 4],
}

#[repr(u32)]
pub enum CommandOp {
    Label = 0,
    Button = 1,
    BeginHorizontal = 2,
    BeginVertical = 3,
    End = 4,
    Spacing = 5,
}
```

This is intentionally minimal — enough for a real "Hello World" plugin (Section 8), not full `egui` coverage. `CommandOp` is `#[repr(u32)]` with explicit discriminants so adding a variant is backward-compatible for old readers (an old host sees an unknown opcode and skips it, logged once, rather than misinterpreting the record). Text is fixed-capacity by design: a `*const u8` string pointer would reintroduce a cross-allocator, cross-DLL lifetime problem identical in kind to the one this RFC exists to remove.

---

## 6. Safety & Validation

- **Bounds checking.** The host never trusts `command_count`/`payload_len` from the plugin without checking them against the buffer capacity it allocated. A plugin that lies about its own output gets its buffer truncated to what's actually valid, not a host-side out-of-bounds read.
- **No raw pointers cross the boundary** except the command-buffer pointer + capacity itself — the same shape as any C ABI accepting a caller-provided output buffer, a well-understood pattern.
- **Authentication stays a separate mechanism.** A real signature check (e.g., Ed25519 against an injected trusted-key list, failing closed with no keys configured) gates *loading* the library at all, independent of the ABI. This RFC does not re-specify that mechanism in detail — it only requires that whatever replaces SEC-01's fake check lives here, at load time, not folded into the command-buffer format.

---

## 7. Versioning & Panic Containment

- `plugin_abi_version()` and `CommandBufferHeader.wire_version` are two separate numbers: the first gates whether the plugin loads at all, the second lets the *format* evolve independently of the symbol contract (e.g., a new `Command` field could bump the wire version while `plugin_abi_version()` stays the same, if the host can still read the old layout via padding rules agreed here).
- Plugins **must** compile with `panic = "abort"` for their `cdylib` target, or the host wraps every FFI call in `std::panic::catch_unwind` and treats a caught panic as `TickResult::PluginFaulted` — unwinding across an `extern "C"` boundary is undefined behavior in Rust today, and this RFC does not accept a design that relies on it not happening.
- A plugin that returns `TickResult::PluginFaulted` (or panics, if caught) three times in a row is unloaded automatically — a bounded blast radius, not a silent hang.

---

## 8. Proof-of-Concept Acceptance Criteria

Before any of this lands as real `studio_hud` code, a standalone PoC must demonstrate, and a review must confirm:

1. A minimal plugin `cdylib` (label + one clickable button) loads, ticks for N frames, and unloads cleanly with `libloading`, with no leaked handles (checked via repeated load/unload cycles under a leak-detecting allocator or the OS's own handle count).
2. A version-mismatched plugin (`plugin_abi_version()` outside the host's supported range) is refused at load with a clear log line — not a crash, not a silent no-op.
3. A plugin that panics inside `plugin_tick` does not crash or corrupt the host process — verified with `catch_unwind` at the boundary and a `panic = "abort"` `cdylib` test build.
4. A plugin that lies about `command_count` (claims more commands than fit in `payload_len`) is truncated, not read out-of-bounds — verified with a fuzzed/adversarial test plugin, not just a well-behaved one.
5. The whole command-buffer path is `#![deny(unsafe_code)]`-clean on the host side wherever possible; any remaining `unsafe` (the raw buffer read/write itself) is isolated to the smallest function that needs it and documented per AGENTS.md's Domain-Aware Unsafe Permissions.

Only after these pass does implementing this in `crates/studio_hud/src/plugin_api.rs` become a scoped, reviewable change instead of a research question.

---

## 9. Open Questions

- Full `egui` command coverage: this RFC scopes a minimal v1 set (Section 5). A follow-up should define the process for adding commands (who reviews new `CommandOp` variants, how wire-version bumps are tested for backward compatibility) rather than leaving it ad hoc.
- Should the command buffer be double-buffered (plugin writes to buffer N+1 while host replays buffer N) for latency, or is single-buffered synchronous tick-then-replay sufficient given `studio_hud`'s frame budget? Not decided here — needs a measurement, not a guess (per this repo's benchmarking policy: no premature claims without a reproducible benchmark).
- Interaction with the generational rollback journal `TODO.md`'s Phase 5 describes (hot-reloading a *new* plugin binary while preserving its logical state) is a separate, later RFC — this one only covers a plugin being loaded, ticking, and unloading, not being replaced in place.

---

## References

- `crates/studio_hud/src/plugin_api.rs` — current module doc comment records the SEC-01/SEC-03 findings and the decision to remove dynamic loading rather than patch it (queue item C2, private operations workspace).
- `TODO.md` — "P4: Adaptive Runtime Engine" and "Phase 5: Adaptive Runtime & Live Patching" list the roadmapped capability this RFC is designing toward.
- Queue item M4 (private operations workspace) — the removal-versus-repair decision that concluded this module should stay as sound, minimal, in-process-only infrastructure pending this RFC.
