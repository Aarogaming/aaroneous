import struct

# Python implementation of the Synapse Schema
# Ensures "Byte-Level Jive" matches Rust exactly

SYNAPSE_HEADER_FORMAT = "<4sIII" # Magic[4], Version, Status, CommandPtr

def parse_header(buffer):
    magic, version, status, cmd_ptr = struct.unpack(SYNAPSE_HEADER_FORMAT, buffer[:16])
    return {
        "magic": magic,
        "version": version,
        "status": status,
        "command_ptr": cmd_ptr
    }

def pack_header(status=0, cmd_ptr=0):
    return struct.pack(SYNAPSE_HEADER_FORMAT, b"\xAA\x55\xAA\x55", 1, status, cmd_ptr)
