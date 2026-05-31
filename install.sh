#!/usr/bin/env bash
set -e

REPO="RayenBHK/posh-tui"
BIN_NAME="posh-tui"
INSTALL_DIR="${HOME}/.local/bin"

# colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RESET='\033[0m'

info()    { echo -e "${CYAN}  →${RESET} $1"; }
success() { echo -e "${GREEN}  ✓${RESET} $1"; }
warn()    { echo -e "${YELLOW}  ⚠${RESET} $1"; }
error()   { echo -e "${RED}  ✗${RESET} $1"; exit 1; }

echo ""
echo -e "${CYAN}  posh-tui installer${RESET}"
echo "  ─────────────────────────────────"
echo ""

# detect OS and arch
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)       error "unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64)  ARCH_NAME="x86_64" ;;
    aarch64) ARCH_NAME="aarch64" ;;
    arm64)   ARCH_NAME="aarch64" ;;
    *)       error "unsupported architecture: $ARCH" ;;
esac

info "detected: $PLATFORM / $ARCH_NAME"

# get latest release version
info "fetching latest release..."
LATEST=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$LATEST" ]; then
    error "could not fetch latest release — check your internet connection"
fi

info "latest version: $LATEST"

# construct download URL
# binary name format: posh-tui-{version}-{arch}-{platform}
BINARY="${BIN_NAME}-${LATEST}-${ARCH_NAME}-${PLATFORM}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST}/${BINARY}"

# download
TMP_DIR="$(mktemp -d)"
TMP_BIN="${TMP_DIR}/${BIN_NAME}"

info "downloading ${BINARY}..."
if ! curl -sSfL "$DOWNLOAD_URL" -o "$TMP_BIN" 2>/dev/null; then
    # fallback — try plain binary name (single platform release)
    FALLBACK_URL="https://github.com/${REPO}/releases/download/${LATEST}/${BIN_NAME}"
    info "trying fallback download..."
    curl -sSfL "$FALLBACK_URL" -o "$TMP_BIN" \
        || error "download failed — visit https://github.com/${REPO}/releases"
fi

# make executable
chmod +x "$TMP_BIN"

# install
mkdir -p "$INSTALL_DIR"
mv "$TMP_BIN" "${INSTALL_DIR}/${BIN_NAME}"
rm -rf "$TMP_DIR"

success "installed to ${INSTALL_DIR}/${BIN_NAME}"

# check if install dir is in PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    warn "${INSTALL_DIR} is not in your PATH"
    echo ""
    echo "  add this to your ~/.bashrc or ~/.zshrc:"
    echo ""
    echo -e "    ${YELLOW}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
    echo ""
else
    echo ""
    success "run ${CYAN}posh-tui${RESET} to get started"
fi

echo ""
