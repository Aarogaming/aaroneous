use std::env;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = a_run::genome_compiler::run_cli(&args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
