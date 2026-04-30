# Aaroneous Federation: API Documentation

## Overview

Complete API reference for Aaroneous Federation with OpenAPI 3.0 specification and GraphQL schema.

---

## Table of Contents

1. [REST API](#rest-api)
2. [OpenAPI Specification](#openapi-specification)
3. [GraphQL API](#graphql-api)
4. [WebSocket API](#websocket-api)
5. [Authentication](#authentication)
6. [Rate Limiting](#rate-limiting)
7. [Error Handling](#error-handling)

---

## REST API

### Base URL

```
https://api.aaroneous.example.com/api/v1
```

### Endpoints

#### 1. Get Federation Status

```
GET /cluster/status
```

**Response:**
```json
{
  "status": "healthy",
  "uptime_seconds": 86400,
  "specialists_online": 6,
  "hive_count": 3,
  "consensus_working": true,
  "api_version": "1.0.0"
}
```

#### 2. Submit Proposal Request

```
POST /proposals/request
Content-Type: application/json
Authorization: Bearer <token>
```

**Request Body:**
```json
{
  "request_id": "req-001",
  "context": {
    "user_id": "user-123",
    "metadata": {
      "content": "Analyze this product review...",
      "category": "sentiment_analysis"
    },
    "constraints": [
      {
        "type": "latency",
        "value": "100ms"
      }
    ],
    "budget": {
      "max_compute_ms": 100,
      "max_memory_mb": 256
    }
  }
}
```

**Response:**
```json
{
  "request_id": "req-001",
  "proposals": [
    {
      "proposal_id": "prop-001",
      "specialist_id": "sentiment-analyzer",
      "solution": {
        "type": "sentiment_analysis",
        "description": "Detected positive sentiment",
        "parameters": {
          "sentiment_score": 0.87,
          "confidence": 0.92
        }
      },
      "confidence": 0.92
    },
    {
      "proposal_id": "prop-002",
      "specialist_id": "topic-extractor",
      "solution": {
        "type": "topic_extraction",
        "description": "Extracted topics: quality, value, reliability",
        "parameters": {
          "topics": ["quality", "value", "reliability"]
        }
      },
      "confidence": 0.88
    }
  ],
  "status": "ready_for_consensus"
}
```

#### 3. Execute Proposal

```
POST /proposals/{proposal_id}/execute
Authorization: Bearer <token>
```

**Request Body:**
```json
{
  "execution_config": {
    "priority": "high",
    "timeout_ms": 5000
  }
}
```

**Response:**
```json
{
  "execution_id": "exec-001",
  "proposal_id": "prop-001",
  "status": "success",
  "output": {
    "result": "Analysis complete",
    "metrics": {
      "execution_time_ms": 45,
      "memory_used_mb": 128
    }
  }
}
```

#### 4. Get Consensus Decision

```
GET /consensus/{request_id}
Authorization: Bearer <token>
```

**Response:**
```json
{
  "request_id": "req-001",
  "consensus_result": {
    "winning_proposal_id": "prop-001",
    "specialist_agreement": [
      "sentiment-analyzer",
      "topic-extractor"
    ],
    "confidence": 0.96,
    "reasoning": "98% agreement on sentiment classification"
  }
}
```

#### 5. Multi-Hive Federation

```
GET /hives
Authorization: Bearer <token>
```

**Response:**
```json
{
  "hives": [
    {
      "hive_id": "hive-1",
      "region": "us-east-1",
      "specialists_count": 6,
      "status": "healthy",
      "latency_ms": 2.3,
      "last_heartbeat": "2024-01-15T10:30:45Z"
    },
    {
      "hive_id": "hive-2",
      "region": "eu-west-1",
      "specialists_count": 6,
      "status": "healthy",
      "latency_ms": 45.8,
      "last_heartbeat": "2024-01-15T10:30:44Z"
    }
  ],
  "consensus_agreement": 96.5
}
```

#### 6. DNA Bank Query

```
POST /dna/query
Authorization: Bearer <token>
Content-Type: application/json
```

**Request Body:**
```json
{
  "event_type": "proposal_execution",
  "specialist_id": "sentiment-analyzer",
  "confidence_threshold": 0.7,
  "limit": 100
}
```

**Response:**
```json
{
  "events": [
    {
      "event_id": "evt-001",
      "event_type": "proposal_execution",
      "specialist_id": "sentiment-analyzer",
      "timestamp": "2024-01-15T10:30:45Z",
      "details": {
        "confidence": 0.92,
        "execution_time_ms": 45
      }
    }
  ],
  "total_count": 1250,
  "patterns_found": [
    {
      "pattern": "high_confidence_with_fast_execution",
      "occurrence_count": 380,
      "confidence": 0.89
    }
  ]
}
```

#### 7. Audit Log Query

```
GET /audit/logs?event_type=proposal_execution&user_id=user-123&limit=50
Authorization: Bearer <token>
```

**Response:**
```json
{
  "logs": [
    {
      "log_id": "log-001",
      "timestamp": "2024-01-15T10:30:45Z",
      "event_type": "proposal_execution",
      "user_id": "user-123",
      "action": "Executed proposal prop-001",
      "result": "success",
      "resource_id": "prop-001"
    }
  ],
  "total_count": 5432,
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_pages": 109
  }
}
```

#### 8. Metrics

```
GET /metrics?range=1h
Authorization: Bearer <token>
```

**Response:**
```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "range": "1h",
  "metrics": {
    "proposals_total": 4500,
    "proposals_per_sec": 125,
    "consensus_agreement_percent": 96.5,
    "average_latency_ms": 45.2,
    "p95_latency_ms": 120.5,
    "p99_latency_ms": 250.3,
    "memory_usage_mb": 4200,
    "cpu_utilization_percent": 65.3,
    "cache_hit_rate": 0.92
  }
}
```

---

## OpenAPI Specification

### Complete OpenAPI 3.0 Schema

```yaml
openapi: 3.0.0
info:
  title: Aaroneous Federation API
  version: 1.0.0
  description: API for Aaroneous Federation federated specialist system
  contact:
    name: Aaroneous Community
    url: https://github.com/anomalyco/aaroneous

servers:
  - url: https://api.aaroneous.example.com/api/v1
    description: Production

paths:
  /cluster/status:
    get:
      summary: Get federation cluster status
      tags:
        - Cluster
      responses:
        200:
          description: Cluster status
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ClusterStatus'

  /proposals/request:
    post:
      summary: Submit proposal request
      tags:
        - Proposals
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ProposalRequest'
      responses:
        200:
          description: Proposals generated
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ProposalResponse'
        400:
          description: Invalid request
        401:
          description: Unauthorized
        429:
          description: Rate limit exceeded

  /proposals/{proposal_id}/execute:
    post:
      summary: Execute a proposal
      tags:
        - Proposals
      parameters:
        - name: proposal_id
          in: path
          required: true
          schema:
            type: string
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ExecutionConfig'
      responses:
        200:
          description: Execution result
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExecutionResult'

  /consensus/{request_id}:
    get:
      summary: Get consensus decision for request
      tags:
        - Consensus
      parameters:
        - name: request_id
          in: path
          required: true
          schema:
            type: string
      responses:
        200:
          description: Consensus result
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ConsensusResult'

  /hives:
    get:
      summary: List all hives in federation
      tags:
        - Federation
      responses:
        200:
          description: Hive list
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/HiveList'

  /dna/query:
    post:
      summary: Query DNA Bank for patterns
      tags:
        - DNA Bank
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/DNAQuery'
      responses:
        200:
          description: DNA query results
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/DNAQueryResult'

  /audit/logs:
    get:
      summary: Query audit logs
      tags:
        - Audit
      parameters:
        - name: event_type
          in: query
          schema:
            type: string
        - name: user_id
          in: query
          schema:
            type: string
        - name: limit
          in: query
          schema:
            type: integer
            default: 50
      responses:
        200:
          description: Audit logs
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuditLogResult'

  /metrics:
    get:
      summary: Get performance metrics
      tags:
        - Metrics
      parameters:
        - name: range
          in: query
          schema:
            type: string
            default: 1h
      responses:
        200:
          description: Metrics
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Metrics'

components:
  schemas:
    ClusterStatus:
      type: object
      required:
        - status
        - uptime_seconds
        - specialists_online
      properties:
        status:
          type: string
          enum: [healthy, degraded, unhealthy]
        uptime_seconds:
          type: integer
        specialists_online:
          type: integer
        hive_count:
          type: integer
        consensus_working:
          type: boolean
        api_version:
          type: string

    ProposalRequest:
      type: object
      required:
        - request_id
        - context
      properties:
        request_id:
          type: string
        context:
          $ref: '#/components/schemas/Context'

    Context:
      type: object
      required:
        - request_id
        - metadata
      properties:
        request_id:
          type: string
        user_id:
          type: string
          nullable: true
        metadata:
          type: object
          additionalProperties: true
        constraints:
          type: array
          items:
            $ref: '#/components/schemas/Constraint'
        budget:
          $ref: '#/components/schemas/ResourceBudget'

    Constraint:
      type: object
      required:
        - type
        - value
      properties:
        type:
          type: string
        value:
          type: string
        priority:
          type: integer
          minimum: 1
          maximum: 10

    ResourceBudget:
      type: object
      properties:
        max_compute_ms:
          type: integer
        max_memory_mb:
          type: integer
        max_cost_dollars:
          type: number

    ProposalResponse:
      type: object
      properties:
        request_id:
          type: string
        proposals:
          type: array
          items:
            $ref: '#/components/schemas/Proposal'
        status:
          type: string

    Proposal:
      type: object
      required:
        - proposal_id
        - specialist_id
        - confidence
      properties:
        proposal_id:
          type: string
        specialist_id:
          type: string
        solution:
          type: object
          additionalProperties: true
        confidence:
          type: number
          minimum: 0
          maximum: 1

    ExecutionResult:
      type: object
      properties:
        execution_id:
          type: string
        proposal_id:
          type: string
        status:
          type: string
          enum: [success, failed, timeout]
        output:
          type: object
          additionalProperties: true

    ConsensusResult:
      type: object
      properties:
        request_id:
          type: string
        consensus_result:
          type: object
          additionalProperties: true

    HiveList:
      type: object
      properties:
        hives:
          type: array
          items:
            $ref: '#/components/schemas/HiveInfo'
        consensus_agreement:
          type: number

    HiveInfo:
      type: object
      properties:
        hive_id:
          type: string
        region:
          type: string
        specialists_count:
          type: integer
        status:
          type: string
        latency_ms:
          type: number

    DNAQuery:
      type: object
      properties:
        event_type:
          type: string
        specialist_id:
          type: string
        confidence_threshold:
          type: number
        limit:
          type: integer

    DNAQueryResult:
      type: object
      properties:
        events:
          type: array
          items:
            type: object
        total_count:
          type: integer
        patterns_found:
          type: array
          items:
            type: object

    AuditLogResult:
      type: object
      properties:
        logs:
          type: array
          items:
            $ref: '#/components/schemas/AuditLog'
        total_count:
          type: integer

    AuditLog:
      type: object
      properties:
        log_id:
          type: string
        timestamp:
          type: string
          format: date-time
        event_type:
          type: string
        user_id:
          type: string
        action:
          type: string
        result:
          type: string

    Metrics:
      type: object
      properties:
        timestamp:
          type: string
          format: date-time
        metrics:
          type: object
          additionalProperties:
            type: number

  securitySchemes:
    Bearer:
      type: http
      scheme: bearer
      bearerFormat: JWT

security:
  - Bearer: []
```

---

## GraphQL API

### GraphQL Endpoint

```
POST https://api.aaroneous.example.com/graphql
```

### Schema

```graphql
type Query {
  """Get federation cluster status"""
  clusterStatus: ClusterStatus!

  """Get specialist by ID"""
  specialist(id: ID!): Specialist

  """List all specialists"""
  specialists(limit: Int, offset: Int): [Specialist!]!

  """Get hive by ID"""
  hive(id: ID!): Hive

  """List all hives"""
  hives: [Hive!]!

  """Query DNA Bank events"""
  dnaEvents(
    eventType: String
    specialistId: ID
    confidenceThreshold: Float
    limit: Int
  ): [DNAEvent!]!

  """Query audit logs"""
  auditLogs(
    eventType: String
    userId: ID
    action: String
    limit: Int
    offset: Int
  ): AuditLogConnection!

  """Get metrics"""
  metrics(range: String!): Metrics!

  """Get proposal by ID"""
  proposal(id: ID!): Proposal

  """Get execution result"""
  execution(id: ID!): ExecutionResult
}

type Mutation {
  """Submit proposal request"""
  submitProposal(input: ProposalRequestInput!): ProposalResponse!

  """Execute a proposal"""
  executeProposal(proposalId: ID!, config: ExecutionConfigInput): ExecutionResult!

  """Trigger learning feedback"""
  submitFeedback(input: FeedbackInput!): Feedback!

  """Update specialist configuration"""
  updateSpecialistConfig(
    specialistId: ID!
    config: SpecialistConfigInput!
  ): Specialist!
}

type Subscription {
  """Subscribe to proposal updates"""
  proposalUpdates(requestId: ID!): Proposal!

  """Subscribe to metrics updates"""
  metricsUpdates(interval: Int!): Metrics!

  """Subscribe to consensus decisions"""
  consensusUpdates(hiveId: ID): ConsensusResult!
}

type ClusterStatus {
  status: String!
  uptimeSeconds: Int!
  specialistsOnline: Int!
  hiveCount: Int!
  consensusWorking: Boolean!
  apiVersion: String!
}

type Specialist {
  id: ID!
  name: String!
  capabilities: [String!]!
  status: String!
  proposals: [Proposal!]!
  learningMetrics: LearningMetrics!
}

type Hive {
  id: ID!
  region: String!
  status: String!
  specialistsCount: Int!
  specialists: [Specialist!]!
  latencyMs: Float!
  lastHeartbeat: DateTime!
}

type Proposal {
  id: ID!
  specialistId: ID!
  solution: JSON!
  confidence: Float!
  estimatedCost: Cost!
  dependencies: [String!]!
  alternatives: [Proposal!]!
}

type ExecutionResult {
  executionId: ID!
  proposalId: ID!
  status: ExecutionStatus!
  output: JSON!
  executionTimeMs: Int!
  memoryUsedMb: Int!
}

type DNAEvent {
  id: ID!
  eventType: String!
  specialistId: ID!
  timestamp: DateTime!
  details: JSON!
}

type Metrics {
  timestamp: DateTime!
  proposalsTotal: Int!
  proposalsPerSecond: Float!
  consensusAgreementPercent: Float!
  averageLatencyMs: Float!
  p95LatencyMs: Float!
  p99LatencyMs: Float!
  memoryUsageMb: Int!
  cpuUtilizationPercent: Float!
  cacheHitRate: Float!
}

type Cost {
  computeMs: Int!
  memoryMb: Int!
  storageMb: Int!
  networkMb: Int!
}

type LearningMetrics {
  totalProposals: Int!
  successRate: Float!
  averageConfidence: Float!
  improvementPercent: Float!
}

type AuditLogConnection {
  edges: [AuditLogEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type AuditLogEdge {
  cursor: String!
  node: AuditLog!
}

type AuditLog {
  id: ID!
  timestamp: DateTime!
  eventType: String!
  userId: ID
  action: String!
  resourceId: String!
  result: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

input ProposalRequestInput {
  requestId: ID!
  context: ContextInput!
}

input ContextInput {
  userId: ID
  metadata: JSON!
  constraints: [ConstraintInput!]
  budget: ResourceBudgetInput
}

input ExecutionConfigInput {
  priority: String
  timeoutMs: Int
}

input FeedbackInput {
  specialistId: ID!
  successRate: Float!
  improvementPercent: Float!
}

input SpecialistConfigInput {
  cacheSize: Int
  quantizationLevel: String
}

enum ExecutionStatus {
  SUCCESS
  FAILED
  TIMEOUT
}

scalar DateTime
scalar JSON
```

### GraphQL Examples

#### Query Cluster Status

```graphql
query {
  clusterStatus {
    status
    specialistsOnline
    hiveCount
    consensusWorking
  }
}
```

#### Submit Proposal with Mutation

```graphql
mutation {
  submitProposal(input: {
    requestId: "req-001"
    context: {
      userId: "user-123"
      metadata: {
        content: "Analyze this content..."
        category: "sentiment_analysis"
      }
    }
  }) {
    proposals {
      id
      specialistId
      confidence
      solution
    }
  }
}
```

#### Subscribe to Metrics Updates

```graphql
subscription {
  metricsUpdates(interval: 5000) {
    proposalsPerSecond
    averageLatencyMs
    cacheHitRate
  }
}
```

---

## WebSocket API

### Connection

```bash
wss://api.aaroneous.example.com/ws
```

### Message Format

```json
{
  "type": "subscribe",
  "channel": "proposals",
  "request_id": "req-001"
}
```

### Available Channels

- `proposals` - Proposal updates
- `consensus` - Consensus decisions
- `metrics` - Real-time metrics
- `hives` - Hive status updates
- `dna` - DNA Bank events

---

## Authentication

### Bearer Token

```bash
curl -H "Authorization: Bearer your-token-here" \
  https://api.aaroneous.example.com/api/v1/cluster/status
```

### OAuth 2.0

```
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=client_credentials&client_id=...&client_secret=...
```

### API Keys

```bash
curl -H "X-API-Key: your-api-key" \
  https://api.aaroneous.example.com/api/v1/cluster/status
```

---

## Rate Limiting

Limits per API key:
- **Free:** 100 requests/hour
- **Pro:** 10,000 requests/hour
- **Enterprise:** Unlimited

Headers:
```
X-RateLimit-Limit: 10000
X-RateLimit-Remaining: 9999
X-RateLimit-Reset: 1705329600
```

---

## Error Handling

### Error Responses

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Too many requests",
    "status": 429,
    "details": {
      "retry_after": 60
    }
  }
}
```

### Error Codes

| Code | HTTP | Description |
|------|------|---|
| INVALID_REQUEST | 400 | Invalid request parameters |
| UNAUTHORIZED | 401 | Missing or invalid authentication |
| FORBIDDEN | 403 | User doesn't have permission |
| NOT_FOUND | 404 | Resource not found |
| RATE_LIMIT_EXCEEDED | 429 | Rate limit reached |
| INTERNAL_ERROR | 500 | Server error |

---

## Summary

Complete API documentation providing:

✅ **REST endpoints** for all operations
✅ **OpenAPI 3.0 specification** for integration
✅ **GraphQL API** for flexible queries
✅ **WebSocket support** for real-time updates
✅ **Multiple authentication methods**
✅ **Rate limiting** and quotas
✅ **Comprehensive error handling**
✅ **Code examples** and samples

---

**Start integrating! 🚀**
