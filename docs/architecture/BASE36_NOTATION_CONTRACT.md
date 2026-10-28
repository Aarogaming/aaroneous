# Aaroneous Base36 Version & Notation Contract (`v1`)

> **STATUS**: CANONICAL SPECIFICATION & MACHINE CONTRACT  
> **APPLIES TO**: Component build sheets, virtual branch lineage stamps, flight records, and lifecycle notation  
> **IMPLEMENTATION**: `crates/core-contracts/src/base36.rs`, `xtask/src/base36.rs`  
> **REVISION**: v1 (Lane C88 / Packet P05)  

---

## 1. Executive Summary & Design Invariants

Aaroneous uses a compact, deterministic Base36 notation for component lifecycle tracking, build-sheet identifiers, virtual branch lineage stamps, and flight recorder telemetry.

The Base36 notation contract eliminates ambient representation ambiguity and guarantees:
1. **Canonical Lowercase Alphabet**: Exactly the 36 characters `[0-9a-z]`. Uppercase characters `[A-Z]` are strictly non-canonical and rejected on validation and decoding.
2. **Deterministic Big-Endian Weighting**: High-order digits precede low-order digits.
3. **Fixed-Width Lifecycle Geometry**: Canonical lifecycle stamps (`Base36Lineage`) have fixed width $N = 24$ ASCII characters (192 bits), left-padded with `'0'`.
4. **Isomorphic Monotonic Ordering**: Because ASCII character values (`'0' < '9' < 'a' < 'z'`) preserve numeric digit values ($0 < 9 < 10 < 35$), lexicographical comparison of equal-width zero-padded stamps strictly matches numerical magnitude comparison:
   $$\text{stamp}_A < \text{stamp}_B \iff \text{val}_A < \text{val}_B$$
5. **Reserved Sentinels**:
   - **Zero / Uninitialized Sentinel**: All `'0'` characters (`"000000000000000000000000"`).
   - **Max / Tombstone Sentinel**: All `'z'` characters (`"zzzzzzzzzzzzzzzzzzzzzzzz"`).
   - **Reserved System Prefixes**: Prefixes beginning with `"0000"` are reserved for internal microkernel slot identifiers.
6. **Zero Ambient Authority**: Pure `#![no_std]` state transformations without heap allocation or ambient system calls.

---

## 2. Alphabet & Canonical Encoding

### 2.1 Symbol Set

The canonical alphabet comprises 36 ASCII characters:
```text
Index:   0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35
Symbol:  0 1 2 3 4 5 6 7 8 9  a  b  c  d  e  f  g  h  i  j  k  l  m  n  o  p  q  r  s  t  u  v  w  x  y  z
```

### 2.2 Case Rules
- Input containing uppercase characters (`A-Z`) is rejected with `Base36Error::InvalidChar`.
- Encoders always emit lowercase symbols.
- Hexadecimal strings or raw byte arrays are decoded to raw bytes before Base36 encoding; Base36 is *never* treated as a loose hex alias.

### 2.3 Leading Zeroes
- For arbitrary-length byte arrays, leading `0x00` bytes in the binary representation are preserved as leading `'0'` characters in the Base36 output.
- For fixed-width stamps (width 24), unused high-order digits are padded on the left with `'0'`.

---

## 3. Mathematical Geometry & Capacities

- **Fixed-width 24-character capacity**:
  $$36^{24} \approx 2.2452 \times 10^{37} \text{ values}$$
  Raw byte capacity:
  $$256^{15} = 2^{120} \approx 1.3292 \times 10^{36} \text{ values} < 36^{24}$$
  Any 15-byte digest (e.g. truncated SHA-256) is guaranteed to fit losslessly within 24 Base36 characters without overflow.
- **Flight Recorder Integration**:
  `FlightFileHeaderPod::build_lineage` is defined as `[u8; 24]`, representing a `Base36Lineage` stamp.

---

## 4. Rust Value Types & APIs

### 4.1 `core-contracts` (`#![no_std]`, `profile = "kernel"`)

```rust
use core_contracts::base36::{Base36Lineage, Base36Error, LINEAGE_STAMP_WIDTH};

// Parse from validated 24-character string slice
let stamp = Base36Lineage::from_str_exact("0123456789abcdefghijklmn")?;

// Encode from 15-byte digest on stack without allocation
let stamp = Base36Lineage::encode_from_digest(&digest_15_bytes)?;

// Inspect sentinels
assert!(Base36Lineage::ZERO.is_zero());
assert!(Base36Lineage::MAX.is_max());

// Ordering
assert!(stamp1 < stamp2);
```

### 4.2 `xtask` (`tooling`)

The `xtask` CLI provides developer and CI verification tooling:
```bash
# Validate that a string conforms to canonical Base36
cargo xtask base36 validate 0iw6eogeftut0vs7xt2bldy8 --width 24

# Encode text or hex bytes
cargo xtask base36 encode "hello world"
cargo xtask base36 encode 0001020304050607 --hex

# Decode Base36 string
cargo xtask base36 decode fuvrsivvnfrbjwajo

# Compute 24-character lineage stamp from commit message / branch digest
cargo xtask base36 lineage "feat(core): initial microkernel"
```

---

## 5. Forward-Compatible Extension Rules

1. **Schema Revision**: The current schema revision is `"v1"`.
2. **Length Extensions**: Future fixed-width stamps must be defined with explicit, documented width constants (e.g., `WIDTH = 32`). Unbounded variable-width strings must declare maximum length constraints $\le 8192$ characters.
3. **Prefix Reservations**: Prefixes starting with `"0000"` through `"000f"` are reserved for future protocol headers or sub-namespace partitioning.
4. **Deprecations**: Historical uppercase or mixed-case identifiers from legacy repositories are unverified until re-encoded into canonical lowercase.
