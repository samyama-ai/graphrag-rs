#!/bin/sh
# graphrag-rs installer
# Usage: curl -sSL https://raw.githubusercontent.com/samyama-ai/graphrag-rs/main/install.sh | sh
set -e

REPO="samyama-ai/graphrag-rs"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Ensure install dir is in PATH
ensure_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo ""
            echo "Add this to your shell profile:"
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac
}

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
    darwin) TARGET="${ARCH}-apple-darwin" ;;
    linux)  TARGET="${ARCH}-unknown-linux-gnu" ;;
    *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Check for pre-built binary release
LATEST=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -n "$LATEST" ]; then
    BINARY_URL="https://github.com/${REPO}/releases/download/${LATEST}/graphrag-rs-${TARGET}.tar.gz"
    echo "Downloading graphrag-rs ${LATEST} for ${TARGET}..."

    if curl -sSfL "$BINARY_URL" -o /tmp/graphrag-rs.tar.gz 2>/dev/null; then
        mkdir -p "$INSTALL_DIR"
        tar -xzf /tmp/graphrag-rs.tar.gz -C /tmp/
        mv /tmp/graphrag-rs "$INSTALL_DIR/graphrag-rs"
        chmod +x "$INSTALL_DIR/graphrag-rs"
        rm -f /tmp/graphrag-rs.tar.gz
        echo "Installed graphrag-rs to $INSTALL_DIR/graphrag-rs"
        ensure_path
        exit 0
    fi
    echo "No pre-built binary for ${TARGET}, building from source..."
fi

# Fallback: build from source
echo "Building graphrag-rs from source..."

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR/graphrag-rs"
cd "$TMPDIR/graphrag-rs"
cargo build --release

mkdir -p "$INSTALL_DIR"
cp target/release/graphrag-rs "$INSTALL_DIR/graphrag-rs"

echo ""
echo "Installed graphrag-rs to $INSTALL_DIR/graphrag-rs"
ensure_path

echo ""
echo "Get started:"
echo "  export OPENAI_API_KEY=\"sk-...\""
echo "  graphrag-rs ingest ./my-docs/"
echo "  graphrag-rs serve"
