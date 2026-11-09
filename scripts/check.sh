#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 -m unittest discover -s tools/captures -v
python3 -m unittest discover -s tools -p test_package.py -v
shellcheck -x -P SCRIPTDIR scripts/*.sh
udevadm verify packaging/70-nix820-pro.rules
desktop-file-validate packaging/nix820-pro.desktop
