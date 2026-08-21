use anyhow::Result;
use transpiler::transpile;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Philosopher's Stone Transpiler ===");

    // Example: Transpile text to Rust code
    let text = "This is a simple article about defragmentation.";
    let code = transpile(text, "text")?;

    println!("\n--- Transpiled Code ---");
    println!("{}", code);

    println!("\n=== Transpiler Ready ===");

    Ok(())
}
