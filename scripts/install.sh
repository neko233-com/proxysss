#!/bin/bash
set -e

VERSION="${1:-latest}"
BINARY_NAME="proxysss"
REPO="neko233-com/proxysss"

detect_os() {
    case "$(uname -s)" in
        Linux*) echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *) echo "unsupported" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "amd64" ;;
        aarch64|arm64) echo "arm64" ;;
        *) echo "amd64" ;;
    esac
}

normalize_version() {
    local v="$1"
    v="${v#v}"
    v="${v#V}"
    echo "$v"
}

install_binary() {
    local os="$1"
    local arch="$2"
    local ver="$3"
    local asset="${BINARY_NAME}-${os}-${arch}"
    local url

    if [ "$ver" = "latest" ]; then
        url="https://github.com/${REPO}/releases/latest/download/${asset}"
    else
        url="https://github.com/${REPO}/releases/download/v${ver}/${asset}"
    fi

    local install_dir="/usr/local/bin"
    local target="${install_dir}/${BINARY_NAME}"
    local tmpdir
    tmpdir=$(mktemp -d)

    echo "Downloading ${url}..."
    curl -fsSL "$url" -o "${tmpdir}/${BINARY_NAME}"

    if [ -w "$install_dir" ]; then
        mv -f "${tmpdir}/${BINARY_NAME}" "$target"
    else
        sudo mv -f "${tmpdir}/${BINARY_NAME}" "$target"
    fi

    chmod +x "$target"
    rm -rf "$tmpdir"

    echo "Installed ${BINARY_NAME} to ${target}"
}

install_deno_if_missing() {
    if command -v deno >/dev/null 2>&1; then
        return
    fi

    echo "Deno not found, installing Deno for TypeScript script runtime..."
    curl -fsSL https://deno.land/install.sh | sh
    export PATH="$HOME/.deno/bin:$PATH"
}

main() {
    local os
    local arch

    os=$(detect_os)
    arch=$(detect_arch)

    if [ "$os" = "unsupported" ]; then
        echo "Unsupported operating system."
        echo "Windows users should run scripts/install.ps1 in PowerShell."
        exit 1
    fi

    if [ "$VERSION" != "latest" ] && [ -n "$VERSION" ]; then
        VERSION=$(normalize_version "$VERSION")
    else
        VERSION="latest"
    fi

    echo "Detected ${os}/${arch}"
    install_binary "$os" "$arch" "$VERSION"
    install_deno_if_missing

    ${BINARY_NAME} init
    ${BINARY_NAME} check-config
    ${BINARY_NAME} service install

    echo ""
    echo "Installed successfully."
    echo "Gateway port: 23380 (TCP for HTTP/1.1 + HTTP/2, UDP for HTTP/3)"
}

main "$@"
