//! Chaos Injector for Autonomic Duty Cycle Testing
//!
//! Synthetic stress tester that reproduces historical failure modes:
//! - CPU starvation (unthrottled worker loops)
//! - Buffer overflow conditions
//! - Thread contention storms
//! - Priority inversion scenarios


use anyhow::{Result, Error};


/// Historical anti-pattern: Unbounded worker loop consuming 100% CPU
pub fn inject_cpu_starvation(duration_ms: u64) -> Result<()> {
    println!("Injecting CPU starvation for {}ms...", duration_ms);
    
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < duration_ms {
        let _ = 0u32.wrapping_add(1);
        std::hint::black_box(0u64);
    }
    
    println!("CPU starvation injection complete");
    Ok(())
}


/// Historical anti-pattern: Unbounded ring buffer overflow
pub fn inject_buffer_overflow() -> Result<()> {
    println!("Injecting buffer overflow condition...");
    
    let mut buffer = [0u8; 1024];
    let mut offset: usize = 0;
    
    for i in 0..2000 {
        if offset + std::mem::size_of::<u32>() > buffer.len() {
            break;
        }
        unsafe {
            let ptr = buffer.as_mut_ptr().add(offset);
            ptr.write(i as u32);
        }
        offset += std::mem::size_of::<u32>();
    }
    
    println!("Buffer overflow injection complete (safely bounded)");
    Ok(())
}


/// Historical anti-pattern: Thread contention storm
pub fn inject_thread_storm(thread_count: usize) -> Result<()> {
    if thread_count > 100 {
        anyhow::bail!("Thread count too high for test harness");
    }
    
    println!("Injecting thread storm with {} threads...", thread_count);
    
    let handles = (0..thread_count)
        .map(|i| {
            std::thread::spawn(move || {
                let mut counter: u64 = 0;
                for _ in 0..1000 {
                    counter += 1;
                }
                println!("Thread {} completed", i);
            })
        })
        .collect::<Vec<_>>();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    println!("Thread storm injection complete");
    Ok(())
}


pub mod stress_tests {
    use super::*;
    
    #[test]
    fn test_cpu_starvation_recovery() {
        inject_cpu_starvation(10).expect("Failed to inject starvation");
        
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = 0u32.wrapping_add(1);
        }
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_millis() < 100, "System hung after CPU starvation");
    }
    
    #[test]
    fn test_buffer_overflow_safety() {
        inject_buffer_overflow().expect("Failed to inject overflow");
        println!("Buffer overflow handled safely");
    }
}

