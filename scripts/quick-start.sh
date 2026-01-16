#!/bin/bash
# CAGE Quick Start Script
# This script builds and starts CAGE for testing

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"

cd "${PROJECT_ROOT}"

echo "=========================================="
echo "CAGE - Quick Start"
echo "=========================================="
echo ""

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v podman &> /dev/null && ! command -v docker &> /dev/null; then
    echo "Error: Neither podman nor docker found. Please install podman."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust: https://rustup.rs"
    exit 1
fi

CONTAINER_CMD="podman"
if ! command -v podman &> /dev/null; then
    CONTAINER_CMD="docker"
fi

echo "Using container runtime: ${CONTAINER_CMD}"
echo ""

# Build sandbox image
echo "Step 1: Building sandbox container image..."
cd sandbox
chmod +x build.sh
./build.sh
cd "${PROJECT_ROOT}"
echo ""

# Build orchestrator
echo "Step 2: Building orchestrator..."
cd orchestrator
cargo build --release 2>&1 | tail -5
cd "${PROJECT_ROOT}"
echo ""

# Create data directory
echo "Step 3: Setting up data directory..."
DATA_DIR="${PROJECT_ROOT}/data"
mkdir -p "${DATA_DIR}"
echo "Data directory: ${DATA_DIR}"
echo ""

# Create config
echo "Step 4: Creating configuration..."
CONFIG_DIR="${PROJECT_ROOT}/config"
mkdir -p "${CONFIG_DIR}"

if [[ ! -f "${CONFIG_DIR}/cage.yaml" ]]; then
    # Generate a simple dev config
    cat > "${CONFIG_DIR}/cage.yaml" << 'EOF'
# CAGE Development Configuration
host: "127.0.0.1"
port: 8080
log_level: "info"
data_dir: "./data"
sandbox_image: "cage-sandbox:latest"
stop_containers_on_shutdown: true

default_limits:
  max_memory_mb: 512
  max_cpus: 1.0
  max_pids: 50
  max_execution_seconds: 30
  max_disk_mb: 512

security:
  jwt_secret: "development-only-change-in-production-32chars"
  admin_token: "dev-admin-token"

admin:
  enabled: true
  require_auth: false
  admin_users:
    - "admin"
EOF
    echo "Created development config at ${CONFIG_DIR}/cage.yaml"
else
    echo "Config already exists at ${CONFIG_DIR}/cage.yaml"
fi
echo ""

# Start orchestrator
echo "Step 5: Starting orchestrator..."
echo ""
echo "=========================================="
echo "CAGE is ready!"
echo "=========================================="
echo ""
echo "API URL:     http://127.0.0.1:8080"
echo "Health:      http://127.0.0.1:8080/health"
echo "API Docs:    See api/openapi.yaml"
echo ""
echo "Quick test with curl:"
echo '  curl http://127.0.0.1:8080/health'
echo ""
echo '  curl -X POST http://127.0.0.1:8080/api/v1/execute \'
echo '    -H "Content-Type: application/json" \'
echo '    -H "X-API-Key: dev_testuser" \'
echo '    -d '\''{"code": "print(\"Hello from CAGE!\")"}'\'''
echo ""
echo "Starting server (Ctrl+C to stop)..."
echo ""

cd "${PROJECT_ROOT}"
CAGE_DATA_DIR="${DATA_DIR}" ./orchestrator/target/release/cage-orchestrator
