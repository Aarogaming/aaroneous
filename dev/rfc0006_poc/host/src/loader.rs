#![allow(unsafe_code)]
//! The RFC-0006 PoC's entire `unsafe` surface: dynamic library loading,
//! symbol resolution, and the FFI calls into a plugin's `extern "C"`
//! entry points. Every `unsafe` block below carries a `// SAFETY:` comment
//! naming the specific invariant it relies on, per AGENTS.md's
//! Domain-Aware Unsafe Permissions. Nothing outside this module needs
//! `unsafe`: the command buffer itself is read back through
//! `rfc0006_abi::decode_commands`, a pure safe function operating on an
//! ordinary `&[u8]` this host already owns.

use std::path::Path;

use libloading::{Library, Symbol};
use rfc0006_abi::{
    DecodeError, DecodedBuffer, PLUGIN_ABI_MAX, PLUGIN_ABI_MIN, TICK_FAULTED, TICK_OK,
    TICK_OVERFLOW, TickResultRaw, decode_commands,
};

/// Size of the buffer the host allocates and hands to a plugin each tick.
/// Bounds how much work a single `plugin_tick` call can claim to have done -
/// see RFC-0006 Section 3's "bounded work per tick" design goal.
pub const BUFFER_CAPACITY: usize = 8192;

#[derive(Debug)]
pub enum LoadError {
    Library(libloading::Error),
    MissingSymbol {
        name: &'static str,
        source: libloading::Error,
    },
    UnsupportedAbiVersion {
        reported: u32,
        min: u32,
        max: u32,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Library(e) => write!(f, "failed to load plugin library: {e}"),
            Self::MissingSymbol { name, source } => {
                write!(f, "plugin is missing required symbol `{name}`: {source}")
            }
            Self::UnsupportedAbiVersion { reported, min, max } => write!(
                f,
                "plugin reports ABI version {reported}, outside this host's supported range [{min}, {max}]"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug)]
pub enum TickOutcome<'a> {
    Ok(DecodedBuffer<'a>),
    Overflow,
    Faulted,
    /// The plugin returned a status code this host doesn't recognize -
    /// treated as opaque failure, never interpreted as success.
    UnknownStatus(u32),
    Decode(DecodeError),
}

type PluginAbiVersionFn = unsafe extern "C" fn() -> u32;
type PluginTickFn = unsafe extern "C" fn(*mut u8, usize) -> TickResultRaw;
type PluginHandleEventFn = unsafe extern "C" fn(*const u8, usize);
type PluginShutdownFn = unsafe extern "C" fn();

/// A loaded, version-checked plugin. Dropping it calls `plugin_shutdown`
/// and then unloads the library (via `libloading::Library`'s own `Drop`),
/// completing the RFC-0006 lifecycle's `Unload` step.
pub struct LoadedPlugin {
    library: Library,
    buffer: Vec<u8>,
}

impl LoadedPlugin {
    /// Loads `path`, resolves the fixed RFC-0006 symbol set, and checks
    /// `plugin_abi_version()` against this host's supported range. A plugin
    /// missing any required symbol, or outside the version range, is
    /// refused with a specific `LoadError` - never a crash, never a silent
    /// partial activation (RFC-0006 Section 4, "Load"/"Version Negotiate").
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        // SAFETY: `Library::new` runs the target's load-time initializers -
        // the one unavoidable risk of dynamic loading at all. Authenticating
        // *which* paths are safe to load is an explicitly separate,
        // orthogonal mechanism per RFC-0006 Section 6 ("Authentication stays
        // a separate mechanism") and out of scope for this ABI-soundness
        // PoC; every path this PoC loads is one of its own just-built test
        // fixtures under `../plugins`, never attacker-controlled input.
        let library = unsafe { Library::new(path) }.map_err(LoadError::Library)?;

        let version = {
            // SAFETY: resolving a symbol by name only reads the library's
            // exported-symbol table; it does not call into plugin code.
            let sym: Symbol<PluginAbiVersionFn> = unsafe { library.get(b"plugin_abi_version\0") }
                .map_err(|source| LoadError::MissingSymbol {
                name: "plugin_abi_version",
                source,
            })?;
            // SAFETY: the resolved symbol's type (`unsafe extern "C" fn() -> u32`)
            // matches the RFC-0006 contract for `plugin_abi_version`, which
            // must be side-effect-free and take no arguments.
            unsafe { sym() }
        };
        if version < PLUGIN_ABI_MIN || version > PLUGIN_ABI_MAX {
            return Err(LoadError::UnsupportedAbiVersion {
                reported: version,
                min: PLUGIN_ABI_MIN,
                max: PLUGIN_ABI_MAX,
            });
        }

        // Resolve the rest of the fixed symbol set now, so a plugin missing
        // any of them is refused at load time rather than failing later
        // mid-lifecycle ("no partial activation").
        {
            let _: Symbol<PluginTickFn> =
                unsafe { library.get(b"plugin_tick\0") }.map_err(|source| {
                    LoadError::MissingSymbol {
                        name: "plugin_tick",
                        source,
                    }
                })?;
            let _: Symbol<PluginHandleEventFn> = unsafe { library.get(b"plugin_handle_event\0") }
                .map_err(|source| LoadError::MissingSymbol {
                name: "plugin_handle_event",
                source,
            })?;
            let _: Symbol<PluginShutdownFn> = unsafe { library.get(b"plugin_shutdown\0") }
                .map_err(|source| LoadError::MissingSymbol {
                    name: "plugin_shutdown",
                    source,
                })?;
        }

        Ok(Self {
            library,
            buffer: vec![0u8; BUFFER_CAPACITY],
        })
    }

    /// Runs one tick: hands the plugin a buffer this host owns, then decodes
    /// whatever it wrote back through the pure-safe `decode_commands` - the
    /// plugin's returned byte count is a claim, never trusted past the
    /// buffer's real length (`decode_commands` re-derives its own bound).
    pub fn tick(&mut self) -> TickOutcome<'_> {
        let raw = {
            // SAFETY: resolved and type-checked once already at `load`;
            // re-resolving here (rather than caching a `Symbol<'_>`, whose
            // lifetime would otherwise pin an immutable borrow of
            // `self.library` for as long as it's kept) avoids fighting the
            // borrow checker over `self.buffer`'s concurrent mutable
            // borrow below - both are the same underlying symbol.
            let sym: Symbol<PluginTickFn> = match unsafe { self.library.get(b"plugin_tick\0") } {
                Ok(s) => s,
                Err(_) => return TickOutcome::Faulted,
            };
            let ptr = self.buffer.as_mut_ptr();
            let cap = self.buffer.len();
            // SAFETY: `ptr` is valid for `cap` writable bytes for the
            // duration of this call alone (borrowed from `self.buffer`,
            // owned by `self` and not touched elsewhere concurrently). The
            // plugin's own `plugin_tick` is responsible, per the RFC-0006
            // contract, for never writing past `cap` and for catching its
            // own panics before returning across this boundary (see
            // `plugins/panicker` for the fixture that verifies exactly
            // that contract).
            unsafe { sym(ptr, cap) }
        };
        match raw.status {
            TICK_OK => match decode_commands(&self.buffer, raw.bytes_used as usize) {
                Ok(decoded) => TickOutcome::Ok(decoded),
                Err(e) => TickOutcome::Decode(e),
            },
            TICK_OVERFLOW => TickOutcome::Overflow,
            TICK_FAULTED => TickOutcome::Faulted,
            other => TickOutcome::UnknownStatus(other),
        }
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        // SAFETY: symbol resolution only reads the export table. A failure
        // to resolve `plugin_shutdown` here is ignored (best-effort at drop
        // time, matching Rust's general rule against panicking in `Drop`)
        // rather than surfaced as an error - `load` already required this
        // symbol to exist, so this can only fail if the library's export
        // table changed underneath us, which cannot happen for a `Library`
        // this `LoadedPlugin` exclusively owns.
        if let Ok(sym) = unsafe { self.library.get::<PluginShutdownFn>(b"plugin_shutdown\0") } {
            // SAFETY: no arguments; RFC-0006 requires this be safe to call
            // exactly once per successful `load`, immediately before unload,
            // which is exactly this `Drop` impl's contract.
            unsafe { sym() };
        }
    }
}
