# First release preparation

The source is a standalone Git repository under the name Nix820 Pro. Original code is 0BSD. No GitHub repository, remote, tag or release is created automatically.

Completed: research/inactive UI removed; vendor images removed; original flat gray/yellow drawing, matching SVG icon, text sidebar and action search; native window decoration/resizing; XDG storage, automatic Keyboard Studio name migration and explicit prototype migration; legacy unknown preferences retained; scoped test fixtures separated from runtime; strict lint checks; CI; optimized stripped packaging; bundled font/dependency notices.

All five sections were captured and visually reviewed through the host desktop using deterministic screenshot mode. Before tagging a binary release, test the renamed build against the keyboard and on intended Linux distributions, and confirm the support/firmware matrix and installation instructions.

Build outputs, distributions, raw captures, generated capture output and personal settings are ignored. Release archives contain the binary, documentation/notices, icon/desktop entry and user installer. Original private captures and the prototype remain in the old workspace. The development/test data in fixtures contains only intentional configuration traffic.

The operator confirmed wireless lighting, device settings, resets and Base/Fn remapping, then reported a successful quick wired test after these changes. Exact combinations/features in those checks were not enumerated. Current raw captures under `tests/wireshark/` remain local and the entire folder is ignored. The publication review found no personal profiles, presets, credentials or machine-specific user paths in Git candidates; public third-party attribution is retained.
