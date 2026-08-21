use anyhow::Result;
use deconstruction::deconstruct;
use std::path::Path;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("=== Deconstruction Pipeline ===");
    
    // Example: Deconstruct a WASM file
    let wasm_path = Path::new("test.wasm");
    let result = deconstruct(wasm_path);
    
    match result {
        Ok(deconstruction_result) => {
            println!("\n--- Deconstruction Result ---");
            println!("Success: {}", deconstruction_result.success);
            println!("Original size: {} bytes", deconstruction_result.metadata.unwrap().original_size);
            println!("Functions: {}", deconstruction_result.metadata.unwrap().functions_count);
            println!("Tables: {}", deconstruction_result.metadata.unwrap().tables_count);
            println!("Memory: {}", deconstruction_result.metadata.unwrap().memory_count);
            
            if let Some(wat) = &deconstruction_result.wat {
                println!("\n--- WAT Output ---");
                println!("{}", wat);
            }
            
            if let Some(decompiled) = &deconstruction_result.decompiled {
                println!("\n--- Decompiled Code ---");
                println!("{}", decompiled);
            }
        }
        Err(e) => {
            println!("\n--- Error ---");
            println!("{}", e);
        }
    }
    
    println!("\n=== Deconstruction Pipeline Ready ===");
    
    Ok(())
}