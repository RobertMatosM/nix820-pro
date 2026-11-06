#!/usr/bin/env bash
set -euo pipefail
package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -f "$package_dir/nix820-pro" ]]; then
    printf '%s\n' 'Run install-user.sh from an extracted release archive.' >&2
    exit 1
fi
: "${HOME:?HOME must be set}"
install -Dm755 -- "$package_dir/nix820-pro" "$HOME/.local/bin/nix820-pro"
install -Dm644 -- "$package_dir/nix820-pro.desktop" "${XDG_DATA_HOME:-$HOME/.local/share}/applications/nix820-pro.desktop"
install -Dm644 -- "$package_dir/nix820-pro.svg" "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps/nix820-pro.svg"
for notice in LICENSE THIRD_PARTY_NOTICES.md; do
    install -Dm644 -- "$package_dir/$notice" "${XDG_DATA_HOME:-$HOME/.local/share}/nix820-pro/licenses/$notice"
done
printf '%s\n' 'Installed Nix820 Pro. Ensure ~/.local/bin is on PATH.'
