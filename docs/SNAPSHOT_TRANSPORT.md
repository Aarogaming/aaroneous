# Snapshot transport contract

The version 3 snapshot bridge is a bounded, nonblocking shared-memory transport on targets with native 64-bit atomics. Publication and reading use atomic 64-bit words throughout, including the payload. Shared bytes must never be accessed as a `SnapshotRingSlot` or `EngineSnapshotPod` reference. Those types describe local values and geometry only.

The segment contains a 32-byte header followed by 64 slots. Each slot contains a begin sequence, an end sequence, and the payload. All accesses use sequentially consistent ordering. Publishers acquire the shared header claim with a single compare/exchange attempt; contention returns `SnapshotPublishError::Busy`. Under that claim a publisher reads the shared sequence, invalidates the slot begin sequence, stores the payload words, stores the end and begin sequences, and publishes the header sequence. Sequence overflow is rejected.

A reader loads the published sequence, checks the begin marker, copies every payload word into private storage, and then checks both markers again. A writer reusing the slot must invalidate the begin marker before its payload writes. In the sequentially consistent order, a reader that observes words from that reuse cannot also observe the original begin marker after its copy. It therefore rejects a mixed snapshot. No ordinary concurrent payload reads or writes occur. Reading returns the latest available frame; it does not promise delivery of every frame.

Mapping acquisition is control-plane I/O. `SwmrSnapshotReader::refresh` explicitly retries a missing mapping. A mapped reader does not reopen the file when there are no new frames. Mappings use read/write access to support atomic objects, although the reader API only loads words. The application must not truncate, replace, or modify the mapped file outside this protocol while peers are active. Unsupported versions and geometry are rejected.

The default application endpoint is `engine_state_v3`; it does not reuse the old version 2 endpoint. Custom configurations must likewise use a fresh segment when migrating from a non-atomic implementation. Native endianness and the Rust-defined Pod payload layout require peers with the same target ABI and protocol version.

The writer claim is released on normal return, including error returns. If a process dies while holding it, the segment remains unavailable for publication. Recovery must stop the old peers and supply a fresh segment path; automatically stealing a potentially live writer's claim is deliberately unsupported. Readers may still return the last fully published frame. The transport is telemetry, not a durable event journal.

Regression tests cover forced slot reuse between payload copying and validation, mismatched markers, independent publishers sharing the sequence and claim, incompatible versions, dropped-frame accounting, and concurrent readers. These tests are not a formal proof of all platform behavior or a worst-case latency benchmark.
