retry
# Aaroneous Federation: Deployment & Operations Guide

**Complete reference guide for deploying, monitoring, and operating Aaroneous Federation in production.**

---

## Quick Links

### Getting Started
- [**5-Minute Quickstart**](#5-minute-quickstart) - Local development
- [**Deployment Comparison**](#deployment-comparison) - Choose your platform
- [**Configuration Guide**](#configuration-guide) - Environment setup

### Deployment Guides
- [**Docker Compose** (Local Dev)](#docker-compose-local-development) - docker-compose.yml
- [**Kubernetes** (Production)](#kubernetes-production-deployment) - Helm charts
- [**Cloud Platforms**](#cloud-deployment) - AWS/GCP/Azure
- [**Mobile Devices** (iOS/Android)](#mobile-deployment) - Mobile apps
- [**Infrastructure-as-Code**](#infrastructure-as-code) - Terraform

### Operations & Monitoring
- [**Monitoring Dashboard**](#monitoring-observability) - Prometheus/Grafana
- [**Alerts & Health**](#alerting-rules) - 20+ alert rules
- [**Backup & Recovery**](#backup--recovery) - Disaster recovery
- [**Performance Tuning**](#performance-tuning) - Optimization
- [**CI/CD Pipeline**](#cicd-pipeline) - GitHub Actions

### Reference
- [**Environment Variables**](#environment-variables) - Configuration options
- [**Metrics Reference**](#metrics-reference) - Available metrics
- [**Troubleshooting**](#troubleshooting) - Common issues
- [**Architecture**](./FEDERATION_ARCHITECTURE.md) - System design
- [**API Documentation**](./architecture/FEDERATION_README.md) - API reference

---

## 5-Minute Quickstart

### Prerequisites
- Docker & Docker Compose OR Kubernetes cluster
- PostgreSQL 15+ OR use Docker
- Redis 7+ OR use Docker

### Option 1: Docker Compose (Recommended for local development)

```bash
# Clone repository
git clone https://github.com/anomalyco/aaroneous.git
cd aaroneous

# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f aaroneous

# Test API
curl http://localhost:8001/health
```

**Services:**
- Aaroneous API: http://localhost:8001
- Grafana Dashboards: http://localhost:3000
- Prometheus Metrics: http://localhost:9090
- Kibana Logs: http://localhost:5601
- Jaeger Tracing: http://localhost:16686

### Option 2: Kubernetes (Production)

```bash
# Create namespace
kubectl create namespace aaroneous

# Add Helm repo
helm repo add aaroneous https://charts.aaroneous.ai
helm repo update

# Deploy
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --namespace aaroneous \
  --values helm/values.yaml

# Verify
kubectl get pods -n aaroneous
kubectl port-forward -n aaroneous svc/aaroneous-federation 8001:8001

# Test
curl http://localhost:8001/health
```

### Option 3: Docker (Single instance)

```bash
# Build image
docker build -t aaroneous-federation:1.0.0 .

# Run with external database
docker run -d \
  --name aaroneous \
  -p 8001:8001 \
  -e DATABASE_URL="postgresql://user:pass@db-host:5432/aaroneous" \
  -e REDIS_URL="redis://redis-host:6379" \
  -e FEDERATION_MODE="multi-hive" \
  -e CONSENSUS_THRESHOLD=66 \
  aaroneous-federation:1.0.0

# Check logs
docker logs -f aaroneous
```

---

## Deployment Comparison

| Feature | Docker Compose | Docker | Kubernetes | Cloud (TF) | Mobile |
|---------|---|---|---|---|---|
| **Setup Time** | 2 min | 3 min | 10 min | 15 min | 30 min |
| **Learning Curve** | Easy | Medium | Hard | Hard | Hard |
| **Production Ready** | No | Yes | Yes | Yes | Yes |
| **Auto-scaling** | No | No | Yes | Yes | No |
| **Multi-region** | No | No | Yes | Yes | Local only |
| **Cost (monthly)** | $0 | $50-100 | $200-500 | $300-600 | $0 |
| **Monitoring** | Full | Basic | Full | Full | Limited |
| **High Availability** | No | No | Yes | Yes | N/A |

---

## Configuration Guide

### Environment Variables

```bash
# ==== Core Configuration ====
RUST_LOG=aaroneous=info
LOG_LEVEL=info

# ==== Database ====
DATABASE_URL=postgresql://user:password@localhost:5432/aaroneous_federation
DATABASE_MAX_CONNECTIONS=50
DATABASE_SSL=false
DATABASE_POOL_SIZE=20

# ==== Cache/Redis ====
REDIS_URL=redis://localhost:6379/0
REDIS_POOL_SIZE=50
CACHE_TTL=3600

# ==== Federation ====
FEDERATION_MODE=multi-hive  # single-hive or multi-hive
CONSENSUS_THRESHOLD=66      # Percentage
PEER_DISCOVERY=true
PEER_HIVES="hive-1:8001,hive-2:8001"

# ==== Specialists ====
SPECIALIST_SENTINEL_ENABLED=true
SPECIALIST_SENTINEL_INSTANCES=3
SPECIALIST_VISIONARY_ENABLED=true
SPECIALIST_OMNIPRESENT_ENABLED=true
SPECIALIST_SYMBIOTIC_ENABLED=true
SPECIALIST_PHYGITAL_ENABLED=true
SPECIALIST_ARCHIVIST_ENABLED=true

# ==== Optimization ====
QUANTIZATION_ENABLED=true
QUANTIZATION_PRECISION=fp16          # int4, int8, fp16, fp32
GPU_ACCELERATION_ENABLED=false
CACHE_WARMING_ENABLED=true
BATCH_PROCESSING_ENABLED=true
KERNEL_FUSION_ENABLED=true
MEMORY_POOLING_ENABLED=true

# ==== Performance ====
MAX_PROPOSAL_QUEUE=10000
MAX_AUDIT_EVENTS=100000
DNA_BANK_BATCH_SIZE=1000
LEADER_ELECTION_TIMEOUT_MS=5000

# ==== Enterprise ====
AUDIT_LOG_ENABLED=true
AUDIT_LOG_RETENTION_DAYS=90
COMPLIANCE_ENABLED=true
COMPLIANCE_FRAMEWORKS=gdpr,hipaa,soc2

# ==== Security ====
TLS_ENABLED=false
TLS_CERT_PATH=/etc/aaroneous/certs/server.crt
TLS_KEY_PATH=/etc/aaroneous/certs/server.key
RATE_LIMITING_ENABLED=true
RATE_LIMIT_RPS=1000
RATE_LIMIT_BURST=2000

# ==== Metrics ====
METRICS_ENABLED=true
METRICS_PORT=8001
METRICS_INTERVAL_SECS=60
```

### Using .env File

```bash
# Create .env file
cat > .env << EOF
DATABASE_URL=postgresql://aaroneous:password@db:5432/aaroneous_federation
REDIS_URL=redis://cache:6379
FEDERATION_MODE=multi-hive
CONSENSUS_THRESHOLD=66
EOF

# Load environment
export $(cat .env | xargs)

# Run
./target/release/aaroneous --config config.toml
```

### Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: aaroneous-config
  namespace: aaroneous
data:
  LOG_LEVEL: "info"
  FEDERATION_MODE: "multi-hive"
  CONSENSUS_THRESHOLD: "66"
  AUDIT_LOG_ENABLED: "true"
  COMPLIANCE_ENABLED: "true"
---
apiVersion: v1
kind: Secret
metadata:
  name: aaroneous-secrets
  namespace: aaroneous
type: Opaque
data:
  DATABASE_URL: cG9zdGdyZXM6Ly9kYnVzZXI6ZGJwYXNzQHBn... # base64 encoded
  REDIS_URL: cmVkaXM6Ly9jYWNoZTozNjM3OQ==              # base64 encoded
```

---

## Deployment Guides

### Docker Compose: Local Development

```bash
# Start all services
docker-compose up -d

# Follow logs
docker-compose logs -f

# Stop all services
docker-compose down

# Clean up volumes
docker-compose down -v

# Scale services
docker-compose up -d --scale aaroneous=3

# Access services
# API:        http://localhost:8001
# Grafana:    http://localhost:3000 (admin:admin)
# Prometheus: http://localhost:9090
# Kibana:     http://localhost:5601
# Jaeger:     http://localhost:16686
```

**Troubleshooting:**
```bash
# Check service status
docker-compose ps

# View service logs
docker-compose logs aaroneous
docker-compose logs database
docker-compose logs cache

# Restart service
docker-compose restart aaroneous

# Rebuild and restart
docker-compose up -d --build
```

### Kubernetes: Production Deployment

```bash
# Create namespace
kubectl create namespace aaroneous

# Create secrets
kubectl create secret generic aaroneous-db-secret \
  --from-literal=username=aaroneous \
  --from-literal=password=secure_password \
  -n aaroneous

# Deploy
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --namespace aaroneous \
  --values values.yaml

# Verify deployment
kubectl get deployment -n aaroneous
kubectl get pods -n aaroneous
kubectl get svc -n aaroneous

# Check logs
kubectl logs -n aaroneous -l app=aaroneous-federation -f

# Port forward
kubectl port-forward -n aaroneous svc/aaroneous-federation 8001:8001

# Scale manually
kubectl scale deployment aaroneous-federation --replicas=5 -n aaroneous

# Check status
kubectl get pods -n aaroneous -o wide

# Update image
kubectl set image deployment/aaroneous-federation \
  aaroneous=your-registry/aaroneous:1.0.1 \
  -n aaroneous

# Rollback if needed
kubectl rollout undo deployment/aaroneous-federation -n aaroneous
```

### Cloud Deployment

#### AWS EKS

```bash
# Create cluster
aws eks create-cluster \
  --name aaroneous-federation \
  --region us-east-1 \
  --version 1.27 \
  --roleArn arn:aws:iam::ACCOUNT:role/eks-service-role \
  --resourcesVpcConfig subnetIds=subnet-xxx,subnet-yyy

# Update kubeconfig
aws eks update-kubeconfig \
  --name aaroneous-federation \
  --region us-east-1

# Deploy
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --values deploy/aws-values.yaml \
  -n aaroneous --create-namespace
```

#### GCP GKE

```bash
# Create cluster
gcloud container clusters create aaroneous-federation \
  --zone us-central1-a \
  --num-nodes 3 \
  --machine-type n2-standard-4

# Get credentials
gcloud container clusters get-credentials aaroneous-federation

# Deploy
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --values deploy/gcp-values.yaml \
  -n aaroneous --create-namespace
```

#### Azure AKS

```bash
# Create resource group
az group create --name aaroneous-rg --location eastus

# Create cluster
az aks create \
  --resource-group aaroneous-rg \
  --name aaroneous-federation \
  --node-count 3

# Get credentials
az aks get-credentials \
  --resource-group aaroneous-rg \
  --name aaroneous-federation

# Deploy
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --values deploy/azure-values.yaml \
  -n aaroneous --create-namespace
```

### Mobile Deployment

See [MOBILE_APP_DEPLOYMENT_GUIDE.md](./MOBILE_APP_DEPLOYMENT_GUIDE.md) for detailed iOS and Android deployment instructions.

```bash
# iOS
cd (mobile app planned)
xcodebuild -scheme Aaroneous -configuration Release

# Android
cd android
./gradlew assembleRelease
```

### Infrastructure-as-Code: Terraform

```bash
# Initialize Terraform
cd deploy/terraform
terraform init

# Plan infrastructure
terraform plan -var-file=environments/production.tfvars -out=plan.tfplan

# Review plan
# ... inspect plan.tfplan ...

# Apply infrastructure
terraform apply plan.tfplan

# Get outputs
terraform output

# Destroy (careful!)
terraform destroy -var-file=environments/production.tfvars
```

---

## Monitoring & Observability

### Health Checks

```bash
# Liveness probe (is service alive?)
curl -f http://localhost:8001/health || exit 1

# Response:
# {
#   "status": "healthy",
#   "uptime_seconds": 3600,
#   "specialists_online": 6,
#   "consensus_working": true
# }

# Readiness probe (is service ready for traffic?)
curl -f http://localhost:8001/ready || exit 1

# Response:
# {
#   "ready": true,
#   "database_connected": true,
#   "redis_connected": true,
#   "peers_connected": 2,
#   "specialists_initialized": true
# }
```

### Metrics

```bash
# Prometheus metrics endpoint
curl http://localhost:8001/metrics | head -50

# Key metrics to monitor:
# - aaroneous_proposals_total - Total proposals submitted
# - aaroneous_consensus_decisions_total - Decisions made
# - aaroneous_consensus_agreement_percentage - Agreement level
# - aaroneous_dna_events_recorded_total - Events recorded
# - aaroneous_memory_bytes - Current memory usage
# - aaroneous_specialist_response_time_ms - Response times
```

### Dashboards

Access Grafana dashboards:

```bash
# Local: http://localhost:3000 (admin:admin)
# Production: https://grafana.aaroneous.example.com

# Dashboards:
# 1. Federation Overview - Throughput, consensus, specialists
# 2. Performance - CPU, memory, cache, GPU
# 3. DNA Bank - Events, patterns, storage
# 4. Enterprise - Audit, compliance, rate limiting
```

### Logs

```bash
# Docker Compose
docker-compose logs -f aaroneous

# Kubernetes
kubectl logs -f -n aaroneous -l app=aaroneous-federation

# Kibana (ELK Stack)
# Access at http://localhost:5601
# Search: app:aaroneous AND level:ERROR
```

### Alerting

```bash
# Slack notifications configured for:
# - High proposal latency (>50ms p95)
# - Low consensus agreement (<80%)
# - High memory usage (>7GB)
# - Peer communication errors
# - DNA event backlog (>50k)
# - Compliance violations

# Check alert status
curl http://localhost:9090/api/v1/alerts
```

---

## Backup & Recovery

### Database Backup

```bash
# Manual RDS snapshot
aws rds create-db-snapshot \
  --db-instance-identifier aaroneous-federation \
  --db-snapshot-identifier aaroneous-backup-$(date +%Y%m%d)

# Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier aaroneous-restored \
  --db-snapshot-identifier aaroneous-backup-20240101
```

### DNA Bank Backup

```bash
# Backup to S3
aws s3 sync /data/dna_bank s3://aaroneous-backups/dna-bank/

# Restore from S3
aws s3 sync s3://aaroneous-backups/dna-bank/ /data/dna_bank/
```

### Kubernetes Backup

```bash
# Install Velero
velero install --provider aws \
  --bucket aaroneous-backups \
  --secret-file ~/velero-credentials

# Create backup
velero backup create aaroneous-$(date +%Y%m%d)

# List backups
velero backup get

# Restore from backup
velero restore create --from-backup aaroneous-20240101
```

---

## Performance Tuning

### Database Tuning

```sql
-- Optimize PostgreSQL
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '2GB';
ALTER SYSTEM SET work_mem = '32MB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';

SELECT pg_reload_conf();

-- Index optimization
CREATE INDEX idx_audit_timestamp ON audit_events(timestamp);
CREATE INDEX idx_audit_user ON audit_events(user_id);
CREATE INDEX idx_dna_confidence ON dna_patterns(confidence);
```

### Redis Optimization

```bash
redis-cli CONFIG SET maxmemory 2gb
redis-cli CONFIG SET maxmemory-policy allkeys-lru
redis-cli CONFIG SET appendonly yes
redis-cli CONFIG REWRITE

# Monitor
redis-cli MONITOR
redis-cli INFO stats
redis-cli INFO memory
```

### Kubernetes Tuning

```yaml
# Increase resource limits
kubectl set resources deployment aaroneous-federation \
  --limits=cpu=4,memory=8Gi \
  --requests=cpu=2,memory=4Gi \
  -n aaroneous

# Adjust autoscaling
kubectl patch hpa aaroneous-federation \
  -p '{"spec":{"maxReplicas":20}}' \
  -n aaroneous
```

---

## CI/CD Pipeline

### GitHub Actions

```bash
# Push to main (production)
git push origin main
# → Runs: tests, builds image, deploys to production

# Push to develop (staging)
git push origin develop
# → Runs: tests, builds image, deploys to staging

# Manual deployment
gh workflow run deploy.yml \
  -f environment=production \
  -f version=1.0.1
```

### Local Testing

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test federation::tests::test_proposal_ranking

# Run with output
cargo test -- --nocapture

# Check code quality
cargo clippy --all-features -- -D warnings
cargo fmt --check
```

---

## Alerting Rules

### Critical Alerts

```yaml
- High proposal latency (p95 > 50ms)
- Low consensus agreement (< 80%)
- Specialist down (count < 5)
- Database connection pool full (> 90%)
- Compliance violation detected
- Rate limit exceeded (> 100 rejections/5m)
```

### Warning Alerts

```yaml
- Medium memory usage (> 6GB)
- High CPU utilization (> 80%)
- DNS event backlog (> 10k pending)
- Cache hit rate low (< 80%)
- Peer communication latency high (> 10ms)
```

### Info Alerts

```yaml
- Deployment started
- Deployment completed
- Backup created
- Config reloaded
- Specialist initialized
```

---

## Troubleshooting

### Common Issues

#### 1. Database Connection Failed
```bash
# Check database is running
docker-compose ps | grep database

# Test connection
psql postgresql://user:pass@localhost:5432/aaroneous_federation

# Check logs
docker-compose logs database
```

#### 2. High Memory Usage
```bash
# Check memory
free -h
docker stats

# Reduce cache size
# Set CACHE_SIZE env var lower

# Check for memory leaks
# Enable profiling: curl http://localhost:8001/debug/pprof/heap
```

#### 3. Slow Proposal Processing
```bash
# Check metrics
curl http://localhost:8001/metrics | grep proposal_latency

# Check CPU
top -b -n 1 | head -20

# Enable profiling
curl http://localhost:8001/debug/pprof/profile?seconds=30 > cpu.prof
cargo flamegraph / samply cpu.prof
```

#### 4. Pod Crashes in Kubernetes
```bash
# Check pod status
kubectl describe pod <pod-name> -n aaroneous

# View logs
kubectl logs <pod-name> -n aaroneous

# Check events
kubectl get events -n aaroneous

# Increase resource requests if OOMKilled
kubectl set resources deployment aaroneous-federation \
  --requests=memory=6Gi -n aaroneous
```

---

## Maintenance Procedures

### Regular Tasks

```bash
# Daily
- Monitor dashboards
- Check alert status
- Review error logs

# Weekly
- Review performance metrics
- Validate backup completion
- Update dependencies

# Monthly
- Run security audit
- Review compliance status
- Test disaster recovery
- Performance analysis

# Quarterly
- Major version updates
- Architecture review
- Capacity planning
```

### Update Procedure

```bash
# 1. Update code
git pull origin main

# 2. Run tests
cargo test --all-features

# 3. Build image
docker build -t aaroneous:1.0.1 .

# 4. Push to registry
docker push your-registry/aaroneous:1.0.1

# 5. Update deployment
helm upgrade aaroneous-federation aaroneous/aaroneous-federation \
  --values values.yaml

# 6. Verify
kubectl rollout status deployment/aaroneous-federation -n aaroneous
```

---

## Support & Resources

- **Documentation:** [FEDERATION_ARCHITECTURE.md](./FEDERATION_ARCHITECTURE.md)
- **API Reference:** [architecture/FEDERATION_README.md](./architecture/FEDERATION_README.md)
- **Performance Guide:** [PHASE_H_OPTIMIZATION.md](./PHASE_H_OPTIMIZATION.md)
- **Mobile Guide:** [MOBILE_APP_DEPLOYMENT_GUIDE.md](./MOBILE_APP_DEPLOYMENT_GUIDE.md)
- **GitHub Issues:** https://github.com/anomalyco/aaroneous/issues
- **Community:** https://github.com/anomalyco/aaroneous/discussions

---

**Aaroneous Federation - Ready for Production! 🚀**

