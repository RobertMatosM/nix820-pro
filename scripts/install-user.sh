#!/usr/bin/env bash
set -euo pipefail
package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -f "$package_dir/keyboard-studio" ]]; then
    printf '%s\n' 'Run install-user.sh from an extracted release archive.' >&2
    exit 1
fi
: "${HOME:?HOME must be set}"
install -Dm755 -- "$package_dir/keyboard-studio" "$HOME/.local/bin/keyboard-studio"
install -Dm644 -- "$package_dir/keyboard-studio.desktop" "${XDG_DATA_HOME:-$HOME/.local/share}/applications/keyboard-studio.desktop"
install -Dm644 -- "$package_dir/keyboard-studio.svg" "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps/keyboard-studio.svg"
for notice in LICENSE THIRD_PARTY_NOTICES.md; do
    install -Dm644 -- "$package_dir/$notice" "${XDG_DATA_HOME:-$HOME/.local/share}/keyboard-studio/licenses/$notice"
done
printf '%s\n' 'Installed Keyboard Studio. Ensure ~/.local/bin is on PATH.'
