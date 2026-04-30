// Aaroneous CLI Binary Entry Point

use a_run::cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::parse_args();
    cli::execute(args).await
}
