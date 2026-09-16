use criterion::{Criterion, criterion_group, criterion_main};
use emulator_harness::{ReductionBudget, TraceEvent, reduce_trace_bounded};

fn generate_synthetic_trace(count: usize) -> Vec<TraceEvent> {
    let mut trace = Vec::with_capacity(count);
    for i in 0..count {
        trace.push(TraceEvent {
            timestamp_qpc: i as u64 * 10,
            pc: 0x1000 + (i as u64 * 4),
            memory_addr: if i % 2 == 0 { 0xBEEF } else { 0xCAFE },
            prev_value: i as u32,
            next_value: (i + 1) as u32,
            opcode: (i % 255) as u16,
            access_kind: if i % 3 == 0 { 1 } else { 2 },
            reserved: [0; 5],
        });
    }
    trace
}

fn bench_trace_reduction(c: &mut Criterion) {
    let trace_1k = generate_synthetic_trace(1000);

    c.bench_function("reduce_trace_bounded_1k_events", |b| {
        b.iter(|| {
            let _ = reduce_trace_bounded(&trace_1k, 0xBEEF, ReductionBudget::UNLIMITED);
        })
    });
}

criterion_group!(benches, bench_trace_reduction);
criterion_main!(benches);
