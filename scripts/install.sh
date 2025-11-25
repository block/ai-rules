#!/bin/bash

set -euo pipefail

# Installation script for ai-rules
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | bash
#   Or with specific version:
#   curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | VERSION=v0.0.25 bash

REPO_OWNER="block"
REPO_NAME="ai-rules"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"

detect_platform() {
    local os=""
    local arch=""

    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="darwin"
            ;;
        *)
            echo "ERROR: Unsupported operating system: $(uname -s)" >&2
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        arm64|aarch64)
            arch="aarch64"
            ;;
        *)
            echo "ERROR: Unsupported architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac

    if [ "$os" = "linux" ]; then
        echo "${arch}-unknown-linux-gnu"
    else
        echo "${arch}-apple-darwin"
    fi
}

get_latest_version() {
    local latest_url="https://github.com/$REPO_OWNER/$REPO_NAME/releases/latest"
    local version

    version=$(curl -fsSLI "$latest_url" | grep -i "^location:" | sed -E 's/.*\/tag\/([^[:space:]]+).*/\1/' | tr -d '\r')

    if [ -z "$version" ]; then
        echo "ERROR: Failed to determine latest release version" >&2
        exit 1
    fi

    echo "$version"
}

install_binary() {
    local version="$1"
    local target="$2"
    local filename="ai-rules-${version}-${target}.tar.gz"
    local download_url="https://github.com/$REPO_OWNER/$REPO_NAME/releases/download/${version}/${filename}"
    local tmp_dir
    tmp_dir=$(mktemp -d)

    echo "Downloading ai-rules ${version} for ${target}..."

    if ! curl -fsSL "$download_url" -o "$tmp_dir/$filename"; then
        echo "ERROR: Failed to download from $download_url" >&2
        rm -rf "$tmp_dir"
        exit 1
    fi

    checksum_url="${download_url}.sha256"
    if ! curl -fsSL "$checksum_url" -o "$tmp_dir/$filename.sha256"; then
        echo "WARNING: Could not download checksum file, skipping verification"
    else
        echo "Verifying checksum..."

        expected=$(cat "$tmp_dir/$filename.sha256" | cut -d' ' -f1)

        if command -v sha256sum &> /dev/null; then
            actual=$(sha256sum "$tmp_dir/$filename" | cut -d' ' -f1)
        elif command -v shasum &> /dev/null; then
            actual=$(shasum -a 256 "$tmp_dir/$filename" | cut -d' ' -f1)
        else
            echo "WARNING: No checksum tool found, skipping verification"
            actual=""
        fi

        if [ -n "$actual" ]; then
            if [ "$actual" = "$expected" ]; then
                echo "Checksum verified successfully"
            else
                echo "ERROR: Checksum verification failed!" >&2
                echo "  Expected: $expected" >&2
                echo "  Got:      $actual" >&2
                rm -rf "$tmp_dir"
                exit 1
            fi
        fi
    fi

    echo "Extracting binary..."
    if ! tar -xzf "$tmp_dir/$filename" -C "$tmp_dir"; then
        echo "ERROR: Failed to extract archive" >&2
        rm -rf "$tmp_dir"
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"

    if ! cp "$tmp_dir/ai-rules" "$INSTALL_DIR/ai-rules"; then
        echo "ERROR: Failed to copy binary to $INSTALL_DIR" >&2
        rm -rf "$tmp_dir"
        exit 1
    fi

    chmod +x "$INSTALL_DIR/ai-rules"

    rm -rf "$tmp_dir"

    echo "Successfully installed ai-rules to $INSTALL_DIR/ai-rules"
}

verify_installation() {
    if ! command -v ai-rules &> /dev/null; then
        echo "WARNING: ai-rules was installed to $INSTALL_DIR but is not in your PATH"
        echo "WARNING: Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
        return 1
    fi

    local installed_version
    installed_version=$(ai-rules --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    echo "ai-rules $installed_version is now available"
    return 0
}

main() {
    echo "Installing ai-rules..."

    for cmd in curl tar; do
        if ! command -v "$cmd" &> /dev/null; then
            echo "ERROR: Required command not found: $cmd" >&2
            exit 1
        fi
    done

    local target
    target=$(detect_platform)
    echo "Detected platform: $target"

    local version="$VERSION"
    if [ "$version" = "latest" ]; then
        version=$(get_latest_version)
    fi
    echo "Installing version: $version"

    install_binary "$version" "$target"

    echo ""
    if verify_installation; then
        echo "Installation complete! Run 'ai-rules --help' to get started."
    else
        echo "Installation complete!"
        echo "WARNING: Please add $INSTALL_DIR to your PATH and restart your shell."
    fi
}

main