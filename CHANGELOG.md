# Changelog

## 0.1.0 — Unreleased

- Experimental 2.4 GHz selected-key remapping on Base/Fn layers, including keyboard/modifier/media/browser destinations. Base restoration sends the original key; Fn Use normal key assigns the printed key explicitly. Whole-layer defaults and factory Fn restoration remain wired-only.
- Remap reply handling skips exact observed background messages under a fixed deadline, with no automatic retries or success inferred from USB write completion alone.

- Experimental 2.4 GHz device settings and both resets, with 18 captured action comparisons, delayed Reset Keyboard queries, transaction-scoped input clearing and bounded reply handling.

- Experimental 2.4 GHz receiver lighting: all twenty modes, RGB/colorful, brightness and speed, with capture-backed encoding, reply validation and a bounded timeout.
- Connection-aware transfer controls block unsupported receiver actions; offline editing and exports remain available. Screenshot mode skips host-device discovery.
- Renamed application, Cargo package/binary, desktop integration and release archives to Nix820 Pro (`nix820-pro`).
- Starry Grey charcoal/light-gray/dark-gray/yellow theme; RGB previews retain their selected colors.
- Automatic non-destructive migration from the previous Keyboard Studio XDG folders.
- Malformed saved data stops startup without replacing it with defaults; saves use private, unique staging files and atomic replacement.
- Standalone Rust project and independent visual presentation under the name Nix820 Pro.
- Base/Fn remapping, searchable keyboard/modifier/media/browser destinations and independent saved layers.
- Wired lighting, display image/animation upload, clock sync, settings and captured reset workflows.
- XDG storage and explicit prototype-data migration that preserves source and existing destination files.
- Capture-based regression tests, headless UI checks, strict Clippy, CI and native Linux release packaging.
- Manufacturer artwork, prototype research UI, inactive editors, raw captures and personal data excluded.
- Original code licensed under 0BSD; third-party notices retained separately.
