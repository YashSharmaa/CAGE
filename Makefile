# CAGE - Contained AI-generated Code Execution
# Main Makefile for building all components

.PHONY: all build build-orchestrator build-sandbox build-tui clean test run help

# Default target
all: build

# Build all components
build: build-sandbox build-orchestrator build-tui build-cli
	@echo "All components built successfully!"

# Build all sandbox container images
build-sandbox: build-sandbox-python build-sandbox-r build-sandbox-julia build-sandbox-typescript build-sandbox-ruby build-sandbox-go build-sandbox-wasm
	@echo "All sandbox images built successfully!"

# Build individual sandbox images
build-sandbox-python:
	@echo "Building Python sandbox..."
	cd sandbox && chmod +x build.sh && ./build.sh

build-sandbox-r:
	@echo "Building R sandbox..."
	cd sandbox-r && chmod +x build.sh && ./build.sh

build-sandbox-julia:
	@echo "Building Julia sandbox..."
	cd sandbox-julia && chmod +x build.sh && ./build.sh

build-sandbox-typescript:
	@echo "Building TypeScript/Deno sandbox..."
	cd sandbox-typescript && chmod +x build.sh && ./build.sh

build-sandbox-ruby:
	@echo "Building Ruby sandbox..."
	cd sandbox-ruby && chmod +x build.sh && ./build.sh

build-sandbox-go:
	@echo "Building Go sandbox..."
	cd sandbox-go && chmod +x build.sh && ./build.sh

build-sandbox-wasm:
	@echo "Building WebAssembly sandbox..."
	cd sandbox-wasm && chmod +x build.sh && ./build.sh

# Build the orchestrator
build-orchestrator:
	@echo "Building orchestrator..."
	cd orchestrator && cargo build --release
	@echo "Orchestrator binary: orchestrator/target/release/cage-orchestrator"

# Build the admin TUI
build-tui:
	@echo "Building admin TUI..."
	cd admin-tui && chmod +x build.sh && ./build.sh

# Build the CLI tool
build-cli:
	@echo "Building CLI tool..."
	cd cli && chmod +x build.sh && cargo build --release
	@echo "CLI binary: cli/target/release/cage"

# Run the orchestrator (development mode)
run:
	@echo "Starting orchestrator in development mode..."
	cd orchestrator && RUST_LOG=debug cargo run

# Run with release build
run-release:
	@echo "Starting orchestrator..."
	./orchestrator/target/release/cage-orchestrator

# Run tests
test: test-orchestrator
	@echo "All tests passed!"

test-orchestrator:
	@echo "Running orchestrator tests..."
	cd orchestrator && cargo test

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cd orchestrator && cargo clean
	rm -f admin-tui/cage-tui*
	@echo "Clean complete"

# Development setup
dev-setup:
	@echo "Setting up development environment..."
	# Create data directory
	mkdir -p /tmp/cage/data
	# Copy config
	mkdir -p config
	cp config/cage.yaml.example config/cage.yaml 2>/dev/null || true
	@echo "Development environment ready"

# Generate API client from OpenAPI spec
generate-client:
	@echo "Generating API clients from OpenAPI spec..."
	@echo "Install openapi-generator-cli first: npm install @openapitools/openapi-generator-cli -g"
	# Python client
	openapi-generator-cli generate -i api/openapi.yaml -g python -o clients/python
	# TypeScript client
	openapi-generator-cli generate -i api/openapi.yaml -g typescript-fetch -o clients/typescript
	@echo "API clients generated"

# Validate OpenAPI spec
validate-api:
	@echo "Validating OpenAPI specification..."
	@command -v openapi-generator-cli >/dev/null 2>&1 && \
		openapi-generator-cli validate -i api/openapi.yaml || \
		echo "Install openapi-generator-cli to validate: npm install @openapitools/openapi-generator-cli -g"

# Docker/Podman compose for testing
compose-up:
	@echo "Starting CAGE with podman-compose..."
	podman-compose up -d

compose-down:
	podman-compose down

# Help
help:
	@echo "CAGE - Contained AI-generated Code Execution"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all              Build all components (default)"
	@echo "  build            Build all components"
	@echo "  build-sandbox    Build the sandbox container image"
	@echo "  build-orchestrator Build the Rust orchestrator"
	@echo "  build-tui        Build the Go admin TUI"
	@echo "  run              Run orchestrator in development mode"
	@echo "  run-release      Run orchestrator release build"
	@echo "  test             Run all tests"
	@echo "  clean            Clean build artifacts"
	@echo "  dev-setup        Set up development environment"
	@echo "  validate-api     Validate OpenAPI specification"
	@echo "  generate-client  Generate API clients from OpenAPI"
	@echo "  help             Show this help message"
