#!/bin/bash
set -e

VERSION="latest"
ACTION="install"
ALLOW_DOWNGRADE="false"
NO_SERVICE_RESTART="false"
SKIP_INIT="false"
DRY_RUN="false"
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

installed_version() {
    if ! command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        return
    fi
    "${BINARY_NAME}" --version 2>/dev/null | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -n 1
}

compare_versions() {
    local left="$1"
    local right="$2"
    if [ -z "$left" ] || [ -z "$right" ]; then
        echo "unknown"
        return
    fi
    if [ "$left" = "$right" ]; then
        echo "0"
    elif [ "$(printf '%s\n%s\n' "$left" "$right" | sort -V | head -n 1)" = "$left" ]; then
        echo "-1"
    else
        echo "1"
    fi
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --action)
                ACTION="$2"
                shift 2
                ;;
            --version)
                VERSION="$2"
                shift 2
                ;;
            --allow-downgrade)
                ALLOW_DOWNGRADE="true"
                shift
                ;;
            --no-service-restart)
                NO_SERVICE_RESTART="true"
                shift
                ;;
            --skip-init)
                SKIP_INIT="true"
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            install|update|upgrade|downgrade)
                ACTION="$1"
                shift
                ;;
            *)
                VERSION="$1"
                shift
                ;;
        esac
    done
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
    if [ "$DRY_RUN" = "true" ]; then
        echo "[dry-run] install ${asset} to ${target}"
        return
    fi
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

stop_service_if_present() {
    if [ "$NO_SERVICE_RESTART" = "true" ]; then
        return
    fi
    if command -v systemctl >/dev/null 2>&1 && systemctl --user list-unit-files proxysss.service >/dev/null 2>&1; then
        systemctl --user stop proxysss.service >/dev/null 2>&1 || true
    fi
    if command -v launchctl >/dev/null 2>&1; then
        launchctl unload "$HOME/Library/LaunchAgents/com.neko233.proxysss.plist" >/dev/null 2>&1 || true
    fi
}

start_service_if_present() {
    if [ "$NO_SERVICE_RESTART" = "true" ]; then
        return
    fi
    if command -v systemctl >/dev/null 2>&1 && [ -f "$HOME/.config/systemd/user/proxysss.service" ]; then
        systemctl --user start proxysss.service >/dev/null 2>&1 || true
    fi
    if command -v launchctl >/dev/null 2>&1 && [ -f "$HOME/Library/LaunchAgents/com.neko233.proxysss.plist" ]; then
        launchctl load -w "$HOME/Library/LaunchAgents/com.neko233.proxysss.plist" >/dev/null 2>&1 || true
    fi
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
    parse_args "$@"
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

    local current
    current=$(installed_version || true)
    local cmp="unknown"
    if [ "$VERSION" != "latest" ] && [ -n "$current" ]; then
        cmp=$(compare_versions "$VERSION" "$current")
        if [ "$cmp" = "0" ]; then
            echo "Target version ${VERSION} already installed."
            exit 0
        fi
        if [ "$cmp" = "-1" ] && [ "$ALLOW_DOWNGRADE" != "true" ] && [ "$ACTION" != "downgrade" ]; then
            echo "Requested version ${VERSION} is lower than installed ${current}. Use --action downgrade or --allow-downgrade." >&2
            exit 1
        fi
        if [ "$cmp" = "1" ] && [ "$ACTION" = "downgrade" ]; then
            echo "Action downgrade requires lower target than current ${current}." >&2
            exit 1
        fi
    fi

    echo "Detected ${os}/${arch}; action=${ACTION}; current=${current:-none}; target=${VERSION}"
    stop_service_if_present
    install_binary "$os" "$arch" "$VERSION"
    install_deno_if_missing

    if [ "$SKIP_INIT" != "true" ]; then
        ${BINARY_NAME} init
        ${BINARY_NAME} check-config
    fi
    ${BINARY_NAME} service install || start_service_if_present

    echo ""
    echo "Installed successfully."
    echo "Gateway port: 23380 (TCP for HTTP/1.1 + HTTP/2, UDP for HTTP/3)"
}

main "$@"
