#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
cargo build --release --locked
python3 tools/generate-notices.py
version="$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')"
architecture="$(uname -m)"
package_dir="$project_dir/dist/nix820-pro-v${version}-linux-${architecture}"
if [[ -e "$package_dir" ]]; then
    printf '%s\n' "Package folder already exists: $package_dir" >&2
    exit 1
fi
mkdir -p -- "$package_dir"
cp -- "${CARGO_TARGET_DIR:-target}/release/nix820-pro" "$package_dir/"
cp -- README.md INSTALL.md LICENSE THIRD_PARTY_NOTICES.md CHANGELOG.md "$package_dir/"
cp -- packaging/nix820-pro.desktop packaging/nix820-pro.svg "$package_dir/"
cp -- scripts/install-user.sh scripts/uninstall-user.sh scripts/install-paths.sh "$package_dir/"
cp -- packaging/70-nix820-pro.rules "$package_dir/"
mkdir -p -- "$package_dir/docs/images"
cp -- docs/images/keymap.png docs/images/lighting.png "$package_dir/docs/images/"
{
    printf 'Nix820 Pro %s\nArchitecture: %s\n' "$version" "$architecture"
    if [[ -f /etc/os-release ]]; then
        # Only public distribution identifiers, never hostnames or user paths.
        sed -n -E '/^(ID|VERSION_ID|PRETTY_NAME)=/p' /etc/os-release
    fi
    getconf GNU_LIBC_VERSION
    rustc -vV
} > "$package_dir/BUILDINFO.txt"
release_epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || printf 0)}"
[[ "$release_epoch" =~ ^[0-9]+$ ]] || { printf '%s\n' 'Invalid SOURCE_DATE_EPOCH' >&2; exit 1; }
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$release_epoch" -C "$project_dir/dist" -czf "$package_dir.tar.gz" "$(basename -- "$package_dir")"
(cd -- "$project_dir/dist" && sha256sum "$(basename -- "$package_dir").tar.gz" > "$(basename -- "$package_dir").tar.gz.sha256")
printf '%s\n' "Created $package_dir.tar.gz"
