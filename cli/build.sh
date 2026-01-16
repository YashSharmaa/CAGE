#!/bin/bash
# Build script for CAGE CLI

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "Building CAGE CLI..."

# Build for current platform
cargo build --release

echo ""
echo "Build complete!"
echo "Binary: target/release/cage"
echo ""
echo "Install with:"
echo "  sudo cp target/release/cage /usr/local/bin/"
echo ""
echo "Or add to PATH:"
echo "  export PATH=\"\$PATH:$(pwd)/target/release\""
echo ""
echo "Usage examples:"
echo "  cage execute \"print('Hello')\" "
echo "  cage execute -l javascript \"console.log('Hi')\""
echo "  cage execute @script.py"
echo "  cage upload data.csv"
echo "  cage list"
echo "  cage health"
