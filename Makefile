.PHONY: build build-sdk build-server dev clean

# Build everything: SDK first (produces pulse.min.js), then Rust binary
build: build-sdk build-server

# Build the TypeScript SDK and copy script to Rust static dir
build-sdk:
	cd sdk && npm install && npm run build
	cp sdk/dist/pulse.min.global.js crates/pulse-server/static/pulse.min.js

# Build the Rust server (release)
build-server:
	cargo build --release -p pulse-server

# Development: run with hot-reload (requires cargo-watch)
dev:
	cargo run -p pulse-server

# Run in development with auto-rebuild
dev-watch:
	cargo watch -x 'run -p pulse-server'

# Clean all build artifacts
clean:
	cargo clean
	rm -rf sdk/dist sdk/node_modules

# Run cargo check
check:
	cargo check

# Build Docker image
docker:
	docker build -t pulse-analytics .

# Run with Docker Compose (local dev with PostgreSQL + Redis)
docker-up:
	docker compose up --build

docker-down:
	docker compose down

# Publish SDK to npm (requires npm login)
publish-sdk:
	cd sdk && npm publish --access public
