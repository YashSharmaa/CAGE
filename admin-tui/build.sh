#!/bin/bash
# Build script for CAGE Admin TUI

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "Building CAGE Admin TUI..."

# Get dependencies
go mod tidy

# Build for current platform
go build -o cage-tui .

echo "Built: cage-tui"

# Optionally build for multiple platforms
if [[ "${BUILD_ALL:-false}" == "true" ]]; then
    echo "Building for all platforms..."

    GOOS=linux GOARCH=amd64 go build -o cage-tui-linux-amd64 .
    GOOS=linux GOARCH=arm64 go build -o cage-tui-linux-arm64 .
    GOOS=darwin GOARCH=amd64 go build -o cage-tui-darwin-amd64 .
    GOOS=darwin GOARCH=arm64 go build -o cage-tui-darwin-arm64 .
    GOOS=windows GOARCH=amd64 go build -o cage-tui-windows-amd64.exe .

    echo "Built all platforms"
fi

echo ""
echo "Usage:"
echo "  ./cage-tui --api http://localhost:8080 --token YOUR_ADMIN_TOKEN"
echo ""
echo "Or set environment variables:"
echo "  export CAGE_API_URL=http://localhost:8080"
echo "  export CAGE_ADMIN_TOKEN=your_token"
echo "  ./cage-tui"
