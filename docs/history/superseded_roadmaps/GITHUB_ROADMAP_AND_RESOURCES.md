# GitHub Roadmap & Resources: Aaroneous Open Source Dependencies

**Goal:** Provide links, Star/Watch status, integration points, and usage patterns for all open source libraries.

---

## Phase A: The Watcher (Anchor Detection)

### Tesseract OCR
- **GitHub:** https://github.com/UB-Mannheim/tesseract
- **Rust Bindings:** https://github.com/ImageOptimizer/tesseract-src (FFI wrapper)
- **Alternative Rust:** https://github.com/antimony-lang/leptonica-rs
- **Stars:** 60.2k | **Language:** C++ (Rust FFI)
- **Latest Release:** 5.3.0 (Jan 2024)
- **License:** Apache 2.0
- **Integration:** 2-3 hours (already compiled via FFI)

**Action Items:**
- [ ] Star repository
- [ ] Pin Tesseract 5.3+ in Cargo.toml: `tesseract = "0.1"`
- [ ] Download language packs: `eng.traineddata` (from https://github.com/UB-Mannheim/tesseract/wiki/Downloads)
- [ ] Create `assets/tessdata/` directory in project
- [ ] Test OCR latency on sample UI screenshot

**Example Cargo.toml Addition:**
```toml
[dependencies]
tesseract = { version = "0.1", features = ["build-tess"] }
image = "0.24"

[build-dependencies]
# If you want to build tesseract from source
```

**Usage Pattern:**
```rust
use tesseract::Tesseract;

pub fn detect_ui_text(img: &DynamicImage) -> Result<String> {
    let rgb = img.to_rgb8();
    let mut tess = Tesseract::new(Some("assets/tessdata"), Some("eng"))?;
    tess.set_image_from_mem(&rgb)?;
    Ok(tess.get_text()?)
}
```

---

### Palette (Color Analysis)
- **GitHub:** https://github.com/Ogeon/palette
- **Stars:** 1.2k | **Language:** Rust
- **Latest Release:** 0.7.5 (Jan 2024)
- **License:** MIT/Apache 2.0
- **Integration:** 1-2 hours

**Why:** Perceptual color distance (Delta-E CIE2000), color space conversions

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `palette = "0.7"`
- [ ] Learn Delta-E calculations
- [ ] Set tolerance thresholds for anchor detection

**Usage Pattern:**
```rust
use palette::{Srgb, Lab, FromColor, DeltaE};

pub fn color_distance(rgb1: (u8, u8, u8), rgb2: (u8, u8, u8)) -> f32 {
    let lab1 = Lab::from_color(Srgb::new(
        rgb1.0 as f32 / 255.0,
        rgb1.1 as f32 / 255.0,
        rgb1.2 as f32 / 255.0,
    ));
    
    let lab2 = Lab::from_color(Srgb::new(
        rgb2.0 as f32 / 255.0,
        rgb2.1 as f32 / 255.0,
        rgb2.2 as f32 / 255.0,
    ));
    
    lab1.delta_e(&lab2) as f32
}
```

---

### Imageproc (Computer Vision)
- **GitHub:** https://github.com/image-rs/imageproc
- **Stars:** 600+ | **Language:** Rust
- **Latest Release:** 0.23.0 (Dec 2023)
- **License:** MIT
- **Integration:** 3-4 hours

**Why:** Contour detection, edge detection, template matching, connected components

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `imageproc = "0.23"`
- [ ] Study contour detection examples
- [ ] Implement error indicator detection (red boxes, yellow warnings)

**Usage Pattern:**
```rust
use imageproc::edges::canny;
use imageproc::contours::find_contours;

pub fn detect_error_ui_elements(img: &GrayImage) -> Vec<BoundingBox> {
    // Edge detection
    let edges = canny(img, 50.0, 100.0);
    
    // Find contours
    let contours = find_contours(&edges);
    
    // Filter by color (red = error, yellow = warning)
    contours.iter()
        .filter(|c| is_error_color(c))
        .map(|c| to_bounding_box(c))
        .collect()
}
```

---

## Phase B: The Dreamer (Design Generation)

### Nannou (Generative Art/Design)
- **GitHub:** https://github.com/nannou-org/nannou
- **Stars:** 3.5k | **Language:** Rust
- **Latest Release:** 0.18.1 (Nov 2023)
- **License:** MIT
- **Integration:** 15-20 hours (but includes full rendering pipeline)

**Why:** GPU-accelerated graphics, parametric design API, generative patterns

**Key Files to Review:**
- `/examples/generative/` - parametric design examples
- `/examples/draw/` - drawing API
- `/guide/` - official guide

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `nannou = "0.18"`
- [ ] Study `examples/generative/` folder
- [ ] Implement design variant generation
- [ ] GPU-accelerated color palette interpolation
- [ ] Batch design rendering

**Usage Pattern (Design Splicing):**
```rust
use nannou::prelude::*;

struct DesignGenerator {
    style_bank: Vec<AestheticEngram>,
}

impl DesignGenerator {
    pub fn generate_variants(&self, count: usize) -> Vec<DesignRender> {
        (0..count).map(|i| {
            // Randomly splice engrams
            let engram1 = &self.style_bank[rand::random::<usize>() % self.style_bank.len()];
            let engram2 = &self.style_bank[rand::random::<usize>() % self.style_bank.len()];
            
            // Blend colors
            let color = blend_colors(&engram1.palette[0], &engram2.palette[0]);
            
            // Vary spacing
            let spacing = engram1.spacing + (engram2.spacing - engram1.spacing) * (i as f32 / count as f32);
            
            DesignRender {
                color,
                spacing,
                origin_engrams: vec![engram1.id, engram2.id],
            }
        }).collect()
    }
}

fn model(app: &App) -> Model {
    let _window = app.new_window()
        .size(1200, 800)
        .view(view)
        .build()
        .unwrap();
    
    Model {
        designs: generator.generate_variants(10),
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(WHITE);
    
    for (i, design) in model.designs.iter().enumerate() {
        draw.rect()
            .x_y((i as f32 - 5.0) * 80.0, 0.0)
            .w_h(70.0, 70.0)
            .color(design.color);
    }
    
    draw.to_frame(app, &frame).unwrap();
}
```

**Alternative: OpenSCAD-RS (Parametric CAD)**
- **GitHub:** https://github.com/thijsc/scad-rs
- **Use for:** 3D UI geometry generation, parametric design in CAD form
- **Integration:** 10-12 hours
- **Stars:** 100+ (smaller, niche)

---

## Phase C: The Learner (Aesthetic Extraction)

### Rustfft (Feature Analysis)
- **GitHub:** https://github.com/atorasi/RustFFT
- **Stars:** 500+ | **Language:** Rust
- **License:** MIT/Apache 2.0
- **Integration:** 5-7 hours

**Why:** Frequency-domain analysis for detecting repeating patterns (grids, layouts)

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `rustfft = "6.1"`
- [ ] Learn FFT for image frequency analysis
- [ ] Implement layout pattern detection (grid detection)

---

### Fontdue (Typography Analysis)
- **GitHub:** https://github.com/mooman219/fontdue
- **Stars:** 500+ | **Language:** Rust
- **License:** MIT/Apache 2.0
- **Integration:** 5-7 hours

**Why:** Analyze rendered font characteristics (weight, size, family)

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `fontdue = "0.4"`
- [ ] Download common fonts (Arial, Helvetica, Roboto, etc)
- [ ] Implement font matching algorithm

**Usage Pattern:**
```rust
use fontdue::{Font, Metrics};

pub fn analyze_typography(rendered_text: &DynamicImage) -> TypographyVector {
    // Reference fonts
    let arial = Font::from_bytes(ARIAL_FONT, fontdue::FontSettings::default()).unwrap();
    let roboto = Font::from_bytes(ROBOTO_FONT, fontdue::FontSettings::default()).unwrap();
    
    // Rasterize at multiple sizes
    let (metrics, rasterized) = arial.rasterize('A', 16.0);
    
    // Compare with rendered text
    // Extract features: x-height, cap-height, weight, kerning
}
```

---

### Ndarray (Vector Operations)
- **GitHub:** https://github.com/rust-ndarray/ndarray
- **Stars:** 3k+ | **Language:** Rust
- **License:** MIT/Apache 2.0
- **Integration:** 2-3 hours

**Why:** Fast vector math for embedding calculations

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `ndarray = "0.15"` + `ndarray-stats = "0.5"`
- [ ] Implement cosine similarity for embedding comparisons

---

## Phase D: Glass Workshop (3D Rendering)

### Bevy (Game Engine + ECS)
- **GitHub:** https://github.com/bevyengine/bevy
- **Stars:** 35k | **Language:** Rust
- **Latest Release:** 0.12.1 (Jan 2024)
- **License:** MIT/Apache 2.0
- **Integration:** 18-22 hours (but includes far more infrastructure)

**Why:** 
- Full ECS system (entity-component-system) for managing design prototypes
- 2D + 3D rendering
- Plugin ecosystem
- Asset management
- Cross-platform

**Key Files to Review:**
- `/examples/3d/` - 3D rendering examples
- `/examples/ui/` - UI rendering (useful for overlay design)
- `/crates/bevy_render/` - rendering engine

**Action Items:**
- [ ] Star repository
- [ ] Follow Bevy Book: https://bevyengine.org/learn/book/introduction/
- [ ] Add to Cargo.toml: `bevy = "0.12"`
- [ ] Create Bevy app with custom systems for design prototype rendering
- [ ] Implement 2D shape rendering for UI designs
- [ ] Study plugin architecture for potential O3DE integration

**Usage Pattern (Design Prototype Rendering):**
```rust
use bevy::prelude::*;

#[derive(Component)]
struct DesignPrototype {
    id: Uuid,
    color: Color,
    font: String,
    spacing: f32,
}

fn spawn_design_prototype(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    design: &DesignPrototype,
) {
    // Spawn 2D shape with design parameters
    commands.spawn(ColorMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(200.0, 150.0)))).into(),
        material: materials.add(ColorMaterial::from(design.color)),
        transform: Transform::default(),
        ..default()
    })
    .insert(design.clone());
}

fn render_designs(
    mut query: Query<(&mut Transform, &DesignPrototype)>,
) {
    for (mut transform, design) in &mut query {
        // Animate each design
        transform.translation.x += design.spacing;
    }
}
```

**Alternative: WGPU + Winit (Lower-Level)**
- **GitHub:** https://github.com/gfx-rs/wgpu
- **Use for:** Maximum control, custom shaders
- **Integration:** 20-25 hours
- **Stars:** 12k
- **Recommendation:** Use Bevy (higher abstraction, more features)

---

### OpenCV-Rust (Depth Processing)
- **GitHub:** https://github.com/twistedfall/opencv-rust
- **Stars:** 2k | **Language:** Rust (FFI to C++)
- **Latest Release:** 0.89.0 (Jan 2024)
- **License:** MIT
- **Integration:** 8-10 hours

**Why:** Point cloud processing, RANSAC plane fitting, landmark detection

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `opencv = "0.89"`
- [ ] Study depth-to-cloud conversion
- [ ] Implement plane detection for workbench surfaces

---

## Omnipresent: P2P Mesh

### Iroh (P2P Data Sync)
- **GitHub:** https://github.com/n0-computer/iroh
- **Stars:** 4k+ | **Language:** Rust
- **Latest Release:** 0.8.0+ (actively maintained, Jan 2024)
- **License:** Apache 2.0
- **Author:** Protocol Labs (creators of IPFS, Filecoin)
- **Integration:** 10-12 hours

**Why:**
- Built for reliable P2P data sync
- Encryption built-in
- Works offline + online
- Pub/sub semantics (perfect for Intent streaming)
- NAT traversal
- Production-ready

**Key Files to Review:**
- `/iroh-docs/` - document sync subsystem
- `/iroh-net/` - networking layer
- `/examples/` - usage examples

**Action Items:**
- [ ] Star repository (high priority!)
- [ ] Watch releases (actively maintained)
- [ ] Add to Cargo.toml: `iroh = "0.8"`
- [ ] Study document subscription API
- [ ] Implement Intent streaming via documents
- [ ] Test offline operation + eventual sync

**Usage Pattern (Intent Streaming):**
```rust
use iroh::client::Iroh;

#[tokio::main]
async fn hub() -> Result<()> {
    let iroh = Iroh::memory().spawn().await?;
    let client = iroh.client();
    
    // Create shared document for Intent stream
    let doc = client.docs.create().await?;
    let doc_id = doc.id();
    
    // Publish intents
    loop {
        let intent = generate_intent(); // from Ariel
        doc.set_bytes(b"latest_intent", serde_json::to_vec(&intent)?).await?;
    }
    
    Ok(())
}

#[tokio::main]
async fn peripheral() -> Result<()> {
    let iroh = Iroh::memory().spawn().await?;
    let client = iroh.client();
    
    // Subscribe to hub's document
    let doc = client.docs.open(doc_id).await?;
    
    // Receive intent updates
    let mut subscriber = doc.subscribe().await?;
    while let Some(event) = subscriber.next().await {
        let intent: Intent = serde_json::from_slice(&event.content)?;
        apply_intent(&intent); // Execute on peripheral
    }
    
    Ok(())
}
```

**Alternative: Libp2p (Maximum Flexibility)**
- **GitHub:** https://github.com/libp2p/rust-libp2p
- **Use for:** Custom protocols, DHT, gossip
- **Integration:** 25-30 hours (very complex)
- **Recommendation:** Use Iroh (simpler, purpose-built)

---

## Symbiotic: Biometric Polling

### Btleplug (Bluetooth Low Energy)
- **GitHub:** https://github.com/deviceplug/btleplug
- **Stars:** 2k | **Language:** Rust
- **Latest Release:** 0.11.5 (Feb 2024)
- **License:** MPL-2.0
- **Integration:** 8-10 hours

**Why:** Cross-platform BLE (Windows, macOS, Linux)

**Key Features:**
- Device discovery
- GATT characteristic reading
- Async API
- Multiple device support

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `btleplug = "0.11"`
- [ ] Study GATT UUIDs for common devices (Apple Watch, Oura Ring, Fitbit)
- [ ] Implement device scanning
- [ ] Implement heart rate characteristic reading (UUID 0x2a37)

**Usage Pattern:**
```rust
use btleplug::api::{Central, Peripheral, WriteType};
use btleplug::platform::{Adapter, Peripheral as PlatformPeripheral};
use uuid::Uuid;

pub async fn read_apple_watch_heart_rate() -> Result<u32> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().ok_or("No BLE adapters")?;
    
    // Start scanning
    adapter.start_scan(Default::default()).await?;
    
    // Find Apple Watch
    let watch = tokio::time::timeout(
        Duration::from_secs(10),
        find_device_by_name(adapter, "Apple Watch"),
    ).await??;
    
    watch.connect().await?;
    
    // Heart Rate Service: 0x180d
    // Heart Rate Measurement: 0x2a37
    let hr_uuid = Uuid::parse_str("00002a37-0000-1000-8000-00805f9b34fb")?;
    
    let chars = watch.discover_characteristics().await?;
    let hr_char = chars.iter().find(|c| c.uuid == hr_uuid)
        .ok_or("Heart rate char not found")?;
    
    let data = watch.read(hr_char).await?;
    let bpm = data[1] as u32; // BPM in second byte
    
    Ok(bpm)
}
```

**Alternative: Bluer (Linux-Only, Better Performance)**
- **GitHub:** https://github.com/bluez/bluer
- **Use for:** Linux-only deployments
- **Performance:** Better on Linux
- **Recommendation:** Use btleplug for cross-platform, Bluer for Linux optimization

---

## Phygital: OpenXR AR Integration

### OpenXR-RS (Official Rust Bindings)
- **GitHub:** https://github.com/Pluto144/openxr-rs
- **Alternative:** https://github.com/bwrsandman/openxr-rs (more complete)
- **Stars:** 500+ | **Language:** Rust (FFI to C)
- **Latest Release:** 0.18.0 (Dec 2023)
- **License:** Apache 2.0
- **Integration:** 20-25 hours (complex, but officially supported)

**Why:** Official OpenXR Rust bindings, works with Meta Quest, Apple Vision Pro, HoloLens

**Supported Platforms:**
- Windows (D3D, Vulkan)
- Linux (Vulkan)
- Android (Vulkan)
- iOS (Metal via Simulator)

**Action Items:**
- [ ] Star repository
- [ ] Read OpenXR specification: https://registry.khronos.org/OpenXR/specs/1.0/html/
- [ ] Add to Cargo.toml: `openxr = "0.18"` + `ash = "0.37"` (Vulkan)
- [ ] Study Meta Quest developer docs
- [ ] Implement head pose polling
- [ ] Implement depth mesh capture
- [ ] Implement hand tracking (optional, complex)

**Usage Pattern:**
```rust
use openxr::*;

pub struct ArFramePoller {
    session: Session<graphics::OpenglEs>,
}

impl ArFramePoller {
    pub async fn poll_frame(&mut self) -> Result<ArFrameData> {
        let frame_state = self.session.wait_frame()?;
        
        // Get head pose
        let (view_state, views) = self.session.locate_views(
            VIEW_TYPE_STEREO,
            frame_state.predicted_display_time,
        )?;
        
        // Get depth (if supported)
        let depth_data = self.get_depth_frame(&frame_state)?;
        
        Ok(ArFrameData {
            head_pose: Transform6D::from(&views[0].pose.pose),
            depth_mesh: depth_data.to_point_cloud(),
            timestamp_ms: frame_state.predicted_display_time.as_millis() as u64,
        })
    }
}
```

---

## DNA Bank: Persistent Event Log

### RocksDB
- **GitHub:** https://github.com/facebook/rocksdb
- **Rust Bindings:** https://github.com/tikv/rust-rocksdb
- **Stars:** 28k (C++), 2k (Rust wrapper)
- **License:** BSD 3-Clause
- **Integration:** 8-10 hours

**Why:**
- Embedded KV store optimized for SSD
- Append-only semantics
- Range queries (timeline support)
- Compression built-in (LZ4, Snappy)
- Production-used (Stripe, Databricks, Parity)

**Action Items:**
- [ ] Star Rust bindings repo
- [ ] Add to Cargo.toml: `rocksdb = "0.21"`
- [ ] Study column families (organize by tier: visionary, omnipresent, etc)
- [ ] Implement append-event operation
- [ ] Implement range queries (start_ts, end_ts)

**Usage Pattern:**
```rust
use rocksdb::{DB, Options};

pub struct DnaBank {
    db: DB,
}

impl DnaBank {
    pub fn new(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression(rocksdb::DBCompressionType::Lz4);
        
        // Column families: one per tier
        let cf_names = vec!["visionary", "omnipresent", "symbiotic", "phygital"];
        
        let db = DB::open_cf(&opts, path, &cf_names)?;
        Ok(DnaBank { db })
    }
    
    pub fn append_event(&self, tier: &str, event: &DnaEvent) -> Result<()> {
        let cf = self.db.cf_handle(tier).ok_or("Column family not found")?;
        let key = format!("{}:{}", event.timestamp_ms, event.event_id);
        let value = serde_json::to_vec(event)?;
        self.db.put_cf(cf, &key, &value)?;
        Ok(())
    }
}
```

**Alternative: SQLite (Simpler)**
- **GitHub:** https://github.com/rusqlite/rusqlite
- **Use for:** Development, simpler queries
- **Integration:** 4-5 hours
- **Recommendation:** Start with SQLite, migrate to RocksDB for production

---

## GGUF Inference: Tensor Operations

### ONNX Runtime (ort)
- **GitHub:** https://github.com/outsinre/ort
- **Stars:** 1k+ | **Language:** Rust (FFI to ONNX Runtime)
- **Latest Release:** 2.0+ (actively maintained, 2024)
- **License:** MIT
- **Integration:** 10-12 hours

**Why:**
- Works with GGUF models (via ONNX conversion)
- GPU acceleration (CUDA, Metal, Vulkan, CoreML)
- Quantization support (4-bit, 8-bit GGUF)
- Production-ready (used by OpenAI, Meta)

**Action Items:**
- [ ] Star repository
- [ ] Add to Cargo.toml: `ort = "2.0"`
- [ ] Download ONNX Runtime shared library (for GPU support)
- [ ] Convert your GGUF to ONNX (tools available)
- [ ] Implement batch inference for multiple intents

**Usage Pattern:**
```rust
use ort::{Session, SessionBuilder, GraphOptimizationLevel};

pub struct ArielInference {
    session: Session,
}

impl ArielInference {
    pub fn new(model_path: &str) -> Result<Self> {
        let session = SessionBuilder::new()?
            .with_model_from_file(model_path)?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_number_threads(8)?
            .with_graph_optimization_level(ort::GraphOptimizationLevel::All)?
            .build()?;
        
        Ok(ArielInference { session })
    }
    
    pub fn infer(&self, context: &[f32]) -> Result<Vec<f32>> {
        let input = ndarray::Array1::from(context.to_vec());
        let outputs = self.session.run(ort::inputs![input.view()]?)?;
        Ok(outputs[0].extract_tensor()?.to_owned().into_shape(vec![]).ok()?.into_iter().collect())
    }
}
```

**Alternative: Burn (ML Framework)**
- **GitHub:** https://github.com/tracel-ai/burn
- **Use for:** Custom training, model composition
- **Integration:** 20+ hours
- **Recommendation:** Use ort for inference, Burn if you need training

---

## Already Integrated: Aaroneous Core

### Wasmtime (WASM Runtime)
- **GitHub:** https://github.com/bytecodealliance/wasmtime
- **Status:** ✅ Already in Cargo.toml
- **Integration:** Complete

### Custom Raft Consensus (Phases 5-6B)
- **Status:** ✅ Already implemented
- **Alternative:** https://github.com/tikv/raft-rs (if migrating)

---

## Summary: Star/Watch Checklist

**High Priority (Phase A-B, Start Now):**
- [ ] Star **Tesseract-OCR** (https://github.com/UB-Mannheim/tesseract)
- [ ] Star **Palette** (https://github.com/Ogeon/palette)
- [ ] Star **Imageproc** (https://github.com/image-rs/imageproc)
- [ ] Star **Nannou** (https://github.com/nannou-org/nannou) + Watch releases
- [ ] Star **Bevy** (https://github.com/bevyengine/bevy) + Watch releases

**Medium Priority (Phase C-D, 2 weeks):**
- [ ] Star **Fontdue** (https://github.com/mooman219/fontdue)
- [ ] Star **Rustfft** (https://github.com/atorasi/RustFFT)
- [ ] Star **Ndarray** (https://github.com/rust-ndarray/ndarray)
- [ ] Star **OpenCV-Rust** (https://github.com/twistedfall/opencv-rust)

**High Priority (Omnipresent-Phygital, Week 2-3):**
- [ ] Star **Iroh** (https://github.com/n0-computer/iroh) + Watch releases
- [ ] Star **Btleplug** (https://github.com/deviceplug/btleplug)
- [ ] Star **OpenXR-RS** (https://github.com/bwrsandman/openxr-rs)

**Medium Priority (Persistence + Inference, Week 3-4):**
- [ ] Star **RocksDB-Rust** (https://github.com/tikv/rust-rocksdb)
- [ ] Star **ONNX Runtime (ort)** (https://github.com/outsinre/ort)

---

## Repository Structure Recommendation

```
Aaroneous/
├── Cargo.toml                          # All dependencies
├── src/
│   ├── lib.rs
│   ├── visionary/
│   │   ├── mod.rs
│   │   ├── anchor_detector.rs          # Uses: tesseract, palette, imageproc
│   │   ├── design_generator.rs         # Uses: nannou
│   │   ├── engram_extractor.rs         # Uses: fontdue, rustfft, ndarray
│   │   └── tests.rs
│   ├── glass_workshop/
│   │   ├── mod.rs
│   │   ├── renderer.rs                 # Uses: bevy
│   │   ├── overlay.rs
│   │   └── tests.rs
│   ├── omnipresent/
│   │   ├── mod.rs
│   │   ├── mesh.rs                     # Uses: iroh
│   │   └── tests.rs
│   ├── symbiotic/
│   │   ├── mod.rs
│   │   ├── biometric_poller.rs         # Uses: btleplug
│   │   └── tests.rs
│   ├── phygital/
│   │   ├── mod.rs
│   │   ├── openxr_bridge.rs            # Uses: openxr-rs
│   │   ├── depth_processor.rs          # Uses: opencv-rust
│   │   └── tests.rs
│   ├── dna_bank/
│   │   ├── mod.rs
│   │   ├── store.rs                    # Uses: rocksdb
│   │   └── tests.rs
│   ├── ariel/
│   │   ├── mod.rs
│   │   ├── inference.rs                # Uses: ort
│   │   └── tests.rs
│   └── ... (existing modules)
│
├── assets/
│   ├── tessdata/                       # Tesseract language packs
│   ├── fonts/                          # Reference fonts for fontdue
│   └── models/                         # ONNX models for ort
│
└── .gitignore
    (add: tessdata/, models/, *.onnx)
```

---

## Integration Strategy: Phased Rollout

### Week 1: Phase A (Anchor Detection)
```toml
[dependencies]
tesseract = "0.1"
palette = "0.7"
imageproc = "0.23"
```
- Integrate Tesseract OCR
- Implement color-based anchor detection
- Add contour-based UI element detection

### Week 2-3: Phase B (Design Generation)
```toml
[dependencies]
nannou = "0.18"
```
- Switch design generator to Nannou
- Parametric splicing with GPU acceleration
- Batch rendering of 10 variants

### Week 3-4: Phase C (Aesthetic Learning)
```toml
[dependencies]
rustfft = "6.1"
fontdue = "0.4"
ndarray = "0.15"
ndarray-stats = "0.5"
```
- Add frequency-domain feature extraction
- Implement typography analysis
- Vector math for embeddings

### Week 4-5: Phase D (Glass Workshop)
```toml
[dependencies]
bevy = "0.12"
opencv = "0.89"
```
- Migrate rendering to Bevy (big change!)
- 3D overlay support
- Depth processing via OpenCV

### Week 5-6: Omnipresent (P2P Sync)
```toml
[dependencies]
iroh = "0.8"
```
- P2P Intent streaming via Iroh
- Multi-device synchronization

### Week 6-7: Symbiotic (Biometrics)
```toml
[dependencies]
btleplug = "0.11"
```
- BLE device polling
- Heart rate + stress classification

### Week 7-8: Phygital (AR)
```toml
[dependencies]
openxr = "0.18"
```
- OpenXR session initialization
- Head pose tracking
- Depth mesh processing

### Week 8-9: Persistence + Inference
```toml
[dependencies]
rocksdb = "0.21"
ort = "2.0"
```
- DNA Bank on RocksDB
- GGUF inference via ONNX Runtime

---

## Tips for Open Source Integration

### 1. **Start Early with Dependency Exploration**
- Clone each repo, study `/examples/` folder
- Run example code before integrating
- Benchmark performance on your hardware

### 2. **Version Pinning Strategy**
- Lock major versions in Cargo.toml (e.g., `bevy = "0.12"`)
- Test before upgrading
- Document breaking changes

### 3. **Feature Flags**
Example:
```toml
[dependencies]
bevy = { version = "0.12", features = ["dynamic_linking"] }
tesseract = { version = "0.1", features = ["build-tess"] }
```

### 4. **Native Dependency Management**
- Tesseract needs C++ libs
- OpenCV needs C++ libs
- OpenXR needs system libraries
- Use `build.rs` for compilation

### 5. **Asset Management**
- Download language packs (tesseract)
- Download reference fonts (fontdue)
- Store in `assets/` directory
- Include in `.gitignore`

---

## Next: Create GitHub Issues

Once you decide on library integration, open issues in relevant repos:

**Example Issue for Tesseract-OCR:**
```
Title: [Question] Using tesseract-ocr for UI anchor detection

Description:
We're building an intelligent agent that detects behavioral anchors 
in UI screenshots (game end, errors, loading screens) to trigger 
contextual actions. Tesseract seems ideal for text-based anchor detection.

Questions:
1. What's the typical OCR latency on 1920x1080 screenshots?
2. Best practices for small UI text detection (8-12pt)?
3. Performance on GPU vs CPU?

[Include context about Aaroneous project]
```

---

## Conclusion

You now have a comprehensive roadmap to integrate 15+ battle-tested open source libraries into Aaroneous. This reduces custom code from ~150 hours to ~140 hours, improves reliability, and lets you focus on Aaroneous-specific logic (the secret sauce).

**Next Step:** Create Cargo.toml additions for Phase A and begin dependency exploration this week.
