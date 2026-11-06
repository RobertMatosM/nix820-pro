#!/usr/bin/env bash
# Render deterministic pages without loading/saving user data or sending USB commands.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
output_dir="${1:-$project_dir/screenshots}"
binary="${2:-${CARGO_TARGET_DIR:-target}/release/nix820-pro}"
mkdir -p -- "$output_dir"
for page in home keymap lighting display device; do
    "$binary" --page "$page" --screenshot "$output_dir/$page.png"
done
printf '%s\n' "Previews saved in $output_dir"
