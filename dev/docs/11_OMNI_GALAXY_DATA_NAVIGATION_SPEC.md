# 11: Omni Galaxy — 3D Semantic Data & Visual Navigation System

## Vision: The Galaxy of Star-Nodes

**Omni** is the internal visual data navigation, search, and access engine of the Aaroneous platform. 

Instead of traditional hierarchical folder trees (which mirror arbitrary physical disk layouts rather than intellectual relationships), Omni visualizes the entire system's memory, code, documents, agents, and data as a **3D Galaxy of Star-Nodes**.

```
                           🌌 GALAXY VIEW (Macro Level)
                   ┌─────────────────────────────────────────┐
                   │    ✧    ★ Domain Cluster (Galaxy)   ✧    │
                   │  ★   ✧        ★          ★   ✧      │
                   │      ✧    ★  (Spiral Arms)   ★          │
                   └────────────────────┬────────────────────┘
                                        │ (Zoom / Scroll In)
                                        ▼
                         ⭐ STAR-NODE VIEW (Micro Level)
                   ┌─────────────────────────────────────────┐
                   │  • Minimized Data Node (Pulsing Star)   │
                   │  • 3D Position: (Domain, Temporal, Pri) │
                   │  • Content-based Gravitational Links    │
                   │  • Click to Expand Full Payload / Exec  │
                   └─────────────────────────────────────────┘
```

---

## 🧭 Spatial Coordinate System (The 3D Coordinate Plane)

Every data node in the Omni galaxy is positioned along three fundamental semantic axes:

| Axis | Semantic Meaning | Negative Value Range (-) | Positive Value Range (+) |
| :--- | :--- | :--- | :--- |
| **X Axis (Domain Spectrum)** | Functional / Knowledge Domain | Abstract Theory, Research, Spec (`-1000.0`) | Concrete Execution, Raw Binary, I/O (`+1000.0`) |
| **Y Axis (Temporal Phase)** | State & Lifecycle | Completed / Archived History (`-800.0`) | Active Execution / Future Roadmap (`+800.0`) |
| **Z Axis (Priority & Depth)** | Criticality & Visibility | Hidden / Deep Background Data (`-500.0`) | Critical / Immediate High Priority (`+1000.0`) |

---

## 🌟 Star-Node Architecture

A **Star-Node** represents an atomic unit of data, memory, capability, or code:

```rust
pub struct StarNode {
    pub id: String,
    pub title: String,
    pub node_type: NodeType,          // Feature, Memory, Agent, Code, Bug, Lore, LatentPulse
    pub spatial_coord: SpatialCoord,  // (x, y, z)
    pub latent_vector: [f32; 32],     // Semantic embedding vector for clustering
    pub activity_pulse: f32,          // 0.0 to 1.0 (visual pulse / glow intensity)
    pub gravitational_links: HashMap<String, LinkType>, // Content-based relationships
    pub payload_uri: String,          // Link to actual data, code file, or API trigger
}
```

### Visual Characteristics:
- **Color**: Mapped to `NodeType` (Features: Cyan, Lore/Knowledge: Green, Bugs/Crises: Red, Latent Thoughts: Neon Blue, Specialists: Gold).
- **Size & Brightness**: Proportional to Priority (`Z` coordinate) and access frequency (`activity_pulse`).
- **Pulsing Animation**: When a background specialist (e.g. Synthesizer or Desktop Emulator) accesses or writes to a node, its `activity_pulse` flares up.

---

## 🪐 Galaxies & Clusters (Content-Based Grouping)

In traditional systems, files are grouped by whatever folder path they were saved to on disk. In Omni:
1. **Gravitational Clustering**: Star-Nodes exert semantic gravitational attraction toward each other based on **cosine similarity of their latent vectors** and direct dependency links (`DependsOn`, `Implements`, `Documents`, `RelatesTo`).
2. **Dynamic Galaxies**: When 3 or more related nodes cluster tightly in space, Omni dynamically designates a **Galaxy / Cluster** with a spatial center and gravitational radius.
3. **Zooming as "Opening a Folder"**:
   - At high camera altitude (Galaxy View), clusters appear as single luminous nebulae or spiral galaxies.
   - Scrolling into the galaxy acts like opening a folder: the cluster expands, revealing the constituent star-nodes and their interconnecting laser links.
   - The contents are **content-based relationships**, meaning a document, a Rust function, a NATS event stream, and a model weight file will sit in the same galaxy if they share conceptual gravity.

---

## 🔍 Omni Multi-Dimensional Search & Filtering

Omni acts as the unified search engine across all internal and wrapped data:

1. **Spatial Bounding Frustum**: Query nodes within a 3D visual bounding box.
2. **Semantic Proximity Search**: Provide a natural language query ➔ Synthesizer embeds it into a latent vector ➔ Omni highlights the nearest star-nodes in Euclidean 3D space.
3. **Tag & Domain Filtering**: Toggle filter tags to dim or illuminate entire galactic sectors.
4. **Temporal Horizon Slider**: Scrubbing the timeline shifts the `Y` plane to inspect historical snapshots or future planned capabilities.
