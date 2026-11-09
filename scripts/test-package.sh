#!/usr/bin/env bash
# Install only inside a fresh temporary prefix; no USB access or HOME changes.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
archive="${1:?Pass the release .tar.gz archive}"
python3 tools/check-package.py "$archive"
test_root="$(mktemp -d -t nix820-package-test-XXXXXXXX)"
trap 'rm -rf -- "$test_root"' EXIT
tar -xzf "$archive" -C "$test_root"
package_dir="$test_root/$(basename -- "$archive" .tar.gz)"
# Spaces, percent signs and shell-special characters exercise desktop Exec quoting.
test_prefix="$test_root/user space % \$quote\"\`\\"
bash "$package_dir/install-user.sh" --prefix "$test_prefix"
"$test_prefix/bin/nix820-pro" --help > "$test_root/help.txt"
grep -q 'Nix820 Pro' "$test_root/help.txt"
desktop_file="$test_prefix/share/applications/nix820-pro.desktop"
desktop-file-validate "$desktop_file"
# Launch --help through the real GLib desktop parser, without opening the GUI.
sed '/^Exec=/s/$/ --help/' "$desktop_file" > "$test_root/help.desktop"
gio launch "$test_root/help.desktop" > "$test_root/desktop-help.txt"
grep -q 'Nix820 Pro' "$test_root/desktop-help.txt"
# Uninstall must leave user content in the app's data directory untouched.
printf '%s\n' 'private preset sentinel' > "$test_prefix/share/nix820-pro/screen-presets.json"
bash "$test_prefix/share/nix820-pro/uninstall-user.sh" --prefix "$test_prefix"
test ! -e "$test_prefix/bin/nix820-pro"
test ! -e "$desktop_file"
test ! -e "$test_prefix/share/icons/hicolor/scalable/apps/nix820-pro.svg"
test ! -e "$test_prefix/share/nix820-pro/uninstall-user.sh"
test ! -e "$test_prefix/share/nix820-pro/licenses/LICENSE"
grep -q 'private preset sentinel' "$test_prefix/share/nix820-pro/screen-presets.json"
printf '%s\n' 'Package install, desktop launch and uninstall passed; saved data preserved.'
