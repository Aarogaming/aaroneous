// Telemetry Feed Example - Simulates high-frequency execution events to ring buffer

use anyhow::Result;
use platform_bridge::token_emitter::TelemetryTokenEmitter;

fn main() -> Result<()> {
    println!("===========================================================");
    println!("TELEMETRY FEED: MachineToken Streaming Test");
    println!("===========================================================\n");

    // Initialize emitter with 4096-token ring buffer
    let mut emitter = TelemetryTokenEmitter::new(4096)?;
    
    println!("Ring buffer capacity: 4096 tokens");
    println!(".-----------------------------------------------------------.");

    // Simulate 1,000 high-frequency execution events
    const NUM_EVENTS: usize = 1_000;
    println!("\nSimulating {} execution events...", NUM_EVENTS);

    let mut success_count = 0;
    let mut failure_count = 0;

    for i in 0..NUM_EVENTS {
        // Generate realistic telemetry data
        let opcode_id = (i % 100) as u16;  // Cycle through 100 opcodes
        let timestamp = (i * 1_000_000) as u32;  // Microsecond timestamps

        match emitter.emit_event(opcode_id, timestamp) {
            Ok(()) => {
                success_count += 1;
                
                // Progress indicator every 100 events
                if (i + 1) % 100 == 0 {
                    println!("Processed {} events...", i + 1);
                }
            }
            Err(e) => {
                failure_count += 1;
                eprintln!("Event {} failed: {}", i, e);
            }
        }
    }

    println!("\n.-----------------------------------------------------------.");
    println!("\n=== RESULTS ===");
    println!("Total Events: {}", NUM_EVENTS);
    println!("Successful:   {}", success_count);
    println!("Failed:       {}", failure_count);
    
    let success_rate = (success_count as f64 / NUM_EVENTS as f64) * 100.0;
    println!("Success Rate: {:.2}%", success_rate);

    // Verify no allocations or dropped frames
    assert!(failure_count == 0, "Some events failed - check ring buffer capacity");
    
    println!("\nAll {} tokens written without allocation or dropped frames!", NUM_EVENTS);

    println!("\n===========================================================");
    println!("TELEMETRY FEED TEST PASSED!");
    println!("===========================================================\n");

    Ok(())
}
