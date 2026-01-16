#!/bin/bash
# Build script for CAGE sandbox container image

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="${IMAGE_NAME:-cage-sandbox}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo "Building CAGE sandbox container image..."
echo "Image: ${IMAGE_NAME}:${IMAGE_TAG}"

# Detect container runtime
if command -v podman &> /dev/null; then
    RUNTIME="podman"
elif command -v docker &> /dev/null; then
    RUNTIME="docker"
else
    echo "Error: Neither podman nor docker found in PATH"
    exit 1
fi

echo "Using container runtime: ${RUNTIME}"

# Build the image
cd "${SCRIPT_DIR}"
${RUNTIME} build \
    -t "${IMAGE_NAME}:${IMAGE_TAG}" \
    -f Containerfile \
    .

echo ""
echo "Build complete!"
echo "Image: ${IMAGE_NAME}:${IMAGE_TAG}"
echo ""
echo "To test the image:"
echo "  ${RUNTIME} run --rm -it ${IMAGE_NAME}:${IMAGE_TAG} python -c \"import pandas; print('OK')\""
echo ""
echo "To run with security options (as the orchestrator will):"
echo "  ${RUNTIME} run --rm -it \\"
echo "    --read-only \\"
echo "    --tmpfs /tmp:rw,noexec,nosuid,size=100m \\"
echo "    --security-opt no-new-privileges \\"
echo "    --cap-drop ALL \\"
echo "    --network none \\"
echo "    --memory 1g \\"
echo "    --cpus 1.0 \\"
echo "    --pids-limit 100 \\"
echo "    ${IMAGE_NAME}:${IMAGE_TAG} python -c \"print('Secure sandbox!')\""
