#!/usr/bin/env bash
# Run this yourself if the coding session cannot write to your Projects folder.
set -euo pipefail
: "${HOME:?HOME must be set}"
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
destination="${1:-$HOME/Documents/Projects/rust/keyboard-studio}"
if [[ "$project_dir" == "$destination" ]]; then
    printf '%s\n' "Project is already at $destination"
    exit 0
fi
if [[ -e "$destination" ]]; then
    printf '%s\n' "Destination already exists; refusing to overwrite: $destination" >&2
    exit 1
fi
mkdir -p -- "$(dirname -- "$destination")"
mv -- "$project_dir" "$destination"
printf '%s\n' "Project moved to $destination"
