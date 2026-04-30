// Example: Model Discovery & Environment Detection
// Auto-detects installed model loading software and finds available GGUF models

use a_run::{ModelEnvironmentDetector, ModelLoader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🔍 Aaroneous Model Discovery & Environment Detection\n");
    println!("{}", "=".repeat(70));

    // Step 1: Detect model environments
    println!("\n📱 Step 1: Detecting model loading software...\n");
    let mut detector = ModelEnvironmentDetector::new();
    detector.scan()?;
    detector.print_detected_environments();

    // Step 2: Select best environment and scan models
    if let Some(selected) = detector.select_environment_interactive() {
        println!("\n📂 Step 2: Scanning for GGUF models in {}...\n", selected.environment.name());

        let mut loader = ModelLoader::new();
        
        // Add the detected environment's search path
        loader.add_search_path(selected.model_path.clone());
        
        // Initialize (scan for models)
        loader.initialize().await?;

        // Print available models
        loader.print_available_models();

        // Print recommendations
        loader.print_recommendations();

        // Show what we'd use by default
        if let Some(model) = loader.get_recommended_model() {
            println!("✅ Default model configured: {}", model.name);
            println!("   Path: {}\n", model.path.display());
        } else {
            println!("\n⚠️  No models found in {}.", selected.environment.name());
            println!("\nTo add models:");
            match selected.environment {
                a_run::ModelEnvironment::LMStudio => {
                    println!("   1. Open LM Studio");
                    println!("   2. Search for 'qwen 1.8b'");
                    println!("   3. Click 'Download'");
                    println!("   4. Run this example again");
                }
                a_run::ModelEnvironment::Ollama => {
                    println!("   1. Run: ollama pull qwen:1.8b");
                    println!("   2. Run this example again");
                }
                a_run::ModelEnvironment::LocalAI => {
                    println!("   1. Download GGUF model from HuggingFace");
                    println!("   2. Place in ./models/");
                    println!("   3. Run this example again");
                }
                a_run::ModelEnvironment::CustomPath => {
                    println!("   1. Download GGUF model from HuggingFace");
                    println!("   2. Place in your custom directory");
                    println!("   3. Run this example again");
                }
            }
            println!();
        }
    } else {
        println!("\n⚠️  No model loading software detected.\n");
        println!("Aaroneous supports:");
        println!("  • LM Studio (https://lmstudio.ai) - Recommended");
        println!("  • Ollama (https://ollama.ai)");
        println!("  • LocalAI (https://localai.io)\n");
        println!("Install one of these to get started.\n");
    }

    println!("{}", "=".repeat(70));
    println!("\n💡 Next steps:");
    println!("   • Download a model using your software");
    println!("   • Run: cargo run --example discover_models");
    println!("   • Aaroneous will auto-find and use your models\n");

    Ok(())
}
