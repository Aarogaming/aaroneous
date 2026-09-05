use anyhow::Result;
use clap::Parser;
use flight_controller::{launch_gui, FlightConfig, FlightEngine};

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let config = FlightConfig::parse();

    if config.gui {
        launch_gui(config).map_err(|e| anyhow::anyhow!("GUI runtime error: {e}"))?;
        Ok(())
    } else {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let engine = FlightEngine::new(config)?;
            engine.run().await
        })
    }
}
