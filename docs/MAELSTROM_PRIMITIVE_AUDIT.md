# Maelstrom Primitive Audit (Audit 004)

**Status:** Working draft, unreviewed by a second model family. A classification exercise, not a verification.
**Target:** the Maelstrom workspace under AutoWizard101 (`F:\Modlists\AutoWizard101`), searched on 2026-11-18. Its live tree has uncommitted modifications, so line-level facts here can drift.
**Origin:** first drafted by the Gemini seat on 2026-11-17 (attribution inferred from the checkout identity; never committed), then checked against the tree by the Claude seat on 2026-11-18, which makes the Claude seat a contributor, not an independent reviewer. See [ATLAS.md](ATLAS.md) for the vocabulary and the stance.

Maelstrom is being rebuilt as a universal trainer rather than a Wizard101-only one, and the owner intends it to progress together with Aaroneous. This audit therefore matters most where it separates generic capability from game-specific content.

## 1. What was checked

Each named construct was searched for by exact name in `crates/` (Rust) and `ProjectMaelstrom/` (C#), outside the `.local` clone and the vendored Deimos tree. The classification (the Formula column) is the author's judgment and was not reviewed.

| Construct | Found? | Where | Formula (author) |
| :--- | :--- | :--- | :--- |
| `W101FactsStore` | Yes | `ProjectMaelstrom/Utilities/W101FactsStore.cs` (`MaximumFacts = 64`) | State |
| `W101FactQualification` | Yes | `ProjectMaelstrom/Utilities/W101FactsStore.cs` (variants: Observed, Candidate, Unknown) | Trust |
| `W101FactsMapper` | Yes | `ProjectMaelstrom/Utilities/MaelstromCli.cs` | Transition |
| `POLICY_BOUNDARY.md` | Yes | repository root; `ALLOW_LIVE_AUTOMATION=false` is recorded as the default in `AUDIT_TRAIL.md` | Constraint |
| `ExecutionPolicyManager` | Yes | `ProjectMaelstrom/GameWorkspaceForm.cs` (and others) | Constraint + Trust |
| `Arbiter` | Yes | `crates/maelstrom-core/src/lib.rs` | Trust + Constraint |
| `MacroCard` | Yes | `crates/maelstrom-core/src/lib.rs` | Capability |
| `Deck`, `SubDeck` | Yes | `crates/maelstrom-core` (`lib.rs`, `deck_encoder.rs`) | Relationship + Transition |
| `ContinuousAura` | Yes | `crates/maelstrom-core/src/lib.rs` | Perspective + State |
| `ReflexPolicy` | **No** | The reflex crate has the `PolicyController` trait and `StubReflexPolicy`, which returns `NoOp` with a fixed confidence | Capability (author) |
| `DeckRunner` | Yes | `crates/maelstrom-fallback/src/lib.rs` | Transition |
| `TakeoverReplayBuffer` | Yes | `crates/maelstrom-reflex/src/lib.rs` | Resource |
| `ScreenCapturer` | Yes | `crates/maelstrom-platform/src/lib.rs` | Relationship + State |
| `InputActuator` | Yes, elsewhere | `crates/maelstrom-fallback/src/player_controller.rs`, not `maelstrom-platform` | Transition |
| `NavigationMesh` / waypoints | **No** | No match for `NavigationMesh` in the Rust crates | Relationship (author) |
| `ManualCaptureController` | Yes, elsewhere | `ProjectMaelstrom/Utilities/ManualCaptureController.cs`, not `DevTools/` | Perspective + Trust |

Findings from the search:

- `ReflexPolicy` as "a local quantized model with latency under 5 ms" is not implemented. What exists is a stub that proposes `NoOp`. Treat that row as aspirational.
- `NavigationMesh` was not found by name. Whether navigation exists under another name was not checked.
- The arbiter's confidence threshold is a constructor argument (`Arbiter::new(threshold)`); the arbiter compares model confidence with the larger of that and the request's required confidence. No fixed 0.8 literal was found in `maelstrom-core/src/lib.rs`.
- NOT FOUND by search, so unverified: `DXGI` capture in `maelstrom-platform/src/lib.rs`, and `SendInput` or `PostMessage` in `maelstrom-fallback/src`. The implementation may live in another file or crate.
- Sixteen rows is a sample, not the whole workspace: the five Rust crates define 66 top-level public items.

## 2. Candidate observations

### 2.1 Facts, authority and actions kept apart

The code and documents support the structure as a design: a fact carries a qualification (`Observed`, `Candidate` or `Unknown`); live actuation is gated by a policy that defaults to off; actions are cards and decks. The Candidate qualification text reads "trigger only, never an action authority", which is the separation in practice. Whether every action path actually goes through the gate was not tested.

Candidate reading in Atlas terms: Facts are State, Authority is Trust plus Constraint, Actions are Transition. This matches the owner's own formulation ("facts are not authority, authority is not action") and is recorded as a working reading, not a proof.

### 2.2 Dual-loop arbiter as designed

```text
Observation frame
      |
      v
Arbiter (confidence gate)
  |                      |
  | confidence at or     | confidence below the threshold,
  | above the threshold  | or an invariant tripped
  v                      v
Reflex policy         Deterministic fallback runner
  |                      |
  +----------+-----------+
             v
Takeover replay buffer (training data)
```

`Arbiter::decide` and `TakeoverReplayBuffer` exist. Because the reflex side is a stub, the smooth path is not yet exercised by a learned policy.

## 3. Candidate couplings

1. `MacroCard` carries an execution payload (`CardPayload::InputSequence`) and a game-specific school (`AutomationSchool`) in one record. Both exist in `maelstrom-core/src/lib.rs`. Reading: the payload is a generic capability and the school is a game-specific manifestation. Separating them is the path to a game-agnostic macro engine, which is the owner's stated aim.
2. `W101Fact` uses `ObservedUtcTicks` both for ordering and for freshness checks. Code in `JobAliasRegister.cs` and `JobDeck.cs` compares it with the current time to mark facts "unknown" after two seconds. Reading: sequence ordering and wall-clock expiry are different concerns. Separating them also matters for AGENTS.md rules about ambient clocks if this logic moves into Rust.

## 4. Status

| Item | Status |
| :--- | :--- |
| 16 constructs searched by name | 14 found, 2 not found |
| Classification reviewed by a second party | No |
| Tested against a case designed to fail | No |
| Maelstrom "passes" the Atlas | Not claimed; the exercise cannot fail as run |

Next: re-run this audit on the rebuilt, game-agnostic layout, and add one case that could count against the primitives, with the failure condition written down in advance.
