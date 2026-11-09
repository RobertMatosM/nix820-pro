#!/usr/bin/env bash
# shellcheck disable=SC2034 # These path outputs are consumed by the sourcing scripts.
# Shared by the archive's installer and uninstaller. Never changes HOME.
resolve_install_paths() {
    if (( $# == 0 )); then
        install_prefix="${HOME:?HOME must be set}/.local"
        if [[ "${XDG_DATA_HOME:-}" == /* ]]; then
            install_data_dir="$XDG_DATA_HOME"
        else
            install_data_dir="$install_prefix/share"
        fi
    elif (( $# == 2 )) && [[ "$1" == --prefix ]]; then
        install_prefix="${2%/}"
        install_data_dir="$install_prefix/share"
    elif (( $# == 1 )) && [[ "$1" == --help ]]; then
        printf '%s\n' 'Usage: bash install-user.sh [--prefix /absolute/directory]' \
            '       bash uninstall-user.sh [--prefix /absolute/directory]' \
            'Default: ~/.local/bin and XDG_DATA_HOME (or ~/.local/share).'
        exit 0
    else
        printf '%s\n' 'Expected no arguments, or --prefix /absolute/directory.' >&2
        return 1
    fi
    if [[ "$install_prefix" != /* || "$install_data_dir" != /* || "$install_prefix" == *'='* \
          || "$install_prefix$install_data_dir" == *$'\n'* || "$install_prefix$install_data_dir" == *$'\r'* ]]; then
        printf '%s\n' 'Use absolute paths without line breaks; the binary path cannot contain =.' >&2
        return 1
    fi
    install_bin_dir="$install_prefix/bin"
    install_app_dir="$install_data_dir/nix820-pro"
}

# Desktop Entry Exec applies argument quoting, then an outer string escape layer.
desktop_exec_path() {
    local escaped="$1"
    escaped="${escaped//\\/\\\\}"
    escaped="${escaped//\"/\\\"}"
    escaped="${escaped//\`/\\\`}"
    escaped="${escaped//\$/\\\$}"
    escaped="${escaped//%/%%}"
    escaped="${escaped//\\/\\\\}"
    printf '"%s"' "$escaped"
}
