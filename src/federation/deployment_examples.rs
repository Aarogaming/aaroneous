/// Deployment Examples: Real-world deployment scenarios
/// 
/// Shows how to use the bootstrap system for different targets:
/// - Mobile deployment (iOS/Android)
/// - Desktop deployment (full featured)
/// - Server deployment (headless)
/// - Custom deployments

#[cfg(test)]
mod examples {
    use crate::federation::bootstrap::{
        BootstrapSystem, DeploymentTarget, SpecialistModule, DeploymentConfig,
    };

    /// Example 1: Fresh installation on desktop
    /// 
    /// `aaroneous --init`
    /// Install Sentinel core only (2GB)
    #[test]
    fn example_desktop_fresh_install() {
        println!("\n=== Example 1: Fresh Desktop Installation ===");

        let result = BootstrapSystem::init();

        println!("Success: {}", result.success);
        println!("Message: {}", result.message);
        println!("Size: {}MB", result.size_mb);
        println!("Modules: {:?}", result.modules_installed);

        assert!(result.success);
        assert!(result.manifest.is_some());
    }

    /// Example 2: Expand desktop with design specialist
    /// 
    /// `aaroneous --expand --include visionary`
    /// Adds Visionary (1GB) to existing Sentinel
    #[test]
    fn example_expand_add_visionary() {
        println!("\n=== Example 2: Expand Desktop - Add Visionary ===");

        // Start with core
        let mut manifest = crate::federation::bootstrap::Manifest::new(DeploymentTarget::Desktop);
        println!("Initial modules: {}", manifest.modules.len());

        // Expand
        let result = BootstrapSystem::expand(manifest, vec!["visionary"]);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();

        println!("Success: {}", bootstrap_result.success);
        println!("Total size: {}MB", bootstrap_result.size_mb);
        println!("Modules installed: {}", bootstrap_result.modules_installed);

        // Now has: Sentinel (2GB) + Visionary (1GB) = 3GB
        assert!(bootstrap_result.size_mb > 2000);
    }

    /// Example 3: Mobile deployment with essential modules
    /// 
    /// `aaroneous --portable --target mobile`
    /// Sentinel + Omnipresent + Symbiotic (1.5GB)
    #[test]
    fn example_mobile_deployment() {
        println!("\n=== Example 3: Mobile Deployment ===");

        let result = BootstrapSystem::portable(DeploymentTarget::Mobile);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();

        println!("Target: Mobile");
        println!("Success: {}", bootstrap_result.success);
        println!("Size: {}MB / 1500MB max", bootstrap_result.size_mb);
        println!("Modules: {}", bootstrap_result.modules_installed);
        println!("Message: {}", bootstrap_result.message);

        // Mobile max is 3500MB
        assert!(bootstrap_result.size_mb <= 3500);
        
        // Should have: Sentinel, Omnipresent, Symbiotic
        if let Some(manifest) = &bootstrap_result.manifest {
            assert!(manifest.modules.contains(&SpecialistModule::Sentinel));
            assert!(manifest.modules.contains(&SpecialistModule::Omnipresent));
            assert!(manifest.modules.contains(&SpecialistModule::Symbiotic));
            // Should NOT have Phygital (that's tablet+)
            assert!(!manifest.modules.contains(&SpecialistModule::Phygital));
        }
    }

    /// Example 4: Tablet deployment with AR support
    /// 
    /// `aaroneous --portable --target tablet`
    /// Sentinel + Omnipresent + Symbiotic + Phygital (2GB)
    #[test]
    fn example_tablet_deployment() {
        println!("\n=== Example 4: Tablet Deployment (iPad with AR) ===");

        let result = BootstrapSystem::portable(DeploymentTarget::Tablet);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();

        println!("Target: Tablet");
        println!("Success: {}", bootstrap_result.success);
        println!("Size: {}MB / 2000MB max", bootstrap_result.size_mb);
        println!("Modules: {}", bootstrap_result.modules_installed);

        // Tablet max is 4500MB
        assert!(bootstrap_result.size_mb <= 4500);

        // Should have Phygital (AR support)
        if let Some(manifest) = &bootstrap_result.manifest {
            assert!(manifest.modules.contains(&SpecialistModule::Phygital));
        }
    }

    /// Example 5: Full desktop deployment with all specialists
    /// 
    /// `aaroneous --portable --target desktop`
    /// All 6 specialists (4GB)
    #[test]
    fn example_desktop_full_deployment() {
        println!("\n=== Example 5: Desktop Full Deployment ===");

        let result = BootstrapSystem::portable(DeploymentTarget::Desktop);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();

        println!("Target: Desktop");
        println!("Success: {}", bootstrap_result.success);
        println!("Size: {}MB / 4000MB max", bootstrap_result.size_mb);
        println!("Modules: {}", bootstrap_result.modules_installed);
        println!("Message: {}", bootstrap_result.message);

        // Desktop max is 6000MB
        assert!(bootstrap_result.size_mb <= 6000);

        // Should have all 6 specialists
        if let Some(manifest) = &bootstrap_result.manifest {
            assert_eq!(manifest.modules.len(), 6);
            assert!(manifest.modules.contains(&SpecialistModule::Sentinel));
            assert!(manifest.modules.contains(&SpecialistModule::Visionary));
            assert!(manifest.modules.contains(&SpecialistModule::Omnipresent));
            assert!(manifest.modules.contains(&SpecialistModule::Symbiotic));
            assert!(manifest.modules.contains(&SpecialistModule::Phygital));
            assert!(manifest.modules.contains(&SpecialistModule::Archivist));
        }
    }

    /// Example 6: Server deployment (headless, minimal)
    /// 
    /// `aaroneous --portable --target server`
    /// Sentinel only (500MB) - for orchestration/backend
    #[test]
    fn example_server_deployment() {
        println!("\n=== Example 6: Server Deployment (Headless) ===");

        let result = BootstrapSystem::portable(DeploymentTarget::Server);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();

        println!("Target: Server");
        println!("Success: {}", bootstrap_result.success);
        println!("Size: {}MB / 500MB max", bootstrap_result.size_mb);
        println!("Modules: {}", bootstrap_result.modules_installed);

        // Server max is 8000MB
        assert!(bootstrap_result.size_mb <= 8000);

        // Should have only Sentinel
        if let Some(manifest) = &bootstrap_result.manifest {
            assert!(manifest.modules.contains(&SpecialistModule::Sentinel));
            assert_eq!(manifest.modules.len(), 1);
        }
    }

    /// Example 7: Generate deployment configuration for CI/CD
    /// 
    /// Create TOML config for Docker/Kubernetes
    #[test]
    fn example_generate_deployment_config() {
        println!("\n=== Example 7: Generate Deployment Config ===");

        let config = BootstrapSystem::generate_config(DeploymentTarget::Desktop);

        println!("Target: {:?}", config.manifest.target);
        println!("Modules: {}", config.manifest.modules.len());
        println!("DNA Bank: {}", config.dna_bank_path);
        println!("Model Cache: {}", config.model_cache_path);
        println!("Log Level: {}", config.log_level);
        println!("Metrics Enabled: {}", config.enable_metrics);
        println!("Learning Enabled: {}", config.enable_learning);

        // Generate TOML
        let toml_result = config.to_toml();
        assert!(toml_result.is_ok());

        let toml = toml_result.unwrap();
        println!("\n--- Generated TOML ---");
        println!("{}", toml);

        // Verify TOML contains expected sections
        assert!(toml.contains("[deployment]"));
        assert!(toml.contains("[modules]"));
        assert!(toml.contains("[paths]"));
        assert!(toml.contains("[logging]"));
        assert!(toml.contains("[features]"));
    }

    /// Example 8: Custom configuration with paths
    /// 
    /// Configure custom DNA Bank and model cache locations
    #[test]
    fn example_custom_configuration() {
        println!("\n=== Example 8: Custom Configuration ===");

        let mut config = DeploymentConfig::new(DeploymentTarget::Desktop)
            .with_dna_path("/var/lib/aaroneous/dna_bank")
            .with_log_level("debug");

        println!("DNA Bank Path: {}", config.dna_bank_path);
        println!("Log Level: {}", config.log_level);

        // Disable learning for server
        config.enable_learning = false;

        println!("Learning Enabled: {}", config.enable_learning);

        assert_eq!(config.dna_bank_path, "/var/lib/aaroneous/dna_bank");
        assert_eq!(config.log_level, "debug");
        assert!(!config.enable_learning);
    }

    /// Example 9: Progressive expansion workflow
    /// 
    /// Start minimal, add specialists as needed
    #[test]
    fn example_progressive_expansion() {
        println!("\n=== Example 9: Progressive Expansion Workflow ===");

        // Start with server (Sentinel only)
        let result = BootstrapSystem::portable(DeploymentTarget::Server);
        assert!(result.is_ok());
        println!("Step 1: Server (Sentinel) - {}MB", result.unwrap().size_mb);

        // Add Omnipresent for multi-device
        let mut manifest =
            crate::federation::bootstrap::Manifest::new(DeploymentTarget::Desktop);
        let _ = manifest.remove_module(&SpecialistModule::Visionary);
        let _ = manifest.remove_module(&SpecialistModule::Phygital);
        let _ = manifest.remove_module(&SpecialistModule::Archivist);

        println!(
            "Step 2: Add Omnipresent - {}MB",
            manifest.total_size_mb()
        );

        // Add Symbiotic for biometrics
        let result = BootstrapSystem::expand(manifest, vec!["symbiotic"]);
        assert!(result.is_ok());
        println!(
            "Step 3: Add Symbiotic - {}MB",
            result.unwrap().size_mb
        );

        // Add Visionary for design
        let mut manifest2 =
            crate::federation::bootstrap::Manifest::new(DeploymentTarget::Desktop);
        let result = BootstrapSystem::expand(manifest2, vec!["visionary"]);
        assert!(result.is_ok());
        println!(
            "Step 4: Add Visionary - {}MB",
            result.unwrap().size_mb
        );
    }

    /// Example 10: Verify portable versions can coexist on same system
    /// 
    /// Multiple deployments with different targets
    #[test]
    fn example_multiple_deployments() {
        println!("\n=== Example 10: Multiple Deployments on Same System ===");

        let mobile = BootstrapSystem::portable(DeploymentTarget::Mobile).unwrap();
        let tablet = BootstrapSystem::portable(DeploymentTarget::Tablet).unwrap();
        let desktop = BootstrapSystem::portable(DeploymentTarget::Desktop).unwrap();

        println!("Mobile:  {}MB", mobile.size_mb);
        println!("Tablet:  {}MB", tablet.size_mb);
        println!("Desktop: {}MB", desktop.size_mb);

        println!(
            "Total footprint: {}MB (if all installed)",
            mobile.size_mb + tablet.size_mb + desktop.size_mb
        );

        // Each within constraints
        assert!(mobile.size_mb <= 3500);
        assert!(tablet.size_mb <= 4500);
        assert!(desktop.size_mb <= 6000);

        // Progressive capability increase
        assert!(mobile.modules_installed < tablet.modules_installed);
        assert!(tablet.modules_installed < desktop.modules_installed);
    }
}

#[cfg(test)]
mod docker_examples {
    use crate::federation::bootstrap::BootstrapSystem;
    use crate::federation::bootstrap::DeploymentTarget;

    /// Docker Compose example: Multi-container deployment
    #[test]
    fn example_docker_compose() {
        println!("\n=== Example: Docker Compose Deployment ===");

        let config = BootstrapSystem::generate_config(DeploymentTarget::Desktop);
        let toml = config.to_toml().unwrap();

        let docker_compose = r#"
version: '3.8'

services:
  aaroneous:
    image: aaroneous:latest
    volumes:
      - aaroneous-dna:/var/lib/aaroneous/dna_bank
      - aaroneous-models:/var/cache/aaroneous/models
    environment:
      - AARONEOUS_LOG_LEVEL=info
      - AARONEOUS_ENABLE_METRICS=true
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  aaroneous-dna:
  aaroneous-models:
"#;

        println!("{}", docker_compose);
        println!("\nDeployment config (TOML):\n{}", toml);
    }

    /// Kubernetes deployment example
    #[test]
    fn example_kubernetes_deployment() {
        println!("\n=== Example: Kubernetes Deployment ===");

        let config = BootstrapSystem::generate_config(DeploymentTarget::Desktop);

        let k8s_manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aaroneous
  labels:
    app: aaroneous
spec:
  replicas: 3
  selector:
    matchLabels:
      app: aaroneous
  template:
    metadata:
      labels:
        app: aaroneous
    spec:
      containers:
      - name: aaroneous
        image: aaroneous:latest
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "8Gi"
            cpu: "4"
        volumeMounts:
        - name: dna-bank
          mountPath: /var/lib/aaroneous/dna_bank
        - name: model-cache
          mountPath: /var/cache/aaroneous/models
      volumes:
      - name: dna-bank
        persistentVolumeClaim:
          claimName: aaroneous-dna
      - name: model-cache
        persistentVolumeClaim:
          claimName: aaroneous-models
"#;

        println!("{}", k8s_manifest);
    }
}
