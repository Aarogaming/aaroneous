# Aaroneous Federation: Comprehensive Deployment Guide

## Overview

This guide provides production-ready deployment strategies for Aaroneous Federation across multiple platforms and configurations.

## Table of Contents

1. [Local Development](#local-development)
2. [Docker Deployment](#docker-deployment)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Cloud Deployment (AWS/GCP/Azure)](#cloud-deployment)
5. [Multi-Hive Federation](#multi-hive-federation)
6. [Monitoring & Observability](#monitoring--observability)
7. [Backup & Recovery](#backup--recovery)
8. [Performance Tuning](#performance-tuning)

---

## Local Development

### Prerequisites
- Rust 1.70+
- Docker & Docker Compose
- PostgreSQL 15+
- Redis 7.0+

### Setup with Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  aaroneous:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8001:8001"
    environment:
      - LOG_LEVEL=debug
      - DATABASE_URL=postgresql://user:password@db:5432/aaroneous
      - REDIS_URL=redis://cache:6379
    depends_on:
      - db
      - cache
    volumes:
      - ./models:/app/models
      - ./dna_bank:/app/dna_bank

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=aaroneous
      - POSTGRES_PASSWORD=secure_password
      - POSTGRES_DB=aaroneous_federation
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  cache:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

### Build & Run

```bash
# Build the project
cargo build --release

# Start with Docker Compose
docker-compose up -d

# Check logs
docker-compose logs -f aaroneous

# Run tests
cargo test --lib federation

# Access
# API: http://localhost:8001
# DB: localhost:5432
# Cache: localhost:6379
```

---

## Docker Deployment

### Single-Hive Docker Image

```dockerfile
# Dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

# Build optimized release
RUN cargo build --release --features "federation"
RUN strip target/release/aaroneous

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/aaroneous /usr/local/bin/

# Create volumes
RUN mkdir -p /data/models /data/dna_bank /data/logs

VOLUME ["/data/models", "/data/dna_bank", "/data/logs"]

EXPOSE 8001

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD aaroneous --health-check || exit 1

# Run
ENTRYPOINT ["aaroneous"]
CMD ["--config", "/data/config.toml"]
```

### Build & Push

```bash
# Build
docker build -t aaroneous-federation:1.0.0 .

# Tag for registry
docker tag aaroneous-federation:1.0.0 \
  your-registry.azurecr.io/aaroneous-federation:1.0.0

# Push
docker push your-registry.azurecr.io/aaroneous-federation:1.0.0

# Run container
docker run \
  -v ./models:/data/models \
  -v ./dna_bank:/data/dna_bank \
  -p 8001:8001 \
  -e DATABASE_URL=postgresql://user:pass@db:5432/aaroneous \
  -e REDIS_URL=redis://cache:6379 \
  aaroneous-federation:1.0.0
```

---

## Kubernetes Deployment

### Helm Values for Multi-Hive

```yaml
# values.yaml
replicaCount: 3

image:
  repository: your-registry/aaroneous-federation
  tag: "1.0.0"
  pullPolicy: IfNotPresent

resources:
  requests:
    cpu: 2
    memory: 4Gi
  limits:
    cpu: 4
    memory: 8Gi

# Specialist node affinity
nodeSelector:
  workload: specialist

# GPU support
gpu:
  enabled: true
  type: nvidia
  count: 1

# Database configuration
database:
  host: postgresql.default.svc.cluster.local
  port: 5432
  name: aaroneous_federation
  credentials:
    secretName: aaroneous-db-secret

# Redis cache
cache:
  enabled: true
  host: redis.default.svc.cluster.local
  port: 6379

# Persistent volumes
persistence:
  enabled: true
  storageClass: gp2
  size: 100Gi

# Service configuration
service:
  type: LoadBalancer
  port: 8001
  targetPort: 8001

# Auto-scaling
autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

# Ingress
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: aaroneous.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: aaroneous-tls
      hosts:
        - aaroneous.example.com

# Monitoring
monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
    interval: 30s
```

### Deploy Multi-Hive Cluster

```bash
# Add Helm repository
helm repo add aaroneous https://charts.aaroneous.ai
helm repo update

# Install multi-hive cluster
helm install aaroneous-federation aaroneous/aaroneous-federation \
  --namespace aaroneous \
  --create-namespace \
  -f values.yaml

# Verify deployment
kubectl get pods -n aaroneous
kubectl logs -n aaroneous -l app=aaroneous-federation -f

# Check multi-hive status
kubectl port-forward -n aaroneous svc/aaroneous-federation 8001:8001
curl http://localhost:8001/api/v1/cluster/status

# Scale cluster
kubectl scale deployment aaroneous-federation \
  --replicas=5 \
  -n aaroneous
```

---

## Cloud Deployment

### AWS EKS

```bash
# Create EKS cluster with Terraform
cd deploy/terraform
terraform init
terraform apply -var="environment=production"

# Configure kubectl
aws eks update-kubeconfig \
  --region us-east-1 \
  --name aaroneous-federation

# Deploy with Helm
helm install aaroneous-federation ./helm \
  --values deploy/aws-values.yaml \
  -n aaroneous --create-namespace
```

### GCP GKE

```bash
# Create GKE cluster
gcloud container clusters create aaroneous-federation \
  --zone us-central1-a \
  --num-nodes 3 \
  --machine-type n2-standard-4 \
  --enable-autoscaling \
  --min-nodes 3 \
  --max-nodes 10 \
  --enable-ip-alias

# Get credentials
gcloud container clusters get-credentials aaroneous-federation

# Deploy
helm install aaroneous-federation ./helm \
  --values deploy/gcp-values.yaml \
  -n aaroneous --create-namespace
```

### Azure AKS

```bash
# Create resource group
az group create \
  --name aaroneous-rg \
  --location eastus

# Create AKS cluster
az aks create \
  --resource-group aaroneous-rg \
  --name aaroneous-federation \
  --node-count 3 \
  --vm-set-type VirtualMachineScaleSets \
  --enable-managed-identity \
  --enable-addons monitoring

# Get credentials
az aks get-credentials \
  --resource-group aaroneous-rg \
  --name aaroneous-federation

# Deploy
helm install aaroneous-federation ./helm \
  --values deploy/azure-values.yaml \
  -n aaroneous --create-namespace
```

---

## Multi-Hive Federation

### Deploying Multiple Independent Hives

```yaml
# helm-values-hive-1.yaml
cluster:
  name: hive-1
  region: us-east-1
  node_count: 3

federation:
  enabled: true
  peer_hives:
    - name: hive-2
      address: hive-2.example.com:8001
    - name: hive-3
      address: hive-3.example.com:8001

# helm-values-hive-2.yaml
cluster:
  name: hive-2
  region: us-west-2
  node_count: 3

federation:
  enabled: true
  peer_hives:
    - name: hive-1
      address: hive-1.example.com:8001
    - name: hive-3
      address: hive-3.example.com:8001
```

Deploy all three:

```bash
# Hive 1
helm install hive-1 ./helm \
  -f deploy/helm-values-hive-1.yaml \
  --namespace hive-1 --create-namespace

# Hive 2
helm install hive-2 ./helm \
  -f deploy/helm-values-hive-2.yaml \
  --namespace hive-2 --create-namespace

# Hive 3
helm install hive-3 ./helm \
  -f deploy/helm-values-hive-3.yaml \
  --namespace hive-3 --create-namespace

# Verify federation
kubectl get pods -n hive-1
kubectl logs -n hive-1 -l app=aaroneous-federation \
  | grep "consensus\|federation"
```

---

## Monitoring & Observability

### Prometheus Metrics

```yaml
# prometheus-config.yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'aaroneous-federation'
    static_configs:
      - targets: ['localhost:8001']
    metrics_path: '/metrics'
```

### Grafana Dashboard

Create dashboard with panels for:
- Proposal throughput
- Consensus decisions/sec
- Multi-hive latency
- Specialist health status
- Memory/CPU usage
- DNA Bank event rate
- Audit log events

### ELK Stack for Logs

```yaml
# filebeat-config.yaml
filebeat.inputs:
- type: container
  enabled: true
  paths:
    - '/var/lib/docker/containers/*/*.log'

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "aaroneous-%{+yyyy.MM.dd}"
```

---

## Backup & Recovery

### Database Backups

```bash
# Daily automated RDS backups
aws rds create-db-snapshot \
  --db-instance-identifier aaroneous-federation \
  --db-snapshot-identifier aaroneous-$(date +%Y%m%d)

# Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier aaroneous-restored \
  --db-snapshot-identifier aaroneous-20240101
```

### DNA Bank Backups

```bash
# S3 backup
aws s3 sync /data/dna_bank \
  s3://aaroneous-backups/dna-bank/$(date +%Y%m%d)/

# Daily cron job
0 2 * * * aws s3 sync /data/dna_bank \
  s3://aaroneous-backups/dna-bank/$(date +\%Y\%m\%d)/
```

### Kubernetes Cluster Backup

```bash
# Velero for Kubernetes backup
velero backup create aaroneous-$(date +%Y%m%d) \
  --include-namespaces aaroneous

# List backups
velero backup get

# Restore
velero restore create --from-backup aaroneous-20240101
```

---

## Performance Tuning

### Kubernetes Resource Optimization

```yaml
# Production values
resources:
  requests:
    cpu: 2
    memory: 4Gi
  limits:
    cpu: 4
    memory: 8Gi

# Pod Disruption Budget (high availability)
podDisruptionBudget:
  minAvailable: 2

# Network Policy (security)
networkPolicy:
  enabled: true
  policyTypes:
    - Ingress
    - Egress
```

### Database Tuning

```sql
-- PostgreSQL DNA Bank optimization
ALTER TABLE audit_events SET (fillfactor=90);
CREATE INDEX idx_events_timestamp ON audit_events(timestamp);
CREATE INDEX idx_events_user ON audit_events(user_id);
CREATE INDEX idx_events_level ON audit_events(level);

-- Tune parameters
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '2GB';
ALTER SYSTEM SET work_mem = '32MB';

SELECT pg_reload_conf();
```

### Redis Optimization

```bash
# Memory management
redis-cli CONFIG SET maxmemory 2gb
redis-cli CONFIG SET maxmemory-policy allkeys-lru
redis-cli CONFIG SET appendonly yes
redis-cli CONFIG REWRITE

# Monitor
redis-cli MONITOR
redis-cli INFO stats
```

---

## Maintenance

### Health Checks

```bash
# Cluster status
curl http://aaroneous.example.com/health

# Database connectivity
kubectl exec -it deployment/aaroneous-federation \
  -- pg_isready -h postgresql

# Redis connectivity
kubectl exec -it deployment/aaroneous-federation \
  -- redis-cli -h redis PING
```

### Updates

```bash
# Update Helm chart
helm repo update
helm upgrade aaroneous-federation ./helm \
  -f values.yaml \
  -n aaroneous

# Rolling update
kubectl rollout status deployment/aaroneous-federation -n aaroneous
kubectl rollout undo deployment/aaroneous-federation -n aaroneous
```

---

## Summary

This deployment guide provides:

- ✅ Local development with Docker Compose
- ✅ Docker container images
- ✅ Kubernetes Helm charts for all platforms
- ✅ Multi-hive federation setup
- ✅ Cloud deployment (AWS/GCP/Azure)
- ✅ Monitoring and observability
- ✅ Backup and recovery procedures
- ✅ Performance tuning recommendations
- ✅ High availability configuration
- ✅ Security best practices

For production deployment, follow the infrastructure-as-code approach using Terraform and Helm for reproducible, versioned infrastructure.

---

**Aaroneous Federation is ready for production deployment! 🚀**
