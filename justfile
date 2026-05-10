# WorldVPN — Task Runner (https://just.systems)
# Usage: just <recipe>
# Install: cargo install just

# Default: list all available recipes
default:
    @just --list

# ─── Development ─────────────────────────────────────────────
# Run the API backend server (HTTP mode)
dev-backend:
    @echo "🚀 Starting WorldVPN API backend..."
    cd backend/server && cargo run -p vpn-server

# Open the Desktop GUI in dev mode (Tauri + React)
dev-desktop:
    @echo "🖥️  Starting Desktop GUI..."
    cd frontend/worldvpn-gui && bun tauri dev

# Run the mobile app on a connected device/emulator
dev-mobile:
    @echo "📱 Starting Mobile App..."
    cd frontend/worldvpn-mobile && flutter run

# ─── Build ───────────────────────────────────────────────────
# Build the entire Rust workspace (release)
build:
    @echo "🔨 Building Rust workspace..."
    cargo build --workspace --release

# Build only the backend server binary
build-backend:
    @echo "🔨 Building backend server..."
    cargo build -p vpn-server --release

# Build the Desktop GUI (Tauri)
build-desktop:
    @echo "🔨 Building Desktop GUI..."
    cd frontend/worldvpn-gui && bun tauri build

# Build the Mobile app for Android
build-mobile-android:
    @echo "🔨 Building Android APK..."
    cd frontend/worldvpn-mobile && flutter build apk --release

# Build the Mobile app for iOS
build-mobile-ios:
    @echo "🔨 Building iOS app..."
    cd frontend/worldvpn-mobile && flutter build ios --release

# Build the vpn-go sing-box binary
build-go:
    @echo "🔨 Building vpn-go..."
    cd vpn-go && make build

# ─── Testing ─────────────────────────────────────────────────
# Run ALL tests (Rust workspace + Flutter)
test: test-rust test-flutter

# Run all Rust workspace tests
test-rust:
    @echo "🧪 Running Rust tests..."
    cargo test --workspace --lib

# Run only vpn-core unit tests
test-core:
    @echo "🧪 Running vpn-core tests..."
    cargo test -p vpn-core --lib

# Run only backend server tests
test-backend:
    @echo "🧪 Running backend tests..."
    cargo test -p vpn-server --lib

# Run Flutter widget and unit tests
test-flutter:
    @echo "🧪 Running Flutter tests..."
    cd frontend/worldvpn-mobile && flutter test

# ─── Code Quality ────────────────────────────────────────────
# Lint the Rust workspace (clippy)
lint:
    @echo "🔍 Running Clippy..."
    cargo clippy --workspace --all-targets -- -D warnings

# Format all Rust code
fmt:
    @echo "✨ Formatting Rust code..."
    cargo fmt --all

# Check formatting without applying changes (CI)
fmt-check:
    cargo fmt --all -- --check

# ─── Security ────────────────────────────────────────────────
# Audit Rust dependencies for known CVEs
audit:
    @echo "🔒 Running cargo audit..."
    cargo audit

# ─── Infrastructure ──────────────────────────────────────────
# Start local infrastructure (PostgreSQL via Docker Compose)
infra-up:
    @echo "🐳 Starting local infrastructure..."
    docker-compose up -d

# Stop local infrastructure
infra-down:
    docker-compose down

# Run database migrations manually
migrate:
    @echo "🔄 Running database migrations..."
    cd backend/server && sqlx migrate run

# Generate dev TLS certificates
gen-certs:
    @echo "🔑 Generating dev certificates..."
    ./scripts/generate-dev-certs.sh

# ─── CI ──────────────────────────────────────────────────────
# Run the full CI pipeline (format check + lint + tests + audit)
ci: fmt-check lint test audit
    @echo "✅ CI pipeline passed!"
