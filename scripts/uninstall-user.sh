#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=install-paths.sh
source "$script_dir/install-paths.sh"
resolve_install_paths "$@"
rm -f -- "$install_bin_dir/nix820-pro" \
    "$install_data_dir/applications/nix820-pro.desktop" \
    "$install_data_dir/icons/hicolor/scalable/apps/nix820-pro.svg" \
    "$install_app_dir/licenses/LICENSE" "$install_app_dir/licenses/THIRD_PARTY_NOTICES.md" \
    "$install_app_dir/install-paths.sh" "$install_app_dir/uninstall-user.sh"
rmdir -- "$install_app_dir/licenses" "$install_app_dir" 2>/dev/null || true
printf '%s\n' 'Uninstalled Nix820 Pro. Saved profiles, display presets and system USB rules were kept.'
