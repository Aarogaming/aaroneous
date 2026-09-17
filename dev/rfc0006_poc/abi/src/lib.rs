#![deny(unsafe_code)]
//! Shared wire-format types and pure encode/decode logic for the RFC-0006
//! command-buffer plugin ABI proof-of-concept
//! (`docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md`).
//!
//! Everything here is `#![deny(unsafe_code)]`: the wire format is decoded by
//! hand from byte slices (`u32::from_le_bytes` etc.), never by transmuting or
//! reinterpreting raw memory, so parsing an adversarial buffer can never be
//! unsound - at worst it returns a `DecodeError`. The only `unsafe` this PoC
//! needs (loading the library, resolving symbols, constructing the raw
//! buffer slice for the FFI call) lives in `host`'s isolated loader module.

/// Header size in bytes: `wire_version`, `command_count`, `payload_len`,
/// `reserved` - four `u32`s, little-endian.
pub const HEADER_LEN: usize = 16;
/// Fixed size in bytes of one command record (see field layout in
/// [`decode_commands`] / [`CommandBufferWriter::push`]).
pub const COMMAND_LEN: usize = 92;
/// Fixed capacity of a command's inline text field.
pub const MAX_TEXT_LEN: usize = 64;

/// Wire format version this PoC reads and writes.
pub const WIRE_VERSION: u32 = 1;

/// Inclusive range of `plugin_abi_version()` values this host PoC accepts.
pub const PLUGIN_ABI_MIN: u32 = 1;
pub const PLUGIN_ABI_MAX: u32 = 1;

pub const TICK_OK: u32 = 0;
pub const TICK_OVERFLOW: u32 = 1;
pub const TICK_FAULTED: u32 = 2;

/// Raw FFI return type of `plugin_tick`. `#[repr(C)]`, two plain `u32`s -
/// standard C-ABI layout, safe to return by value across the boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickResultRaw {
    pub status: u32,
    /// Meaningful only when `status == TICK_OK`: bytes written to the buffer.
    pub bytes_used: u32,
}

impl TickResultRaw {
    pub const fn ok(bytes_used: u32) -> Self {
        Self {
            status: TICK_OK,
            bytes_used,
        }
    }
    pub const fn overflow() -> Self {
        Self {
            status: TICK_OVERFLOW,
            bytes_used: 0,
        }
    }
    pub const fn faulted() -> Self {
        Self {
            status: TICK_FAULTED,
            bytes_used: 0,
        }
    }
}

/// A single draw command opcode. Explicit discriminants so an old host
/// reading a newer wire version recognizes an unknown opcode and skips it
/// rather than misinterpreting a later variant's fields.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandOp {
    Label = 0,
    Button = 1,
    BeginHorizontal = 2,
    BeginVertical = 3,
    End = 4,
    Spacing = 5,
}

impl CommandOp {
    /// Checked decode: unknown discriminants return `None` instead of being
    /// reinterpreted, so a newer plugin's opcode can never corrupt an older
    /// host's field layout (there is no `transmute` anywhere in this crate).
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Label),
            1 => Some(Self::Button),
            2 => Some(Self::BeginHorizontal),
            3 => Some(Self::BeginVertical),
            4 => Some(Self::End),
            5 => Some(Self::Spacing),
            _ => None,
        }
    }
}

/// A decoded command, borrowed from the source buffer (no allocation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodedCommand<'a> {
    pub op: CommandOp,
    pub widget_id: u64,
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub color_rgba: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    BufferTooSmallForHeader,
    ReservedFieldNonZero,
    TextLenExceedsCapacity,
    TextNotUtf8,
}

/// One safely-decoded buffer: the header plus however many trailing commands
/// actually fit. A plugin that lies about `command_count` gets its claim
/// clamped to what the buffer really holds - never an out-of-bounds read -
/// and an individual record with an unrecognized opcode is skipped rather
/// than failing the whole decode.
#[derive(Debug, Default, PartialEq)]
pub struct DecodedBuffer<'a> {
    pub wire_version: u32,
    pub commands: Vec<DecodedCommand<'a>>,
    /// Command slots the header claimed but the buffer did not actually
    /// contain (truncated, never read out of bounds).
    pub truncated_commands: u32,
    /// Command records with an opcode this decoder doesn't recognize.
    pub skipped_unknown_ops: u32,
}

/// Decode a command buffer a plugin wrote. Pure, bounds-checked, allocates
/// only the returned `Vec`. `used_len` is the byte count the plugin *claims*
/// to have written (its own `TickResultRaw::bytes_used`); this function never
/// trusts it past `buf.len()` - the host's own allocation is the hard
/// ceiling regardless of what the plugin reports. Likewise `payload_len` in
/// the header is read but never used to bound anything: only the real slice
/// length does, which is strictly more conservative than trusting either
/// self-reported field.
pub fn decode_commands(buf: &[u8], used_len: usize) -> Result<DecodedBuffer<'_>, DecodeError> {
    let used_len = used_len.min(buf.len());
    let buf = &buf[..used_len];
    if buf.len() < HEADER_LEN {
        return Err(DecodeError::BufferTooSmallForHeader);
    }
    let wire_version = read_u32(buf, 0);
    let claimed_count = read_u32(buf, 4) as usize;
    let reserved = read_u32(buf, 12);
    if reserved != 0 {
        return Err(DecodeError::ReservedFieldNonZero);
    }

    let available_bytes = buf.len() - HEADER_LEN;
    let available_commands = available_bytes / COMMAND_LEN;
    let actual_count = claimed_count.min(available_commands);
    let truncated_commands = claimed_count.saturating_sub(actual_count) as u32;

    let mut commands = Vec::with_capacity(actual_count);
    let mut skipped_unknown_ops = 0u32;
    for i in 0..actual_count {
        let off = HEADER_LEN + i * COMMAND_LEN;
        let record = &buf[off..off + COMMAND_LEN];
        let Some(op) = CommandOp::from_u32(read_u32(record, 0)) else {
            skipped_unknown_ops += 1;
            continue;
        };
        let widget_id = read_u64(record, 4);
        let text_len = read_u16(record, 76) as usize;
        if text_len > MAX_TEXT_LEN {
            return Err(DecodeError::TextLenExceedsCapacity);
        }
        let text_bytes = &record[12..12 + text_len];
        let text = std::str::from_utf8(text_bytes).map_err(|_| DecodeError::TextNotUtf8)?;
        let x = read_f32(record, 80);
        let y = read_f32(record, 84);
        let color_rgba = [record[88], record[89], record[90], record[91]];
        commands.push(DecodedCommand {
            op,
            widget_id,
            text,
            x,
            y,
            color_rgba,
        });
    }

    Ok(DecodedBuffer {
        wire_version,
        commands,
        truncated_commands,
        skipped_unknown_ops,
    })
}

fn read_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}
fn read_u64(buf: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(buf[at..at + 8].try_into().unwrap())
}
fn read_u16(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(buf[at..at + 2].try_into().unwrap())
}
fn read_f32(buf: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}

/// Safe encoder plugins use to build their buffer. The plugin never computes
/// a raw offset itself; every bounds check lives here, in one place.
pub struct CommandBufferWriter<'a> {
    buf: &'a mut [u8],
    command_count: u32,
    cursor: usize,
}

impl<'a> CommandBufferWriter<'a> {
    /// `buf` must be at least `HEADER_LEN` bytes; `None` otherwise (the
    /// caller should treat that as an immediate overflow).
    pub fn new(buf: &'a mut [u8]) -> Option<Self> {
        if buf.len() < HEADER_LEN {
            return None;
        }
        Some(Self {
            buf,
            command_count: 0,
            cursor: HEADER_LEN,
        })
    }

    /// Appends one command. Returns `false` (without partially writing) if
    /// there is not room for a full `COMMAND_LEN` record - the RFC requires
    /// this be a clean overflow signal, never a torn write.
    #[must_use]
    pub fn push(
        &mut self,
        op: CommandOp,
        widget_id: u64,
        text: &str,
        x: f32,
        y: f32,
        color_rgba: [u8; 4],
    ) -> bool {
        if self.buf.len() - self.cursor < COMMAND_LEN {
            return false;
        }
        let text_bytes = text.as_bytes();
        let text_len = text_bytes.len().min(MAX_TEXT_LEN);
        let off = self.cursor;
        write_u32(self.buf, off, op as u32);
        write_u64(self.buf, off + 4, widget_id);
        self.buf[off + 12..off + 12 + text_len].copy_from_slice(&text_bytes[..text_len]);
        for b in &mut self.buf[off + 12 + text_len..off + 12 + MAX_TEXT_LEN] {
            *b = 0;
        }
        write_u16(self.buf, off + 76, text_len as u16);
        write_u16(self.buf, off + 78, 0);
        write_f32(self.buf, off + 80, x);
        write_f32(self.buf, off + 84, y);
        self.buf[off + 88..off + 92].copy_from_slice(&color_rgba);
        self.cursor += COMMAND_LEN;
        self.command_count += 1;
        true
    }

    /// Finalizes the header and returns the total bytes written - ready to
    /// hand back as `TickResultRaw::bytes_used`.
    pub fn finish(self) -> u32 {
        write_u32(self.buf, 0, WIRE_VERSION);
        write_u32(self.buf, 4, self.command_count);
        write_u32(self.buf, 8, (self.cursor - HEADER_LEN) as u32);
        write_u32(self.buf, 12, 0);
        self.cursor as u32
    }
}

fn write_u32(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn write_u64(buf: &mut [u8], at: usize, v: u64) {
    buf[at..at + 8].copy_from_slice(&v.to_le_bytes());
}
fn write_u16(buf: &mut [u8], at: usize, v: u16) {
    buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn write_f32(buf: &mut [u8], at: usize, v: f32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_hello_world() {
        let mut buf = [0u8; HEADER_LEN + 2 * COMMAND_LEN];
        let mut w = CommandBufferWriter::new(&mut buf).unwrap();
        assert!(w.push(
            CommandOp::Label,
            0,
            "Hello, plugin!",
            10.0,
            10.0,
            [255, 255, 255, 255]
        ));
        assert!(w.push(
            CommandOp::Button,
            42,
            "Click me",
            10.0,
            30.0,
            [0, 128, 255, 255]
        ));
        let used = w.finish();

        let decoded = decode_commands(&buf, used as usize).unwrap();
        assert_eq!(decoded.wire_version, WIRE_VERSION);
        assert_eq!(decoded.truncated_commands, 0);
        assert_eq!(decoded.skipped_unknown_ops, 0);
        assert_eq!(decoded.commands.len(), 2);
        assert_eq!(decoded.commands[0].op, CommandOp::Label);
        assert_eq!(decoded.commands[0].text, "Hello, plugin!");
        assert_eq!(decoded.commands[1].op, CommandOp::Button);
        assert_eq!(decoded.commands[1].widget_id, 42);
    }

    #[test]
    fn writer_refuses_to_overflow_capacity() {
        let mut buf = [0u8; HEADER_LEN + COMMAND_LEN]; // room for exactly 1 command
        let mut w = CommandBufferWriter::new(&mut buf).unwrap();
        assert!(w.push(CommandOp::Label, 0, "fits", 0.0, 0.0, [0, 0, 0, 0]));
        assert!(!w.push(CommandOp::Label, 1, "does not fit", 0.0, 0.0, [0, 0, 0, 0]));
    }

    #[test]
    fn decode_clamps_a_lying_command_count_instead_of_reading_oob() {
        let mut buf = [0u8; HEADER_LEN + COMMAND_LEN]; // real capacity: 1 command
        write_u32(&mut buf, 0, WIRE_VERSION);
        write_u32(&mut buf, 4, 999_999); // claims 999,999 commands
        write_u32(&mut buf, 8, 0);
        write_u32(&mut buf, 12, 0);

        let decoded = decode_commands(&buf, buf.len()).unwrap();
        // The one available slot is all zero bytes, which happens to decode
        // as a valid (opcode 0 = Label, zero-length text) command - the
        // point of this test isn't that slot's content, it's that decoding
        // 999,999 claimed commands against a 1-command buffer never reads
        // past that buffer: exactly 1 command comes out, not 999,999.
        assert_eq!(decoded.commands.len(), 1);
        assert_eq!(decoded.truncated_commands, 999_999 - 1);
    }

    #[test]
    fn decode_rejects_buffer_shorter_than_header() {
        let buf = [0u8; HEADER_LEN - 1];
        assert_eq!(
            decode_commands(&buf, buf.len()),
            Err(DecodeError::BufferTooSmallForHeader)
        );
    }

    #[test]
    fn decode_rejects_nonzero_reserved_field() {
        let mut buf = [0u8; HEADER_LEN];
        write_u32(&mut buf, 12, 1);
        assert_eq!(
            decode_commands(&buf, buf.len()),
            Err(DecodeError::ReservedFieldNonZero)
        );
    }

    #[test]
    fn decode_skips_unknown_opcode_without_failing() {
        let mut buf = [0u8; HEADER_LEN + COMMAND_LEN];
        write_u32(&mut buf, 0, WIRE_VERSION);
        write_u32(&mut buf, 4, 1);
        write_u32(&mut buf, 8, COMMAND_LEN as u32);
        write_u32(&mut buf, 12, 0);
        write_u32(&mut buf, HEADER_LEN, 999); // unknown opcode

        let decoded = decode_commands(&buf, buf.len()).unwrap();
        assert_eq!(decoded.commands.len(), 0);
        assert_eq!(decoded.skipped_unknown_ops, 1);
    }

    proptest::proptest! {
        /// Criterion 4: no arbitrary byte sequence, of any length, with any
        /// claimed command_count, ever panics or reads past `buf`'s own
        /// length - the strongest form of "truncated, not read out of
        /// bounds" is that this function simply can't panic.
        #[test]
        fn decode_never_panics_on_adversarial_bytes(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..2048)) {
            let _ = decode_commands(&bytes, bytes.len());
        }

        #[test]
        fn decode_never_panics_when_used_len_exceeds_buffer(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..512),
            claimed_used_len in 0usize..100_000,
        ) {
            let _ = decode_commands(&bytes, claimed_used_len);
        }
    }
}
