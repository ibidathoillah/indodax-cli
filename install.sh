#!/bin/sh
set -e

DEFAULT_BIN_DIR="/usr/local/bin"
BIN_DIR="${BIN_DIR:-${DEFAULT_BIN_DIR}}"
REPO="ibidathoillah/indodax-cli"

main() {
    need_cmd uname
    need_cmd mktemp
    need_cmd chmod
    need_cmd mkdir
    need_cmd rm
    need_cmd rmdir

    get_architecture || return 1
    local _arch="$RETVAL"
    assert_nz "$_arch" "arch"

    local _ext=""
    case "$_arch" in
        *windows*) _ext=".exe" ;;
    esac

    local _url="https://github.com/${REPO}/releases/latest/download/indodax-${_arch}${_ext}"

    local _dir
    if ! _dir="$(mktemp -d 2>/dev/null || mktemp -d -t indodax-install)"; then
        err "Failed to create temp directory"
    fi

    local _file="${_dir}/indodax${_ext}"

    echo "Downloading indodax-cli..."
    ensure curl -sSfL "$_url" -o "$_file" || ensure wget -qO "$_file" "$_url"

    ensure chmod +x "$_file"

    if [ "$_target" = "system" ]; then
        ensure mkdir -p "$BIN_DIR"
        ensure cp "$_file" "${BIN_DIR}/indodax${_ext}"
        echo "Installed to ${BIN_DIR}/indodax"
    else
        ensure mkdir -p "$HOME/.local/bin"
        ensure cp "$_file" "$HOME/.local/bin/indodax${_ext}"
        echo "Installed to $HOME/.local/bin/indodax"
        echo "Make sure $HOME/.local/bin is in your PATH"
    fi

    ignore rm -r "$_dir"
}

get_architecture() {
    local _ostype _cputype
    _ostype="$(uname -s)"
    _cputype="$(uname -m)"

    case "$_ostype" in
        Linux) _ostype=linux ;;
        Darwin) _ostype=macos ;;
        *) err "Unsupported OS: $_ostype" ;;
    esac

    case "$_cputype" in
        x86_64 | x86-64 | amd64) _cputype=x86_64 ;;
        aarch64 | arm64) _cputype=aarch64 ;;
        *) err "Unsupported CPU: $_cputype" ;;
    esac

    RETVAL="${_ostype}-${_cputype}"
}

need_cmd() {
    if ! check_cmd "$1"; then
        err "need '$1' (command not found)"
    fi
}

check_cmd() {
    command -v "$1" > /dev/null 2>&1
}

assert_nz() {
    if [ -z "$1" ]; then err "assert_nz $2"; fi
}

ensure() {
    if ! "$@"; then err "command failed: $*"; fi
}

ignore() {
    "$@"
}

err() {
    echo "Error: $1" >&2
    exit 1
}

_target="${INDODAX_INSTALL_TARGET:-local}"
main "$@" || exit 1
