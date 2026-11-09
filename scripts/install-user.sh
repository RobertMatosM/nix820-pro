#!/usr/bin/env bash
set -euo pipefail
package_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=install-paths.sh
source "$package_dir/install-paths.sh"
resolve_install_paths "$@"
launcher_env="$(type -P env)"
if [[ ! -f "$package_dir/nix820-pro" ]]; then
    printf '%s\n' 'Run install-user.sh from an extracted release archive.' >&2
    exit 1
fi
install -Dm755 -- "$package_dir/nix820-pro" "$install_bin_dir/nix820-pro"
install -Dm644 -- "$package_dir/nix820-pro.svg" "$install_data_dir/icons/hicolor/scalable/apps/nix820-pro.svg"
install -d -- "$install_data_dir/applications"
while IFS= read -r line; do
    if [[ "$line" == Exec=* ]]; then
        # env avoids GLib treating %% in the executable path as a literal name
        # before desktop field-code expansion. Both paths remain absolute.
        printf 'Exec=%s %s\n' "$(desktop_exec_path "$launcher_env")" "$(desktop_exec_path "$install_bin_dir/nix820-pro")"
    else
        printf '%s\n' "$line"
    fi
done < "$package_dir/nix820-pro.desktop" > "$install_data_dir/applications/nix820-pro.desktop"
chmod 644 -- "$install_data_dir/applications/nix820-pro.desktop"
for notice in LICENSE THIRD_PARTY_NOTICES.md; do
    install -Dm644 -- "$package_dir/$notice" "$install_app_dir/licenses/$notice"
done
for script in uninstall-user.sh install-paths.sh; do
    install -Dm644 -- "$package_dir/$script" "$install_app_dir/$script"
done
printf '%s\n' 'Installed Nix820 Pro and its application-menu entry.' \
    'USB access rules are a separate optional administrator step; see INSTALL.md.'
