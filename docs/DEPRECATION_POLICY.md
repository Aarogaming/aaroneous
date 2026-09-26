# Deprecation Policy

**Applies to:** Aaroneous workspace  
**First deprecation wave:** v0.3.3 (M7 + M22, 2026-09-16)  
**Maintainer reference:** devtools governance/COORDINATION_QUEUE.md M23

---

## 1. Purpose

This document defines how deprecated symbols are introduced, communicated,
and removed. It covers the v0.3.3 first deprecation wave (biological/mythological
terminology aliases) and establishes the policy for all future deprecations.

---

## 2. Deprecation Wave -- v0.3.3 Aliases

All aliases below were introduced by M7 and M22. Each is a Rust type alias
with #[deprecated(since = "0.3.3", note = "...")] pointing at the canonical
replacement. The workspace compiles clean under cargo clippy --workspace -D warnings
because all internal consumers (re-export blocks in lib.rs) carry #[allow(deprecated)].

### 2.1 Type aliases in crates/adaptation_plane/src/capability_spec.rs

| Deprecated alias | Canonical replacement | Note |
|---|---|---|
| ParameterLocus | ProfileLocus | Domain-neutral terminology |
| AgentConfigProfile | AgentProfile | Simplified, idiomatic name |
| ParameterProfile | AgentProfile | Synonymous with AgentConfigProfile |
| SpecialistGenome | AgentProfile | Biological naming retired |
| ParameterGenome | AgentProfile | Biological naming retired |
| GeneticLocus | ProfileLocus | Biological naming retired |
| GeneticCategory | ProfileCategory | Biological naming retired |
| EpigeneticState | AdaptationState | Biological naming retired |
| GeneticRelationship | ProfileRelationship | Biological naming retired |
| GeneticAnalyzer | ProfileAnalyzer | Biological naming retired |

### 2.2 Type alias in core/hypervisor/src/profile_compiler.rs

| Deprecated alias | Canonical replacement |
|---|---|
| GenomeCompiler | ProfileCompiler |

### 2.3 Type aliases in core/hypervisor/src/profile_schema.rs

| Deprecated alias | Canonical replacement |
|---|---|
| HoxPermissions | NodePermissions |
| EnzymeGenetics | ModuleDefinition |
| HoxMap | NodeMap |

### 2.4 Methods in crates/paths/src/lib.rs

| Deprecated method | Canonical replacement |
|---|---|
| WorkspacePaths::hox_db() | WorkspacePaths::node_db() |
| WorkspacePaths::sovereign_hox_preset(name) | WorkspacePaths::node_preset(name) |
| WorkspacePaths::relic_hox_preset(name) | WorkspacePaths::module_preset(name) |

---

## 3. Consumer Audit (v0.3.3 baseline -- 2026-09-16)

**External downstream consumers:** none. Aaroneous is a private workspace; these
types are not published to crates.io. There is no external semver compatibility
obligation.

**Internal consumers (remaining uses of deprecated aliases):**

| File | Line range | Kind | Disposition |
|---|---|---|---|
| crates/adaptation_plane/src/capability_spec.rs | 107-124, 363-364 | Definition sites | Delete on removal |
| core/hypervisor/src/profile_compiler.rs | 303 | Definition site | Delete on removal |
| core/hypervisor/src/profile_schema.rs | 12, 30, 39 | Definition sites | Delete on removal |
| crates/paths/src/lib.rs | 278, 312, 328 | Definition sites | Delete on removal |
| crates/adaptation_plane/src/lib.rs | 45-50 | #[allow(deprecated)] re-export block | Delete on removal |
| core/hypervisor/src/lib.rs | 102-105 | #[allow(deprecated)] re-export block | Delete on removal |
| crates/adaptation_plane/src/capability_spec.rs | 590, 621 | Migration test TypeId assertions | Delete test blocks on removal |
| core/hypervisor/src/profile_schema.rs | 127, 138 | Migration test TypeId assertions | Delete test blocks on removal |
| crates/paths/src/lib.rs | 789, 799 | Migration test path assertions | Delete test blocks on removal |

**No call sites remain in production code outside definition sites and re-export glue.**
All internal production call sites were migrated during M7 and M22.

---

## 4. Removal Window

### 4.1 Policy

- Aliases deprecated in v0.3.3 are **scheduled for removal in v0.4.0**.
- Removal is gated on: (a) cargo clippy --workspace -D warnings passing with zero
  use_of_deprecated errors after removal, and (b) cargo test --workspace passing.
- Removal PR must be a standalone change -- not mixed with feature work.
- The removal PR must include deletion of: (a) the type alias definition, (b) the
  #[allow(deprecated)] re-export block entry, (c) the migration test assertion.

### 4.2 Removal Readiness Checklist

Executed in v0.4.0:

- [x] git grep for all deprecated alias names across crates/ core/ dev/ produces only
      definition-site and test-assertion matches (no production call sites).
- [x] All definition sites, re-export entries, and test assertions are deleted.
- [x] cargo test --workspace passes (no missing-symbol errors).
- [x] cargo clippy --workspace -- -D warnings passes (no lingering allow(deprecated) suppressions).
- [x] cargo run -p ast_auditor -- audit core/ crates/ dev/ reports 0 violations.

### 4.3 Migration Guide

Replace deprecated aliases at each call site:

For adaptation plane aliases:
  ParameterLocus        -> ProfileLocus
  AgentConfigProfile    -> AgentProfile
  ParameterProfile      -> AgentProfile
  SpecialistGenome      -> AgentProfile
  ParameterGenome       -> AgentProfile
  GeneticLocus          -> ProfileLocus
  GeneticCategory       -> ProfileCategory
  EpigeneticState       -> AdaptationState
  GeneticRelationship   -> ProfileRelationship
  GeneticAnalyzer       -> ProfileAnalyzer

For hypervisor aliases:
  GenomeCompiler        -> ProfileCompiler
  HoxPermissions        -> NodePermissions
  EnzymeGenetics        -> ModuleDefinition
  HoxMap                -> NodeMap

For paths methods:
  .hox_db()                        -> .node_db()
  .sovereign_hox_preset(name)      -> .node_preset(name)
  .relic_hox_preset(name)          -> .module_preset(name)

---

## 5. Policy for Future Deprecations

1. Deprecated symbols must carry #[deprecated(since = "X.Y.Z", note = "Use FOO instead")].
2. A migration test must be added in the same PR proving TypeId or runtime equivalence.
3. All internal call sites must be migrated in the same PR as the deprecation.
4. External call sites (if any exist in future public releases) require a two-release
   deprecation window before removal.
5. Every deprecation must be recorded in this file under a dated wave heading.
6. Removal is a separate PR, always after the version bump that introduced since.

---

This document is maintained by the Aaroneous governance process. Update section 2 and 3
when new aliases are added, and update the 4.2 checklist when the v0.4.0 removal is executed.
