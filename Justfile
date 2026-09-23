set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

jobs := env_var_or_default("AUGUSTUS_JOBS", "12")
asset_jobs := env_var_or_default("AUGUSTUS_ASSET_JOBS", "12")
native_package_command := if os() == "windows" { "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-native.ps1" } else { "bash scripts/package-native.sh" }
web_package_command := if os() == "windows" { "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-web.ps1" } else { "bash scripts/package-web.sh" }
packaging_check_command := if os() == "windows" { "powershell -NoProfile -ExecutionPolicy Bypass -File tests/scripts/packaging.ps1" } else { "bash tests/scripts/packaging.sh" }

# List available project commands.
default:
    @just --list

# Build and launch the Augustus menu shell.
run *args:
    cargo run --package augustus --bin augustus -j{{ jobs }} {{ args }}

# Build and launch an optimized Augustus executable.
run-release *args:
    cargo run --package augustus --release --bin augustus -j{{ jobs }} {{ args }}

# Build Augustus for the current host.
build:
    cargo build --package augustus --bin augustus -j{{ jobs }}

build-release:
    cargo build --package augustus --release --bin augustus -j{{ jobs }}

check:
    cargo check --package augustus --bin augustus -j{{ jobs }}

check-wasm:
    cargo check --package augustus --target wasm32-unknown-unknown --bin augustus -j{{ jobs }}

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --package augustus --bin augustus -j{{ jobs }} -- -D warnings

assets:
    cargo run --package augustus --features asset-pipeline --bin build-assets -j{{ jobs }} -- --jobs {{ asset_jobs }}

assets-force:
    cargo run --package augustus --features asset-pipeline --bin build-assets -j{{ jobs }} -- --force --jobs {{ asset_jobs }}

assets-check:
    cargo run --package augustus --features asset-pipeline --bin build-assets -j{{ jobs }} -- --check --jobs {{ asset_jobs }}

ci: fmt-check lint assets-check check-wasm packaging-check

packaging-check:
    {{ packaging_check_command }}

package-native:
    {{ native_package_command }}

package-web:
    {{ web_package_command }}
