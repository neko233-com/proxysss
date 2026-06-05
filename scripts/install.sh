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

proxysss_config_dir() {
    local os="$1"
    case "$os" in
        darwin) printf '%s\n' "$HOME/Library/Application Support/${BINARY_NAME}" ;;
        *) printf '%s\n' "${XDG_CONFIG_HOME:-$HOME/.config}/${BINARY_NAME}" ;;
    esac
}

managed_deno_bin() {
    local os="$1"
    printf '%s\n' "$(proxysss_config_dir "$os")/runtime/deno/bin/deno"
}

bundle_asset_name() {
    local os="$1"
    local arch="$2"
    printf '%s\n' "${BINARY_NAME}-${os}-${arch}.tar.gz"
}

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

install_bundle() {
    local os="$1"
    local arch="$2"
    local ver="$3"
    local asset
    asset="$(bundle_asset_name "$os" "$arch")"
    local url

    if [ "$ver" = "latest" ]; then
        url="https://github.com/${REPO}/releases/latest/download/${asset}"
    else
        url="https://github.com/${REPO}/releases/download/v${ver}/${asset}"
    fi

    local install_dir="/usr/local/bin"
    local target="${install_dir}/${BINARY_NAME}"
    local config_dir
    config_dir="$(proxysss_config_dir "$os")"
    local runtime_target="${config_dir}/runtime"
    local tmpdir
    tmpdir=$(mktemp -d)
    local archive_path="${tmpdir}/${asset}"
    local bundle_dir="${tmpdir}/bundle"

    echo "Downloading ${url}..."
    if [ "$DRY_RUN" = "true" ]; then
        echo "[dry-run] download ${asset}"
        echo "[dry-run] extract bundle to ${bundle_dir}"
        echo "[dry-run] install ${BINARY_NAME} to ${target}"
        echo "[dry-run] install bundled TypeScript runtime to ${runtime_target}"
        return
    fi

    mkdir -p "$bundle_dir"
    curl -fsSL "$url" -o "$archive_path"
    tar -xzf "$archive_path" -C "$bundle_dir"

    if [ ! -f "${bundle_dir}/${BINARY_NAME}" ]; then
        echo "bundle is missing ${BINARY_NAME}" >&2
        exit 1
    fi
    if [ ! -x "${bundle_dir}/runtime/deno/bin/deno" ]; then
        echo "bundle is missing bundled TypeScript runtime" >&2
        exit 1
    fi

    if [ -w "$install_dir" ]; then
        install -m 755 "${bundle_dir}/${BINARY_NAME}" "$target"
    else
        sudo install -m 755 "${bundle_dir}/${BINARY_NAME}" "$target"
    fi

    mkdir -p "$config_dir"
    rm -rf "$runtime_target"
    cp -R "${bundle_dir}/runtime" "$runtime_target"

    rm -rf "$tmpdir"

    echo "Installed ${BINARY_NAME} to ${target}"
    echo "Installed bundled TypeScript runtime to ${runtime_target}"
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
    install_bundle "$os" "$arch" "$VERSION"

    if [ "$SKIP_INIT" != "true" ]; then
        ${BINARY_NAME} init
        ${BINARY_NAME} check-config
    fi
    ${BINARY_NAME} service install || start_service_if_present

    echo ""
    echo "Installed successfully."
    echo "Gateway ports: 80 (HTTP), 443 (HTTPS + HTTP/3)"
}

main "$@"
