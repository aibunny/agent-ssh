#!/usr/bin/env sh
# install.sh — one-line installer for agent-ssh
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aibunny/agent-ssh/main/scripts/install.sh | sh
#
# What it does:
#   1. Detects OS and CPU architecture.
#   2. Downloads the prebuilt binary from the latest GitHub release.
#   3. Installs it to /usr/local/bin/agent-ssh (or $INSTALL_DIR if set).
#
# Environment variables:
#   INSTALL_DIR   Override the installation directory (default: /usr/local/bin)
#   VERSION       Install a specific version tag, e.g. v0.1.0 (default: latest)
#   GITHUB_REPO   Override the repository (default: aibunny/agent-ssh)

set -eu

GITHUB_REPO="${GITHUB_REPO:-aibunny/agent-ssh}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="agent-ssh"

# ── Detect platform ────────────────────────────────────────────────────────────

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)
            echo "unsupported OS: $(uname -s)" >&2
            exit 1
            ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64 | amd64)  echo "x86_64" ;;
        aarch64 | arm64) echo "aarch64" ;;
        *)
            echo "unsupported architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

case "${OS}-${ARCH}" in
    linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
    macos-x86_64)   TARGET="x86_64-apple-darwin" ;;
    macos-aarch64)  TARGET="aarch64-apple-darwin" ;;
    *)
        echo "no prebuilt binary for ${OS}-${ARCH}" >&2
        exit 1
        ;;
esac

# ── Resolve version ────────────────────────────────────────────────────────────

if [ -z "${VERSION:-}" ]; then
    echo "resolving latest version..."
    VERSION="$(curl -fsSL \
        "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"([^"]+)".*/\1/')"
fi

echo "installing agent-ssh ${VERSION} for ${OS}/${ARCH}..."

# ── Download ───────────────────────────────────────────────────────────────────

ARCHIVE="${BINARY_NAME}-${TARGET}.tar.gz"
URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${ARCHIVE}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "downloading: ${URL}"
curl -fsSL --output "${TMP_DIR}/${ARCHIVE}" "${URL}"

# ── Verify checksum (if SHA256SUMS is present in the release) ─────────────────

verify_checksum() {
    if command -v sha256sum >/dev/null 2>&1; then
        grep " ${ARCHIVE}\$" "${TMP_DIR}/SHA256SUMS" | sha256sum -c -
    elif command -v shasum >/dev/null 2>&1; then
        grep " ${ARCHIVE}\$" "${TMP_DIR}/SHA256SUMS" | shasum -a 256 -c -
    else
        echo "warning: no sha256 verification tool found; skipping checksum verification"
        return 0
    fi
}

CHECKSUMS_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/SHA256SUMS"
if curl -fsSL --output "${TMP_DIR}/SHA256SUMS" "${CHECKSUMS_URL}" 2>/dev/null; then
    echo "verifying checksum..."
    verify_checksum
else
    echo "warning: could not download SHA256SUMS, skipping checksum verification"
fi

# ── Extract and install ────────────────────────────────────────────────────────

tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

DEST="${INSTALL_DIR}/${BINARY_NAME}"

require_sudo() {
    if command -v sudo >/dev/null 2>&1; then
        return 0
    fi

    echo "error: cannot write to ${INSTALL_DIR} and sudo is not available" >&2
    echo "set INSTALL_DIR to a writable directory such as \$HOME/.local/bin and rerun" >&2
    exit 1
}

if [ -d "${INSTALL_DIR}" ] && [ -w "${INSTALL_DIR}" ]; then
    cp "${TMP_DIR}/${BINARY_NAME}" "${DEST}"
    chmod 755 "${DEST}"
elif [ ! -d "${INSTALL_DIR}" ]; then
    if mkdir -p "${INSTALL_DIR}" 2>/dev/null; then
        cp "${TMP_DIR}/${BINARY_NAME}" "${DEST}"
        chmod 755 "${DEST}"
    else
        require_sudo
        echo "installing to ${DEST} (requires sudo)..."
        sudo mkdir -p "${INSTALL_DIR}"
        sudo cp "${TMP_DIR}/${BINARY_NAME}" "${DEST}"
        sudo chmod 755 "${DEST}"
    fi
else
    require_sudo
    echo "installing to ${DEST} (requires sudo)..."
    sudo mkdir -p "${INSTALL_DIR}"
    sudo cp "${TMP_DIR}/${BINARY_NAME}" "${DEST}"
    sudo chmod 755 "${DEST}"
fi

# ── Verify installation ────────────────────────────────────────────────────────

if "${DEST}" --version >/dev/null 2>&1; then
    echo ""
    echo "✓ installed: ${DEST}"
    echo "  version:   $("${DEST}" --version)"
    echo ""
    echo "quick start:"
    echo "  agent-ssh init               # create agent-ssh.toml"
    echo "  agent-ssh config validate    # validate config"
    echo "  agent-ssh --help             # full command reference"
else
    echo "installation failed: ${DEST} --version returned an error" >&2
    exit 1
fi
