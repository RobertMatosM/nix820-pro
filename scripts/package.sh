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
cp -- README.md LICENSE THIRD_PARTY_NOTICES.md CHANGELOG.md "$package_dir/"
cp -- packaging/nix820-pro.desktop packaging/nix820-pro.svg "$package_dir/"
cp -- scripts/install-user.sh "$package_dir/"
tar -C "$project_dir/dist" -czf "$package_dir.tar.gz" "$(basename -- "$package_dir")"
(cd -- "$project_dir/dist" && sha256sum "$(basename -- "$package_dir").tar.gz" > "$(basename -- "$package_dir").tar.gz.sha256")
printf '%s\n' "Created $package_dir.tar.gz"
