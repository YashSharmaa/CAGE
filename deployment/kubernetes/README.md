# CAGE Kubernetes Deployment

## Prerequisites

- Kubernetes cluster 1.24+
- kubectl configured
- Container registry access
- Podman or Docker for building images

## Quick Deploy

```bash
# 1. Build and push images
cd ../../sandbox
podman build -t your-registry.com/cage-sandbox:1.0.0 .
podman push your-registry.com/cage-sandbox:1.0.0

cd ../orchestrator
podman build -t your-registry.com/cage-orchestrator:1.0.0 .
podman push your-registry.com/cage-orchestrator:1.0.0

# 2. Create namespace and secrets
kubectl apply -f secrets.yaml
# Edit secrets.yaml first to set real tokens!

# 3. Deploy
kubectl apply -f deployment.yaml

# 4. Verify
kubectl -n cage-system get pods
kubectl -n cage-system logs -f deployment/cage-orchestrator
```

## Configuration

### Secrets

Edit `secrets.yaml` and generate secure tokens:

```bash
# JWT secret
openssl rand -base64 32

# Admin token
openssl rand -base64 32
```

### Resource Limits

Adjust in `deployment.yaml`:

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"
```

### Storage

Default uses 100Gi PVC. Adjust in `deployment.yaml`:

```yaml
spec:
  resources:
    requests:
      storage: 100Gi  # Change this
```

## Access

```bash
# Port forward for local access
kubectl -n cage-system port-forward svc/cage-orchestrator 8080:8080

# Test
curl http://localhost:8080/health
```

## Monitoring

```bash
# View logs
kubectl -n cage-system logs -f deployment/cage-orchestrator

# Check resources
kubectl -n cage-system top pods

# View events
kubectl -n cage-system get events --sort-by='.lastTimestamp'
```

## Security Notes

- Orchestrator runs as privileged (required for Podman)
- NetworkPolicy restricts egress
- Uses dedicated ServiceAccount
- Secrets stored in Kubernetes secrets

## Troubleshooting

**Pods not starting:**
```bash
kubectl -n cage-system describe pod <pod-name>
kubectl -n cage-system logs <pod-name>
```

**Podman issues:**
```bash
kubectl -n cage-system exec -it deployment/cage-orchestrator -- podman ps
```

**Storage issues:**
```bash
kubectl -n cage-system get pvc
kubectl describe pvc cage-data -n cage-system
```
