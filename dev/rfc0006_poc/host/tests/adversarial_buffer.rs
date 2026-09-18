//! RFC-0006 Section 8, criterion 4. (The broader property - no adversarial
//! *byte sequence*, not just this one plugin's specific lie, ever causes a
//! panic or an out-of-bounds read - is proven separately and more
//! thoroughly by `rfc0006_abi`'s own `proptest`-based
//! `decode_never_panics_on_adversarial_bytes` test, which fuzzes
//! `decode_commands` directly with no FFI involved at all. This test proves
//! the same property end-to-end, through a real loaded plugin.)

mod common;

use rfc0006_abi::{COMMAND_LEN, HEADER_LEN};
use rfc0006_host::{BUFFER_CAPACITY, LoadedPlugin, TickOutcome};

#[test]
fn a_plugin_lying_about_command_count_is_truncated_not_read_out_of_bounds() {
    let path = common::plugin_path("rfc0006_plugin_liar");
    let mut plugin =
        LoadedPlugin::load(&path).expect("liar plugin should load: it reports a valid ABI version");

    // The maximum number of full command records that could possibly fit in
    // the host's own buffer - the hard ceiling regardless of what the
    // plugin claims.
    let max_possible_commands = (BUFFER_CAPACITY - HEADER_LEN) / COMMAND_LEN;

    match plugin.tick() {
        TickOutcome::Ok(decoded) => {
            // The safety property: however outrageous the plugin's claimed
            // command_count (it claims u32::MAX), the decoded command list
            // can never exceed what the real buffer holds.
            assert!(
                decoded.commands.len() <= max_possible_commands,
                "decoded {} commands, but the buffer can only ever hold {}",
                decoded.commands.len(),
                max_possible_commands
            );
            // The liar only ever writes its 16-byte header; every byte past
            // it is still this host's own zero-initialized buffer, which
            // happens to decode as `max_possible_commands` well-formed
            // (opcode 0 = Label, zero-length text) empty commands - not a
            // crash, not an out-of-bounds read, just a truncated, entirely
            // safe (if useless) result.
            assert_eq!(decoded.commands.len(), max_possible_commands);
            assert_eq!(
                decoded.truncated_commands,
                u32::MAX - max_possible_commands as u32
            );
        }
        other => panic!("expected Ok (truncated, not a decode failure or crash), got {other:?}"),
    }

    // And the process is, self-evidently, still here to check that.
}
