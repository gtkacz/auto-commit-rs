#!/usr/bin/env bash
# cgen installer for Linux and macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/gtkacz/rust-auto-commit/main/scripts/install.sh | bash

set -euo pipefail

REPO="gtkacz/rust-auto-commit"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="cgen"

info() { printf "\033[1;34m%s\033[0m\n" "$1"; }
success() { printf "\033[1;32m%s\033[0m\n" "$1"; }
error() { printf "\033[1;31merror:\033[0m %s\n" "$1" >&2; exit 1; }

# Detect OS and architecture
detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)  arch="amd64" ;;
        arm64|aarch64) arch="arm64" ;;
        *)             error "Unsupported architecture: $arch" ;;
    esac

    # Only macOS has arm64 builds; Linux arm64 not yet supported
    if [ "$os" = "linux" ] && [ "$arch" = "arm64" ]; then
        error "Linux ARM64 builds are not yet available. Build from source: cargo install --git https://github.com/$REPO"
    fi

    echo "cgen-${os}-${arch}"
}

# Get latest release tag from GitHub API
get_latest_version() {
    local url="https://api.github.com/repos/$REPO/releases/latest"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Install one and try again."
    fi
}

main() {
    local artifact version download_url tmp_dir

    info "Detecting platform..."
    artifact="$(detect_platform)"
    info "Platform: $artifact"

    info "Fetching latest release..."
    version="$(get_latest_version)"

    if [ -z "$version" ]; then
        error "Could not determine latest version. Check https://github.com/$REPO/releases"
    fi

    info "Latest version: $version"

    download_url="https://github.com/$REPO/releases/download/${version}/${artifact}"

    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading $artifact..."
    if command -v curl &>/dev/null; then
        curl -fsSL -o "$tmp_dir/$BINARY_NAME" "$download_url"
    else
        wget -qO "$tmp_dir/$BINARY_NAME" "$download_url"
    fi

    chmod +x "$tmp_dir/$BINARY_NAME"

    info "Installing to $INSTALL_DIR/$BINARY_NAME..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$tmp_dir/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    else
        sudo mv "$tmp_dir/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    fi

    success "cgen $version installed successfully!"
    echo ""
    echo "  Run 'cgen config' to set up your API key."
    echo "  Run 'cgen --help' for usage information."
}

main
