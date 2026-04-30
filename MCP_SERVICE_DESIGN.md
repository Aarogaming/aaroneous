# Universal MCP Service - Architecture & Design

## Overview

Transform Aaroneous MCP from an AAS-specific bridge into a **universalized MCP service** that can be used by any client:
- **OpenCode** - AI coding assistant
- **VS Code Extensions** - IDE integration
- **Claude/ChatGPT** - Direct AI integration
- **Anthropic Claude** - MCP-native
- **Custom Tools** - Any program via HTTP or NATS

## Architecture

### Current (AAS-Biased)
```
AAS (Python)
    ↓
MCP Client (Python-specific)
    ↓
MCP Server (Aaroneous)
```

### Target (Universalized)
```
┌─────────────────────────────────────────────────┐
│         Aaroneous MCP Service                   │
│                                                 │
│  ┌──────────────────────────────────────────┐  │
│  │ Protocol Layer                           │  │
│  │ ├─ MCP Protocol (standard)              │  │
│  │ ├─ HTTP/REST API                        │  │
│  │ ├─ NATS Transport (federation)          │  │
│  │ └─ WebSocket (real-time)                │  │
│  └──────────────────────────────────────────┘  │
│                   ↓                             │
│  ┌──────────────────────────────────────────┐  │
│  │ Capability Management                    │  │
│  │ ├─ Registration & Discovery              │  │
│  │ ├─ Versioning & Compatibility            │  │
│  │ ├─ Documentation & Schemas               │  │
│  │ └─ Billing/Rate Limiting                 │  │
│  └──────────────────────────────────────────┘  │
│                   ↓                             │
│  ┌──────────────────────────────────────────┐  │
│  │ Core Services (Backend)                  │  │
│  │ ├─ Event Log & Tracing                   │  │
│  │ ├─ Consensus & Mutations                 │  │
│  │ ├─ Validation & Repair                   │  │
│  │ ├─ Distillation & Learning               │  │
│  │ └─ Recovery & Resilience                 │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
         ↑            ↑             ↑
         │            │             │
      OpenCode     VS Code       Claude
    Extensions                   Web UI
```

## Transport Protocols

### 1. MCP Protocol (Standard)
- Anthropic's Model Context Protocol
- JSON-RPC 2.0 compatible
- Supports stdio, HTTP, SSE

### 2. HTTP/REST API
```
GET  /api/v1/capabilities      - List capabilities
GET  /api/v1/capabilities/{id} - Get capability details
POST /api/v1/call              - Call a capability
GET  /api/v1/health            - Service health
GET  /api/v1/status            - Federation status
```

### 3. NATS Transport (Internal Federation)
- Inter-service communication
- Event streaming
- Pub/Sub for real-time updates

### 4. WebSocket (Real-time)
- Live capability results
- Event streaming
- Progress tracking for long operations

## Client Types

### 1. OpenCode Integration
```
OpenCode                    Aaroneous MCP
   ↓                             ↓
Uses OpenCode CLI       ←→  HTTP/WebSocket
   ↓                             ↓
Invokes tools          ←→  Tool capabilities
   ↓                             ↓
Streams results        ←→  Real-time updates
```

### 2. VS Code Extension
```
VS Code Extension          Aaroneous MCP
   ↓                             ↓
VS Code MCP Client   ←→  MCP Protocol (stdio/HTTP)
   ↓                             ↓
Command Palette      ←→  Capability Menu
   ↓                             ↓
Status Bar           ←→  Health/Status
```

### 3. Claude Web UI
```
Claude.ai                  Aaroneous MCP
   ↓                             ↓
Claude API ←→ MCP Protocol ←→ MCP Server
   ↓                             ↓
Chat Interface       ←→  Capabilities
   ↓                             ↓
Artifact Viewer      ←→  Results
```

### 4. Direct HTTP Clients
```
curl/Postman               Aaroneous MCP
   ↓                             ↓
HTTP Request         ←→  REST API
   ↓                             ↓
JSON Response        ←→  Capability Result
```

## Capability Categories

### Universal Core Capabilities
```
federation/
├─ healthcheck        - Cluster health status
├─ list_nodes         - Active federation members
├─ system_stats       - Overall system metrics
└─ status_dashboard   - Real-time dashboard data

event_log/
├─ append             - Add event to log
├─ query_by_trace     - Find events by trace
├─ get_stats          - Log statistics
└─ export_events      - Export for analysis

tracing/
├─ emit_span          - Record span
├─ get_trace          - Retrieve trace
├─ list_traces        - Find traces
└─ export_otlp        - OpenTelemetry export
```

### Aaroneous-Specific Capabilities
```
intelligence/
├─ anomaly_detection  - Detect anomalies
├─ forecast           - Predict metrics
├─ auto_scale         - Scaling decisions
├─ self_heal          - Recovery recommendations
└─ optimize           - Optimization suggestions

consensus/
├─ propose_mutation   - Submit change
├─ get_mutation_status - Track mutation
├─ rollback           - Revert change
└─ list_pending       - Pending mutations

recovery/
├─ checkpoint         - Save state
├─ restore            - Restore state
├─ list_checkpoints   - Available snapshots
└─ recovery_plan      - Recovery strategy
```

### Extensible Domain Capabilities
```
<domain>/
├─ custom_capability_1
├─ custom_capability_2
└─ ...
```

## Authentication & Authorization

### Authentication Methods
1. **API Key** - For programmatic access (OpenCode, VS Code ext)
2. **OAuth2** - For web UI (Claude.ai)
3. **Service Account** - For internal federation
4. **NATS Token** - For NATS transport

### Authorization
- Role-based access control (RBAC)
- Per-capability permissions
- Multi-tenancy support
- Audit logging

## Configuration

### Service Configuration
```yaml
# mcp_service.yaml
server:
  name: "Aaroneous MCP Service"
  version: "3.0.0"
  listen_addr: "0.0.0.0:3333"
  
protocols:
  mcp:
    enabled: true
    transports: ["stdio", "http"]
  
  http:
    enabled: true
    port: 8080
    cors_origins: ["https://opencode.ai", "https://api.anthropic.com"]
  
  websocket:
    enabled: true
    port: 8443
  
  nats:
    enabled: true
    url: "nats://localhost:4222"

capabilities:
  auto_register: true
  schema_validation: true
  rate_limiting:
    enabled: true
    default_rps: 100  # requests per second

auth:
  api_key_required: true
  oauth2_enabled: true
  audit_logging: true

federation:
  enabled: true
  peers: ["Guild:3333", "Merlin:3333", "Library:3333"]
  sync_interval: 60  # seconds
```

## API Examples

### Example 1: OpenCode Integration
```bash
# Start Aaroneous MCP server
aaroneous mcp-service start

# OpenCode discovers capabilities
curl http://localhost:8080/api/v1/capabilities

# OpenCode calls a capability
curl -X POST http://localhost:8080/api/v1/call \
  -H "Authorization: Bearer <api-key>" \
  -H "Content-Type: application/json" \
  -d '{
    "capability": "federation.healthcheck",
    "trace_id": "opencode-trace-123",
    "params": {}
  }'

# Response
{
  "result": {
    "status": "healthy",
    "nodes": 3,
    "uptime_hours": 142,
    "requests_total": 50000
  }
}
```

### Example 2: VS Code Extension
```javascript
// VS Code extension using MCP protocol
const mcpClient = new MCPClient({
  url: "http://localhost:3333",
  apiKey: process.env.AARONEOUS_API_KEY
});

// Discover capabilities
const capabilities = await mcpClient.listCapabilities();

// Call a capability
const result = await mcpClient.call("intelligence.anomaly_detection", {
  metrics: [...],
  threshold: 2.5
});

// Subscribe to real-time updates
mcpClient.subscribe("federation.status", (update) => {
  updateStatusBar(update);
});
```

### Example 3: Claude Web UI
```javascript
// Claude integration via MCP protocol
const mcpResponse = await fetch(
  "http://localhost:8080/api/v1/call",
  {
    method: "POST",
    headers: {
      "Authorization": "Bearer <oauth-token>",
      "Content-Type": "application/json"
    },
    body: JSON.stringify({
      capability: "intelligence.optimize",
      trace_id: claude.traceId,
      params: {
        system_metrics: {...}
      }
    })
  }
);

// Display recommendations
const recommendations = await mcpResponse.json();
displayRecommendations(recommendations.result);
```

## Implementation Roadmap

### Phase 1: Generalize MCP (Week 1)
- Remove AAS-specific assumptions
- Create generic server/client interfaces
- Add protocol abstraction layer
- Support multiple transports (MCP, HTTP, NATS)

### Phase 2: HTTP/REST API (Week 2)
- Implement REST endpoint `/api/v1/`
- Add OpenAPI/Swagger documentation
- CORS support for web UIs
- Rate limiting and quotas

### Phase 3: WebSocket Support (Week 3)
- Real-time capability updates
- Event streaming
- Progress tracking
- Connection pooling

### Phase 4: Documentation & SDKs (Week 4)
- Comprehensive API docs
- OpenCode integration guide
- VS Code extension template
- Python/TypeScript/Rust SDKs

### Phase 5: Advanced Features (Week 5)
- OAuth2 / OIDC
- Multi-tenancy
- Billing/metering
- Plugin marketplace

## Benefits

1. **Vendor Agnostic** - Works with any client tool
2. **Standards Compliant** - Uses MCP protocol
3. **Multiple Transports** - HTTP, NATS, WebSocket, etc.
4. **Web Native** - REST API + WebSocket
5. **Easily Discoverable** - Self-describing capabilities
6. **Production Ready** - Auth, rate limiting, monitoring
7. **Extensible** - Custom domains and capabilities
8. **Open Source** - Permissive licensing

## Success Metrics

- OpenCode can call Aaroneous capabilities
- VS Code extension can execute commands
- Claude Web UI can interact with federation
- REST API has full test coverage
- Performance: <100ms latency for 95th percentile
- Rate limiting prevents abuse
- Multi-client concurrent usage works
- Complete audit trail maintained

## Migration Path (Backward Compatible)

```
Old (AAS-only):
├─ MCP Client (Python)
└─ MCP Server (Rust)

New (Universal):
├─ OpenCode CLI ────┐
├─ VS Code ext ─────┤
├─ Claude.ai ───────┼──→ Aaroneous Universal MCP Service
├─ AAS Client ──────┤
└─ Custom Tools ────┘
```

All existing AAS functionality preserved and enhanced!

---

**Timeline:** 4-5 weeks  
**Effort:** ~2000 lines Rust + docs  
**Impact:** Transforms Aaroneous from internal component to public service
