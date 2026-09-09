//! Observability Integration Example
use std::sync::atomic::Ordering;

fn main() -> Result<(), String> {
    println!("Initializing Observability Pipeline...");
    
    // Initialize all telemetry sources
    let dxgi_hook = a_run::native_ingestion::dxgi_swapchain::DxgiSwapchainHook::init()?;
    dxgi_hook.start_capture()?;
    
    let etw_config = a_run::native_ingestion::etw_kernel_trace::EtwKernelConfig::default();
    let etw_trace = a_run::native_ingestion::etw_kernel_trace::EtwKernelTrace::init(&etw_config)?;
    etw_trace.start_capture()?;
    
    let rgb_config = a_run::native_ingestion::rgb_telemetry::RgbTelemetryConfig::default();
    let rgb_reader = a_run::native_ingestion::rgb_telemetry::RgbTelemetryReader::init(&rgb_config)?;
    rgb_reader.start_sync()?;

    let aggregator = a_run::native_ingestion::observability_aggregator::ObservabilityAggregator::init()?;
    aggregator.start()?;

    println!("Observability pipeline active!");
    
    // Simulate telemetry events
    for _ in 0..5 {
        let metrics = a_run::native_ingestion::dxgi_swapchain::DxgiCaptureMetrics::new(60, &[5000u64; 60]);
        aggregator.on_dxgi_frame(&metrics);
        
        aggregator.on_etw_event(5, "telemetry");
        
        let rgb_metrics = a_run::native_ingestion::rgb_telemetry::RgbTelemetryMetrics {
            updates_per_second: 60.0,
            avg_update_latency_us: 5.0,
            max_temperature_celsius: Some(45.0),
            min_clock_speed_mhz: Some(1200),
        };
        aggregator.on_rgb_update(&rgb_metrics);
        
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    dxgi_hook.stop_capture();
    etw_trace.stop_capture();
    rgb_reader.stop_sync();
    aggregator.stop();

    println!("Pipeline stopped. Events: {}", aggregator.event_counter.load(Ordering::SeqCst));
    Ok(())
}
