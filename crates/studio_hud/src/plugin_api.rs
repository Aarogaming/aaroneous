//! In-process UI cartridge registry.
//!
//! # History: why there is no dynamic (out-of-process) loading here
//!
//! This module used to also offer `load_dynamic_plugin`/`unload_dynamic_plugin`,
//! hot-loading a `.dll`/`.so` and reconstructing a `Box<dyn UiCartridge>` from a
//! raw pointer returned across an `extern "C"` boundary. A security revalidation
//! (queue item C2 in the private operations workspace) found two real problems
//! with that mechanism and removed it rather than patching around it:
//!
//! 1. The "signature validation" gating the load was not a signature check at
//!    all — it hashed the file and only rejected a hash whose first two bytes
//!    happened to be zero, which passes almost any file, malicious or not.
//! 2. Even a real signature check would not have made the load *sound*: this
//!    module's [`UiCartridge`] trait's `render` method takes `&mut egui::Ui`,
//!    and `egui::Ui` is a complex, non-`repr(C)` third-party type with no
//!    stable binary layout across compiler versions or even different builds
//!    of the same egui version. Passing it across a real DLL boundary is
//!    undefined behavior regardless of how well-authenticated the plugin is —
//!    authentication and ABI safety are orthogonal concerns, and fixing one
//!    does not fix the other.
//!
//! A sound dynamic-plugin mechanism for this trait would need a real stable
//! ABI — most plausibly an intermediate `repr(C)` command buffer the plugin
//! writes draw commands into, which the host then replays against its own
//! `egui::Ui`, rather than handing the plugin a live reference to host-owned,
//! non-FFI-safe state. That is a real design effort, not a bounded fix; it
//! belongs to the plugin-lifecycle RFC work in the private operations
//! workspace, not here. Until that exists, cartridges are in-process Rust
//! trait objects only — [`PluginManager::load_cartridge`] takes an
//! already-constructed `Box<dyn UiCartridge>` the same binary compiled it.
//!
//! # Why this module still exists with no call sites
//!
//! Nothing in `studio_hud` currently constructs a [`PluginManager`] or calls
//! [`PluginManager::load_cartridge`] (queue item M4 in the private operations
//! workspace reproduced this: zero references outside this module's own
//! tests). That is expected, not a sign this is orphaned code to delete:
//! `TODO.md`'s roadmap lists "dynamic plugin swapping" under **P4: Adaptive
//! Runtime Engine** and "Hot-reload ABI plugins" under **Phase 5: Adaptive
//! Runtime & Live Patching (v0.8.0)** as a real, planned capability, not yet
//! due. This module is the sound, tested foundation that capability will be
//! built on once the plugin-lifecycle RFC (above) defines a real ABI — kept
//! deliberately narrow (in-process only) rather than removed, so the next
//! implementer starts from verified-safe code instead of rebuilding it.

use api::UiCartridge;

/// RFC-0006 Stable Command Buffer ABI Header
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBufferHeader {
    pub wire_version: u32,
    pub command_count: u32,
    pub payload_len: u32,
    pub _reserved: u32,
}

/// Command Opcode discriminants for RFC-0006 stable ABI
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOp {
    Label = 0,
    Button = 1,
    BeginHorizontal = 2,
    BeginVertical = 3,
    End = 4,
    Spacing = 5,
}

/// RFC-0006 Fixed-Size `#[repr(C)]` Draw Command Record
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub op: u32,
    pub _pad0: u32,
    pub widget_id: u64,
    pub text: [u8; 64],
    pub text_len: u16,
    pub _pad1: u16,
    pub x: f32,
    pub y: f32,
    pub color_rgba: [u8; 4],
}

impl Command {
    pub fn op(&self) -> CommandOp {
        match self.op {
            0 => CommandOp::Label,
            1 => CommandOp::Button,
            2 => CommandOp::BeginHorizontal,
            3 => CommandOp::BeginVertical,
            4 => CommandOp::End,
            5 => CommandOp::Spacing,
            _ => CommandOp::Label,
        }
    }

    pub fn text_as_str(&self) -> &str {
        let valid_len = (self.text_len as usize).min(self.text.len());
        let slice = &self.text[..valid_len];
        let nul_pos = slice.iter().position(|&b| b == 0).unwrap_or(valid_len);
        std::str::from_utf8(&slice[..nul_pos]).unwrap_or("")
    }
}

/// RFC-0006 FFI Tick Execution Result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickResult {
    Ok = 0,
    PluginFaulted = 1,
}

/// Registry of in-process UI cartridges and host-side command replay engine.
#[derive(Default)]
pub struct PluginManager {
    pub static_cartridges: Vec<Box<dyn UiCartridge>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_cartridge(&mut self, cartridge: Box<dyn UiCartridge>) {
        self.static_cartridges.push(cartridge);
    }

    /// Replays RFC-0006 bounded draw commands against a host `egui::Ui` viewport.
    pub fn replay_commands(
        &self,
        header: &CommandBufferHeader,
        commands: &[Command],
        ui: &mut eframe::egui::Ui,
    ) -> usize {
        if header.wire_version != 1 {
            return 0;
        }

        let max_commands = (header.command_count as usize).min(commands.len());
        let mut replayed = 0;

        for cmd in &commands[..max_commands] {
            match cmd.op() {
                CommandOp::Label => {
                    ui.label(cmd.text_as_str());
                    replayed += 1;
                }
                CommandOp::Button => {
                    let _ = ui.button(cmd.text_as_str());
                    replayed += 1;
                }
                CommandOp::BeginHorizontal => {
                    ui.horizontal(|_| ());
                    replayed += 1;
                }
                CommandOp::BeginVertical => {
                    ui.vertical(|_| ());
                    replayed += 1;
                }
                CommandOp::End => {
                    replayed += 1;
                }
                CommandOp::Spacing => {
                    ui.add_space(cmd.x.max(0.0));
                    replayed += 1;
                }
            }
        }

        replayed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    struct MockCartridge {
        name: String,
        render_calls: usize,
    }

    impl UiCartridge for MockCartridge {
        fn name(&self) -> &str {
            &self.name
        }

        fn render(&mut self, _ui: &mut egui::Ui) {
            self.render_calls += 1;
        }
    }

    #[test]
    fn command_buffer_header_geometry_is_16_bytes() {
        assert_eq!(std::mem::size_of::<CommandBufferHeader>(), 16);
    }

    #[test]
    fn command_geometry_is_fixed_and_aligned() {
        assert_eq!(std::mem::size_of::<Command>(), 96);
    }

    #[test]
    fn unknown_command_op_defaults_to_label_without_panic() {
        let cmd = Command {
            op: 999,
            _pad0: 0,
            widget_id: 42,
            text: [0u8; 64],
            text_len: 0,
            _pad1: 0,
            x: 0.0,
            y: 0.0,
            color_rgba: [0, 0, 0, 255],
        };
        assert_eq!(cmd.op(), CommandOp::Label);
        assert_eq!(cmd.text_as_str(), "");
    }

    #[test]
    fn text_bounds_are_clamped_safely() {
        let mut text = [0u8; 64];
        text[..5].copy_from_slice(b"hello");
        let cmd = Command {
            op: 0,
            _pad0: 0,
            widget_id: 1,
            text,
            text_len: 128, // Invalid overflow length
            _pad1: 0,
            x: 0.0,
            y: 0.0,
            color_rgba: [255, 255, 255, 255],
        };
        assert_eq!(cmd.text_as_str(), "hello");
    }

    #[test]
    fn new_manager_has_no_cartridges() {
        let manager = PluginManager::new();
        assert!(manager.static_cartridges.is_empty());
    }

    #[test]
    fn default_manager_has_no_cartridges() {
        let manager = PluginManager::default();
        assert!(manager.static_cartridges.is_empty());
    }

    #[test]
    fn load_cartridge_registers_it_by_name() {
        let mut manager = PluginManager::new();
        manager.load_cartridge(Box::new(MockCartridge {
            name: "test-cartridge".to_string(),
            render_calls: 0,
        }));

        assert_eq!(manager.static_cartridges.len(), 1);
        assert_eq!(manager.static_cartridges[0].name(), "test-cartridge");
    }

    #[test]
    fn multiple_cartridges_load_independently_and_preserve_order() {
        let mut manager = PluginManager::new();
        manager.load_cartridge(Box::new(MockCartridge {
            name: "first".to_string(),
            render_calls: 0,
        }));
        manager.load_cartridge(Box::new(MockCartridge {
            name: "second".to_string(),
            render_calls: 0,
        }));

        assert_eq!(manager.static_cartridges.len(), 2);
        assert_eq!(manager.static_cartridges[0].name(), "first");
        assert_eq!(manager.static_cartridges[1].name(), "second");
    }
}
