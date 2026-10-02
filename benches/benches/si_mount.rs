use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::io::Write;
use tempfile::tempdir;

/// Benchmark: .si cartridge mount (memmap) latency
///
/// Measures the time to:
/// 1. Write a minimal .si cartridge to disk
/// 2. Memory-map it via memmap2
/// 3. Validate magic bytes
///
/// Target: < 50µs (design goal - validate here before claiming in docs)
///
/// Run: cargo bench --bench si_mount
fn si_mount_benchmark(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let cart_path = dir.path().join("bench_model.si");

    // Build a minimal valid .si cartridge (64-byte header + 64-byte payload)
    let mut header = [0u8; 64];
    header[0..4].copy_from_slice(b"SINT");
    header[4..6].copy_from_slice(&3u16.to_le_bytes());
    header[6..8].copy_from_slice(&64u16.to_le_bytes());
    header[8..12].copy_from_slice(&4u32.to_le_bytes()); // tier flags
    // block1_offset = 64, block1_len = 64
    header[16..24].copy_from_slice(&64u64.to_le_bytes());
    header[24..32].copy_from_slice(&64u64.to_le_bytes());

    let payload = [0xABu8; 64];
    let mut file = std::fs::File::create(&cart_path).unwrap();
    file.write_all(&header).unwrap();
    file.write_all(&payload).unwrap();
    file.flush().unwrap();

    c.bench_function("si_mount_and_validate", |b| {
        b.iter(|| {
            let file = std::fs::File::open(&cart_path).unwrap();
            let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
            // Validate magic
            assert!(&mmap[0..4] == b"SINT");
            black_box(&mmap);
        })
    });
}

criterion_group!(benches, si_mount_benchmark);
criterion_main!(benches);
