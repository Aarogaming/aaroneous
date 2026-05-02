# Aaroneous O3DE Gem Integration Guide

## Architecture

Aaroneous runs as a standalone Rust process (`cargo run --bin aaroneous -- start`).
The O3DE project connects to it as a client over HTTP. No shared memory or FFI required
for the initial integration.

```
O3DE AaroneousGem (C++)
    └── SystemComponent
            ├── Persistent SSE connection → GET /specialists/stream
            ├── Initial sync on connect  → GET /specialists
            ├── User intent submission   → POST /sessions/{id}/intent
            └── Forge operations         → POST /forge/crystallize
                                                     ↕ JSON over HTTP
Aaroneous (localhost:8765)
    ├── GET /specialists/stream    ← SSE push, fires on every state change
    ├── GET /specialists           ← Full snapshot for initial sync
    ├── GET /results/stream        ← SSE push of execution results only
    ├── POST /dynamic-specialists  ← Hot-add a crystallized sovereign
    └── POST /forge/crystallize    ← Splice tensors, write GGUF to disk
```

## Setup: Day 1

### 1. Install O3DE
Download and run the O3DE Windows installer from https://o3de.org/download/
Default install: `C:\O3DE\<version>`

Set environment variable before launching:
```
$env:AWS_EC2_METADATA_DISABLED = "true"
```

### 2. Create a new O3DE project
Open O3DE Project Manager → New Project → use the "Standard" template.

### 3. Enable required Gems (Project Manager → Configure Gems)
- **HttpRequestor** — HTTP client (already brings AWS SDK as transitive dep)
- **ImGui** — real-time developer dashboard
- **LyShine** — 2D UI for specialist "presence cards"
- **Atom Common Features** — rendering (enabled by default)

### 4. Create the AaroneousGem
```
cd C:\O3DE\<version>
scripts\o3de.bat create-gem --gem-name AaroneousGem --gem-path <your-project>/Gems/AaroneousGem
```

Enable the new Gem in Project Manager.

## The C++ Gem: Minimum Viable Implementation

### AaroneousSystemComponent.h
```cpp
#pragma once
#include <AzCore/Component/Component.h>
#include <AzCore/Component/TickBus.h>
#include <HttpRequestor/HttpRequestorRequestBus.h>

namespace AaroneousGem {

// Event broadcast when Aaroneous sends a specialist update
class SpecialistUpdateNotifications : public AZ::EBusTraits {
public:
    virtual void OnSpecialistUpdate(const AZStd::string& specialistName,
                                    float confidence,
                                    const AZStd::string& lastAction) = 0;
};
using SpecialistUpdateBus = AZ::EBus<SpecialistUpdateNotifications>;

class AaroneousSystemComponent
    : public AZ::Component
    , public AZ::TickBus::Handler
{
public:
    AZ_COMPONENT(AaroneousSystemComponent, "{GENERATE-A-UUID-HERE}");
    static void Reflect(AZ::ReflectContext* context);
    static void GetRequiredServices(AZ::ComponentDescriptor::DependencyArrayType& required);

    void Activate() override;
    void Deactivate() override;

    // AZ::TickBus::Handler
    void OnTick(float deltaTime, AZ::ScriptTimePoint time) override;

private:
    void FetchSpecialistSnapshot();
    void ParseSpecialistPayload(const AZStd::string& body);

    float m_pollIntervalSec = 0.2f;   // 200ms polling (use SSE for production)
    float m_timeSinceLastPoll = 0.0f;
    AZStd::string m_aaroneousBase = "http://localhost:8765";
};

} // namespace AaroneousGem
```

### AaroneousSystemComponent.cpp (polling path — works with standard HTTPRequestor)
```cpp
#include "AaroneousSystemComponent.h"
#include <AzCore/Serialization/SerializeContext.h>
#include <HttpRequestor/HttpRequestorRequestBus.h>
#include <AzCore/JSON/rapidjson.h>  // or use the AWS SDK JSON parser

void AaroneousGem::AaroneousSystemComponent::Activate() {
    AZ::TickBus::Handler::BusConnect();
    FetchSpecialistSnapshot();  // Initial sync
}

void AaroneousGem::AaroneousSystemComponent::Deactivate() {
    AZ::TickBus::Handler::BusDisconnect();
}

void AaroneousGem::AaroneousSystemComponent::OnTick(float deltaTime, AZ::ScriptTimePoint) {
    m_timeSinceLastPoll += deltaTime;
    if (m_timeSinceLastPoll >= m_pollIntervalSec) {
        m_timeSinceLastPoll = 0.0f;

        // Poll GET /specialists for state updates
        HttpRequestor::HttpRequestorRequestBus::Broadcast(
            &HttpRequestor::HttpRequestorRequests::AddTextRequest,
            m_aaroneousBase + "/specialists",
            Aws::Http::HttpMethod::HTTP_GET,
            [this](const AZStd::string& body, Aws::Http::HttpResponseCode code) {
                if (code == Aws::Http::HttpResponseCode::OK) {
                    ParseSpecialistPayload(body);
                }
            }
        );
    }
}

void AaroneousGem::AaroneousSystemComponent::FetchSpecialistSnapshot() {
    HttpRequestor::HttpRequestorRequestBus::Broadcast(
        &HttpRequestor::HttpRequestorRequests::AddTextRequest,
        m_aaroneousBase + "/specialists",
        Aws::Http::HttpMethod::HTTP_GET,
        [this](const AZStd::string& body, Aws::Http::HttpResponseCode code) {
            if (code == Aws::Http::HttpResponseCode::OK) {
                ParseSpecialistPayload(body);
            }
        }
    );
}

void AaroneousGem::AaroneousSystemComponent::ParseSpecialistPayload(const AZStd::string& body) {
    // Parse JSON, extract per-specialist state, broadcast on EBus
    // Each specialist entity in the scene listens on SpecialistUpdateBus
    // and updates its Atom material parameters and LyShine text accordingly.
    
    // Minimal parse: iterate "specialists" array
    // For production: use rapidjson (available in O3DE via AzCore) or
    // the AWS SDK's JsonView passed from AddRequest (not AddTextRequest)
    
    // Example: broadcast to all subscribers
    // SpecialistUpdateBus::Broadcast(&SpecialistUpdateNotifications::OnSpecialistUpdate,
    //     "Visionary", 0.85f, "generated 3 design variants");
}
```

### Submit an intent from O3DE (when user speaks or types)
```cpp
// POST /sessions/{session_id}/intent
AZStd::string body = R"({"content": "redesign the spatial hive dashboard"})";
HttpRequestor::HttpRequestorRequestBus::Broadcast(
    &HttpRequestor::HttpRequestorRequests::AddTextRequestWithHeadersAndBody,
    m_aaroneousBase + "/sessions/" + m_sessionId + "/intent",
    Aws::Http::HttpMethod::HTTP_POST,
    HttpRequestor::Headers{{"Content-Type", "application/json"}},
    body,
    [](const AZStd::string& resp, Aws::Http::HttpResponseCode code) {
        // result arrives in GET /specialists/stream or GET /results/stream
    }
);
```

## Scene Layout: Specialist Entities

Recommended spatial arrangement — the "Federation Constellation":

```
                    [Sentinel]          ← center, largest entity, orchestrator glow
                   /    |    \
         [Visionary] [Symbiotic] [Archivist]
                   \    |    /
              [Omnipresent] [Phygital]
                                         ← GenericSpecialists orbit dynamically
```

Each entity carries:
- **Atom Mesh** — distinct geometry per specialist type (Visionary: crystal, Archivist: book/cube, etc.)
- **Emissive Material** — intensity driven by `confidence_score`; color by status (idle=dim, active=bright, executing=pulsing)
- **LyShine Canvas on Mesh** — "presence card" showing name, last action, confidence bar
- **Custom SpecialistDataComponent** — caches Aaroneous state, drives material parameters each frame

## The /specialists/stream SSE Event Format

Every event has this structure:
```json
{
  "type": "execution_complete",
  "specialist": "Visionary",
  "specialist_id": "Visionary",
  "action": "generate_design",
  "status": "Success",
  "output_preview": "LLM generated 3 design variant(s) for 'redesign...'",
  "duration_ms": 523,
  "active_intent": "redesign the spatial hive dashboard",
  "timestamp_ms": 1746123456789
}
```

Or for intent submissions:
```json
{
  "type": "intent_submitted",
  "intent_id": "intent-abc123",
  "content": "redesign the spatial hive dashboard",
  "priority": "Normal",
  "timestamp_ms": 1746123456000
}
```

Or heartbeats (every 2s if no activity):
```
: heartbeat
```

## Production Path: Persistent Connection via Custom HTTP Client

The HTTPRequestor Gem does not support SSE (streaming responses).
For production, implement a C++ Gem using libcurl or Windows WinHTTP
to hold a persistent chunked-encoding connection to `/specialists/stream`.

Parse SSE events by splitting on `\n\n` delimiters:
```
data: {"type":"execution_complete",...}\n\n
```

Each `data:` line is one broadcast event. Route parsed events into your
`SpecialistUpdateBus` instead of polling.

## Authentication

If `AARONEOUS_API_KEY` is set on the server:
```cpp
HttpRequestor::Headers headers = {
    {"Authorization", "Bearer " + m_apiKey},
    {"Content-Type", "application/json"}
};
```

## Next Steps

1. Build the C++ Gem with polling (Day 2-3)
2. Add ImGui overlay showing specialist state (Day 4)
3. Author specialist entity prefabs with Atom materials (Day 5)
4. Wire LyShine canvases on mesh for presence cards (Day 6-7)
5. Upgrade to persistent SSE connection via libcurl (production)
6. Add OpenXR spatial anchoring via Phygital (see ar-openxr feature in Aaroneous)
