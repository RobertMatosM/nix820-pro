# Release process

The first public prerelease is `v0.1.0-alpha.1`. Original project code is 0BSD; dependencies/fonts retain their notices. Source, screenshots and release packages contain no manufacturer artwork or private app data. Raw bus captures remain ignored.

## Checks and package

GitHub Actions builds on **Ubuntu 24.04 x86_64**. Every push/PR runs formatting, Rust tests, strict Clippy, Python capture-validator tests and ShellCheck. The job also builds the package and tests archive contents/checksum, a temporary user installation, desktop --help launch and uninstall with a saved-data sentinel. The resulting archive/checksum are retained as workflow artifacts.

Locally:

```sh
bash scripts/check.sh
bash scripts/package.sh
bash scripts/test-package.sh dist/nix820-pro-v0.1.0-alpha.1-linux-x86_64.tar.gz
```

Packaging refuses an existing destination; preserve or move a previous generated package before rebuilding the same version. Archives contain only the binary, end-user docs/screenshots, notices, original desktop entry/icon, user install/uninstall scripts, scoped udev rules and `BUILDINFO.txt`. The build-info file records public compiler/distribution/architecture details, not usernames or hostnames.

## Publish a tag

1. Update Cargo.toml/Cargo.lock, CHANGELOG.md, download examples and `docs/releases/TAG.md` together in a PR.
2. Let CI/package checks pass and merge into main.
3. Create an annotated `vVERSION` tag at the intended main commit and push that tag.

For tag pushes, the build job verifies the tag matches Cargo's version. A separate job verifies the tag is reachable from main, downloads the checked archive, checks its SHA256 again and creates the GitHub release with the version-specific notes. Versions containing a hyphen are published as prereleases and do not become latest stable. Existing releases are never silently replaced.

The publishing job alone has `contents: write`; ordinary build/PR jobs have read access. No release is published from a branch push or workflow_dispatch run. An untagged manual run can be used to inspect package artifacts first.

## Hardware evidence

The operator confirmed wireless lighting, device settings, both resets and Base/Fn remapping, then reported successful wired operation after the wireless changes. Exact combinations/features in those checks were not enumerated. Capture fixtures and headless/fake-transport tests complement those reports; other hardware/firmware revisions, power-cycle persistence and distribution compatibility need feedback.

TFT remains wired-only. Factory Fn restoration and whole-layer defaults remain unavailable over the receiver. These limits are intentional for this prerelease; see protocol/wireless notes for evidence details.
