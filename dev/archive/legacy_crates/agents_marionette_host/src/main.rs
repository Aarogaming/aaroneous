use anyhow::Result;
use marionette_host::{host, host_with_gate, pull_string_mouse, pull_string_vision, pull_string_network};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("=== Marionette Host ===");
    
    // Create host with permission gate
    let host = host_with_gate(true)?;
    
    // Test host functions
    println!("\n--- Host Functions ---");
    
    let mouse = host.pull_string_mouse(100, 200)?;
    println!("Mouse: {}", mouse.data);
    
    let vision = host.pull_string_vision()?;
    println!("Vision: {}", vision);
    
    let network = host.pull_string_network("https://example.com")?;
    println!("Network: {}", network);
    
    println!("\n=== Marionette Host Ready ===");
    
    Ok(())
}