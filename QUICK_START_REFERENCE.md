# Aaroneous Federation: Quick Start Reference Card

## One-Page Quick Reference

### Installation (30 seconds)

```bash
git clone https://github.com/anomalyco/aaroneous.git
cd aaroneous
docker-compose up -d
curl http://localhost:8001/health
```

### Core Concepts

| Concept | Meaning |
|---------|---------|
| **Specialist** | AI agent with domain expertise (6 built-in) |
| **Proposal** | Solution suggested by a specialist |
| **Consensus** | >66% agreement among specialists |
| **DNA Bank** | Learning system that records and improves |
| **Hive** | Cluster of specialists in one location |
| **Federation** | Multiple hives working together |

### Key Endpoints

```
GET  /health              - System status
GET  /cluster/status      - Cluster health
POST /proposals/request   - Submit proposal request
POST /proposals/{id}/execute - Execute proposal
GET  /consensus/{id}      - Get consensus result
GET  /metrics             - Performance metrics
POST /dna/query           - Query learning patterns
GET  /audit/logs          - Query audit trail
```

### Built-in Specialists

| Specialist | Role | Capability |
|-----------|------|-----------|
| **Sentinel** | Orchestration | Arbitrates proposals |
| **Visionary** | Design | Creative solutions |
| **Omnipresent** | Coordination | Multi-device sync |
| **Symbiotic** | Sensing | Biometric analysis |
| **Phygital** | Hardware | AR/3D rendering |
| **Archivist** | Memory | Event recording |

### Example Usage

```bash
# E-Commerce recommendations
cargo run --example ecommerce --release
# Output: Recommended products with 96% consensus

# Healthcare diagnostics
cargo run --example healthcare --release
# Output: Diagnosis with confidence scores

# Financial analysis
cargo run --example finance --release
# Output: Risk assessment and hedge recommendations

# Content moderation
cargo run --example content_moderation --release
# Output: Moderation decision with reasoning

# Smart city traffic
cargo run --example traffic --release
# Output: Optimized signal timing and routing
```

### Deployment Options

```bash
# Docker Compose (local dev)
docker-compose up -d

# Docker (single container)
docker run -p 8001:8001 aaroneous-federation:latest

# Kubernetes
helm install aaroneous aaroneous/aaroneous-federation

# Cloud (AWS/GCP/Azure)
terraform apply -f deploy/terraform/
```

### Building Custom Specialist (5 minutes)

```rust
use aaroneous_sdk::*;
#[async_trait]
impl Specialist for MySpecialist {
    fn id(&self) -> SpecialistId { SpecialistId::from("my-specialist") }
    fn name(&self) -> &str { "My Specialist" }
    fn capabilities(&self) -> Vec<String> { vec!["analysis".into()] }
    async fn propose(&self, context: &Context) -> Result<Proposal> {
        // Your logic here
        Ok(Proposal { ... })
    }
    // ... implement other methods
}
```

### Performance Checklist

✅ Consensus latency: 2-5ms
✅ Throughput: 100-2560 ops/sec
✅ Memory: 4-6GB (full) or 500-800MB (mobile)
✅ Cache hit rate: 90%+
✅ GPU acceleration: Available (5-50x)

### Configuration

```bash
# Environment variables
LOG_LEVEL=debug
FEDERATION_MODE=multi-hive
CONSENSUS_THRESHOLD=66
DATABASE_URL=postgresql://user:pass@localhost/db
REDIS_URL=redis://localhost:6379
QUANTIZATION_PRECISION=fp16
GPU_ACCELERATION_ENABLED=true
AUDIT_LOG_ENABLED=true
```

### Monitoring

```bash
# Health
curl http://localhost:8001/health

# Metrics
curl http://localhost:8001/metrics

# Logs
docker-compose logs -f aaroneous

# Dashboard
http://localhost:3000  # Grafana
http://localhost:9090  # Prometheus
```

### Testing

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test federation::tests::test_proposal

# Benchmark
cargo bench

# Coverage
cargo tarpaulin --out Html
```

### Debugging

```bash
# Enable debug logging
export RUST_LOG=aaroneous=debug

# View detailed logs
docker-compose logs aaroneous | grep ERROR

# Check pod status (Kubernetes)
kubectl describe pod -n aaroneous <pod-name>

# Profile CPU
curl http://localhost:8001/debug/pprof/profile?seconds=30 > cpu.prof
```

### API Quick Examples

**Submit Proposal:**
```bash
curl -X POST http://localhost:8001/api/v1/proposals/request \
  -H "Content-Type: application/json" \
  -d '{
    "request_id": "req-001",
    "context": {
      "user_id": "user-123",
      "metadata": {"query": "analyze this"}
    }
  }'
```

**Get Consensus:**
```bash
curl http://localhost:8001/api/v1/consensus/req-001
```

**Query Metrics:**
```bash
curl http://localhost:8001/api/v1/metrics?range=1h
```

**Query DNA Patterns:**
```bash
curl -X POST http://localhost:8001/api/v1/dna/query \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "proposal_execution",
    "confidence_threshold": 0.7
  }'
```

### Common Issues

| Problem | Solution |
|---------|----------|
| Port already in use | Change port: `docker-compose -e PORT=8002 up` |
| Database not connecting | Check DATABASE_URL env var |
| Slow responses | Check CPU/memory usage, enable quantization |
| Cache misses | Increase cache size or enable warming |
| Pod crashes | Check logs, increase memory, verify config |

### Learning Resources

| Resource | Purpose |
|----------|---------|
| [FEDERATION_README.md](./FEDERATION_README.md) | Overview |
| [FEDERATION_ARCHITECTURE.md](./FEDERATION_ARCHITECTURE.md) | Deep dive |
| [SDK_CUSTOM_SPECIALIST_GUIDE.md](./SDK_CUSTOM_SPECIALIST_GUIDE.md) | Building |
| [EXAMPLE_APPLICATIONS_GUIDE.md](./EXAMPLE_APPLICATIONS_GUIDE.md) | Examples |
| [DEPLOYMENT_GUIDE_COMPREHENSIVE.md](./DEPLOYMENT_GUIDE_COMPREHENSIVE.md) | Deployment |
| [MONITORING_AND_OBSERVABILITY.md](./MONITORING_AND_OBSERVABILITY.md) | Monitoring |
| [FAQ_AND_TROUBLESHOOTING.md](./FAQ_AND_TROUBLESHOOTING.md) | Help |

### Getting Help

- **GitHub Issues:** Report bugs
- **GitHub Discussions:** Ask questions
- **Discord:** Real-time chat community
- **Email:** hello@aaroneous.ai

### Key Statistics

- **Latency:** 2-5ms (p95)
- **Throughput:** 100-2560 ops/sec
- **Memory:** 4-6GB standard, 500MB mobile
- **Tests:** 277+ (100% pass)
- **Specialists:** 6 built-in
- **Platforms:** 9 supported
- **Examples:** 5 included

### Next Steps

1. ✅ **Get Started** (5 min): Run `docker-compose up`
2. ✅ **Try Example** (15 min): `cargo run --example ecommerce`
3. ✅ **Read Docs** (30 min): Read architecture guide
4. ✅ **Build Custom** (1 hour): Create specialist
5. ✅ **Deploy** (1 day): Deploy to production

### Important Links

- **Repository:** https://github.com/anomalyco/aaroneous
- **Documentation:** https://docs.aaroneous.ai
- **API Docs:** https://api.docs.aaroneous.ai
- **Community:** https://discord.gg/aaroneous
- **Twitter:** https://twitter.com/AaroneousAI
- **Email:** hello@aaroneous.ai

---

**Print this card. Share it. Use it. Share feedback.**

*Aaroneous Federation v1.0.0 - Open Source, Production Ready*
