#!/bin/bash
# Build script for CAGE Julia sandbox container image

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="${IMAGE_NAME:-cage-sandbox-julia}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo "Building CAGE Julia sandbox container image..."
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
echo "  ${RUNTIME} run --rm -it ${IMAGE_NAME}:${IMAGE_TAG} julia --version"
