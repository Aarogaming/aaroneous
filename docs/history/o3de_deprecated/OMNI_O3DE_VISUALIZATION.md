# Omni Constellation Visualization in O3DE

The Omni Constellation is the "living memory" of the Aaroneous system. It is a multi-dimensional knowledge graph embedded directly into the GGUF model files. Instead of traditional flat databases, knowledge is grouped by semantic relativity ("specks in a solar system").

This guide outlines how to visualize the Omni Constellation within the **O3DE (Open 3D Engine)** environment using the new `/omni/constellation` API endpoint.

## 1. The API Endpoint

Aaroneous exposes the Omni Constellation via a REST API:
```
GET /omni/constellation?model={model_name.gguf}
```

### Response Format
The response contains the entire structural layout of the knowledge graph:
```json
{
  "ok": true,
  "constellation": {
    "name": "hive-master",
    "dimensions": 256,
    "nodes": {
      "omni-fabrication-18-0": {
        "id": "omni-fabrication-18-0",
        "title": "Deployment logic",
        "domain": "fabrication",
        "content": "...",
        "coordinates": {
          "dimensions": [0.1, 0.5, 0.0, ...]
        },
        "mass": 5.0,
        "links": {
          "omni-security-24-1": 0.15
        }
      }
    }
  }
}
```

## 2. O3DE Gem Architecture for Visualization

To bring the Omni Constellation to life in an immersive 3D environment, the Aaroneous O3DE Gem should implement a **Constellation Manager Component**.

### A. Polling / Loading the Constellation
When the O3DE scene loads, the Constellation Manager makes an HTTP GET request to `/omni/constellation`.
It parses the JSON response to extract all `nodes` and their respective `links`.

### B. Spatial Mapping (Dimensionality Reduction)
The Omni vector dimensions (e.g., 256 or 4096 dimensions) must be mapped into 3D space.
There are a few approaches to calculate the 3D position (X, Y, Z) in O3DE:
1. **Direct Mapping:** Map specific vector indices to X, Y, and Z (e.g., `dim[0] = X`, `dim[5] = Y`, `dim[15] = Z`). This is useful if the dimensions represent specific semantic concepts (like "Theory vs Execution").
2. **Force-Directed Graph (Preferred):** Use the `links` data! Since `links` define the relativistic distance between nodes, you can run a real-time physics simulation in O3DE. Treat nodes as repelling particles, and links as springs pulling them together based on their distance coefficient. 
3. **PCA / t-SNE:** Run dimensionality reduction on the vector dimensions to compress them down to 3D space.

### C. Visualizing Nodes (The "Specks")
For each node, spawn an O3DE Entity:
- **Mesh Component:** A Sphere or a glowing particle.
- **Material Component:** 
  - **Scale (Size):** Map the node's `mass` to the Entity's physical scale. Higher mass = bigger, more important speck.
  - **Color/Emission:** Map the `domain` (e.g., "fabrication", "security") to specific colors (e.g., Fabrication = Orange, Security = Blue).
- **Text/UI Component (LyShine):** A floating billboard displaying the `title` of the node when the camera gets close.

### D. Visualizing Links (The "Orbits")
For each link listed in a node's `links` map:
- Spawn a **Line/Trail Component** connecting the source Entity to the target Entity.
- Adjust the line's opacity or thickness based on the relativistic distance (smaller distance = tighter link = brighter line).

## 3. Interactive Experience
Because the Omni is a living artifact:
- **Hot Reloading:** The O3DE Gem can periodically poll `/omni/constellation` or listen to SSE events. When the Omni Relic agent injects new knowledge, O3DE can spawn a new sphere dynamically and animate it falling into orbit around its semantic peers.
- **VR/AR Exploration:** If viewed in VR (using the Phygital specialist integration), users can physically "fly" through the constellation, grabbing nodes to inspect their internal `content`.

This turns Aaroneous from a text-based AI into an interactive, spatial knowledge intelligence system.