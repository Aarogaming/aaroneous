// crates/flight_controller/src/main.rs
use anyhow::Result;
use clap::Parser;
use flight_controller::{FlightConfig, FlightEngine};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let config = FlightConfig::parse();
    let engine = FlightEngine::new(config)?;
    engine.run().await
}
