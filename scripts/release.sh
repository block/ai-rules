#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_DIR/dist"

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)
echo "Building ai-rules cross-platform release binaries..."

cd "$PROJECT_DIR"
source bin/activate-hermit

if ! command -v cross &> /dev/null; then
    echo "Installing cross..."
    cargo install cross --git https://github.com/cross-rs/cross
fi

# Install required targets
echo "Installing Rust targets..."
for target in "${TARGETS[@]}"; do
    echo "Adding target: $target"
    rustup target add "$target"
done

echo "Cleaning previous builds..."
cargo clean --release
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    cross build --release --target "$target"
    binary_name="ai-rules"
    
    target_dir="$DIST_DIR/$target"
    mkdir -p "$target_dir"
    
    cp "$PROJECT_DIR/target/$target/release/$binary_name" "$target_dir/"
    
    echo "Creating archive for $target..."
    cd "$DIST_DIR"
    tar -czf "ai-rules-$target.tar.gz" "$target/"
    cd "$PROJECT_DIR"
    
    echo "✅ Built $target"
done

echo "🎉 All cross-platform builds complete!"
echo "📦 Artifacts available in: $DIST_DIR"
ls -la "$DIST_DIR"

