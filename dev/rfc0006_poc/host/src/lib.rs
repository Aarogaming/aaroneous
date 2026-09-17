#![deny(unsafe_code)]
//! RFC-0006 PoC host: loads a plugin `cdylib`, negotiates its ABI version,
//! ticks it for a command buffer, decodes the result, and unloads it.
//!
//! All `unsafe` is isolated to the [`loader`] module (library loading,
//! symbol resolution, and the FFI calls themselves - the only things that
//! cannot be expressed in safe Rust) and documented with `// SAFETY:`
//! comments per `AGENTS.md`'s Domain-Aware Unsafe Permissions. Everything
//! else in this crate, including the whole command-buffer decode path
//! (`rfc0006_abi::decode_commands`), is `#![deny(unsafe_code)]`.

pub mod loader;

pub use loader::{BUFFER_CAPACITY, LoadError, LoadedPlugin, TickOutcome};
