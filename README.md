# Nix820 Pro

Independent AK820 Pro configurator for Linux, with wired USB and experimental 2.4 GHz support. Written in Rust with egui/eframe. Not affiliated with or endorsed by AJAZZ; AJAZZ and AK820 Pro identify the supported keyboard.

Nix820 Pro has its own flat keyboard illustration, Starry Grey charcoal/gray/yellow interface, text navigation and searchable action picker. It works on Linux generally and does not require Nix or NixOS. It contains no manufacturer screenshots, logos or cropped product artwork.

## Supported features

- **Keymap:** Base and Fn layers, separate saved profiles and keyboard/modifier/media/browser destinations. Wired USB applies full layers; 2.4 GHz applies one selected key. Whole-layer defaults and factory Fn restoration require wired USB.
- **Lighting:** built-in effects, RGB/colorful mode, brightness and speed over wired USB or the 2.4 GHz receiver.
- **Display (wired USB only):** images, GIF animation and clock sync. Uploads use the keyboard's 128×128 format; the local editor is 160×128 and fits its content on upload.
- **Device:** five response levels, sleep time, shortcut locks, Reset Keyboard and Factory Reset over wired USB or the 2.4 GHz receiver.

Connect by wired USB for all features, or use the 2.4 GHz receiver for selected-key remapping, lighting, device settings and resets. Choose the connection in the Connection panel. Resets ask for confirmation. Whole-layer keymap transfers/defaults, display transfers and clock sync require wired USB. The app can save and edit profiles while disconnected. Changes reach the keyboard only through explicit Apply, Upload, Time Sync or a confirmed reset. Wired Apply layer replaces the selected layer's complete override table; existing keyboard overrides are not read or merged. Wireless Apply selected key updates only that source key. The editor shows saved local values, not live device readback.

For wireless remapping, open Keymap, choose Base or Fn, select a physical key and destination, then click **Apply selected key**. Edits remain local until Apply. On Base, **Restore key** prepares the original printed key. On Fn, **Use normal key** prepares an explicit printed-key assignment (for example, Fn+I → I); it does not restore a factory Fn shortcut. An unset Fn default cannot be sent wirelessly. The Default layer buttons are disabled for a receiver.

Fn reserves Esc, Backspace, Home, Win, Fn and Backslash/Pipe. The physical Fn source cannot be remapped on either layer. Bluetooth configuration, macros, custom per-key lighting, audio-driven effects and automatic startup are not implemented. Inactive editors from the prototype are excluded from the app, and their legacy saved data is preserved on import/migration.

The operator reports working lighting/display, Base/Fn/media remapping and the reset/settings UI. Regression tests validate recorded configuration traffic, acknowledgement handling and processing gaps. That does not establish every physical combination or power-cycle persistence; see [protocol notes](docs/protocol.md).

After the wireless changes, the operator performed a quick wired test and reported that wired operation still works. The exact features exercised in that quick test were not enumerated.

Receiver lighting reproduces 47 request/reply pairs from three captures, and the operator has tested it and reported it working. Receiver settings/reset encoding reproduces another 18 actions from five captures; the operator has also tested these implementations and confirmed they work. Reset Keyboard includes two captured follow-up queries and can take a few seconds. Reset failures report an unconfirmed outcome and are never retried automatically. The operator also tested wireless Base/Top and Fn remapping and reported both working; the exact tested key/destination combinations were not specified. Its source positions reuse the wired layout, independently matched at A and I. A receiver acknowledgement alone does not establish that the keyboard is awake, in range or displaying the intended result. TFT transfers remain wired-only, consistent with the operator's observation of the original driver's wireless limitations. See the [wireless investigation](docs/wireless-investigation.md) for evidence and limits.

## Build and run

Use a recent stable Rust toolchain and Python 3 for capture tooling. The supported build target is native Linux; X11 and Wayland are enabled through the Glow renderer. Development needs native X11/Wayland/XKB/OpenGL headers and `pkg-config` (the CI workflow lists Ubuntu packages).

```sh
cargo build --release --locked
./target/release/nix820-pro
```

On Linux, your user must have permission to access the keyboard's `/dev/hidraw*` nodes. The app does not change system permissions or require running the GUI as root. The Connection panel reports discovery and access failures. A native Zenity image chooser is optional; a built-in file browser is available when it is missing.

```sh
bash scripts/check.sh
bash scripts/package.sh
```

The package script generates an optimized, stripped native Linux archive with checksums, license notices, a desktop entry, an original SVG icon and a user-level installer under `dist/`. It does not upload or publish anything. The archive depends on the host's graphics/system libraries; it is not an AppImage or a cross-distribution compatibility guarantee. `install-user.sh` in the archive installs into `~/.local/bin` and the user applications/icon directories.

## Saved data and migration

Writable paths follow XDG base directories:

- Profiles/settings: `$XDG_CONFIG_HOME/nix820-pro/profiles.json`, default `~/.config/nix820-pro/profiles.json`.
- Display presets: `$XDG_DATA_HOME/nix820-pro/screen-presets.json`, default `~/.local/share/nix820-pro/screen-presets.json`.

Relative/empty XDG overrides are ignored. On normal launch, missing files are automatically copied from the previous `keyboard-studio` folders under the same XDG config/data roots. Originals and existing `nix820-pro` files are preserved. All pending files are validated before copying; a migration error stops launch with a message. Help and screenshot mode never trigger migration.

No writable path is derived from the build machine's source tree. The repository and release archive contain no personal profiles or display presets.

Unreadable or malformed saved data stops startup with an error and leaves the files untouched. Saves use private, unique staging files and atomic replacement, so concurrent instances cannot publish a partially written file. When two instances edit the same file, the last save wins.

To copy your prototype data, pass the old **app folder** explicitly:

```sh
./target/release/nix820-pro --migrate-from "/path/to/old/project/app"
```

This command validates available files and exits without opening the GUI or sending USB traffic. It preserves the source files and existing destination files. It supports `profiles.json`, `screen-presets.json` and the old `screen-project.bin`. Profile import/export remains available in Keymap. Migration preserves inactive legacy editor preferences without enabling those editors.

## Repository layout

- `src/`: application, device transport and pure protocol encoders.
- `tests/fixtures/`: scoped configuration reports, actual replies, hashes and operator annotations used by regression tests. No raw input captures.
- `tools/captures/`: offline validators for private captures; Python unit tests run without tshark. Extraction needs tshark and an explicit capture path. Generated output goes to ignored `capture-output/`.
- `assets/fonts/`: bundled fonts with their original licenses.
- `docs/`: protocol and release notes.
- `packaging/`, `scripts/`: desktop integration, checks, release packaging and optional project relocation.

Keep original USB captures private unless they have been reviewed for unrelated keyboard/input traffic. Fixture data is evidence for interoperability, not vendor assets or application code.

The entire `tests/wireshark/` folder is ignored, along with raw `.pcap`, `.pcapng`, `.cap` files and their compressed variants. Captures remain local; only reviewed configuration fixtures belong in Git.

## CLI and screenshots

`--help` describes the options. `--page home|keymap|lighting|display|device` opens a section. `--screenshot FILE` renders that single page using deterministic defaults, without loading/saving user data or sending USB commands; it requires a working desktop/display connection. Native captures of all five sections were reviewed outside the coding sandbox using screenshot mode. `bash scripts/preview.sh` regenerates the pages under ignored `screenshots/`; it accepts an output directory and optional binary path. Headless egui interaction tests also run in CI.

If this folder was prepared inside the original workspace because of filesystem restrictions, run `bash scripts/move-project.sh` to relocate the standalone repository to `~/Documents/Projects/rust/nix820-pro`. Existing destinations are never overwritten. The old prototype stays intact.

## License

Original Nix820 Pro code, documentation and drawings: **BSD Zero Clause (0BSD)**, as selected by the project owner. See [LICENSE](LICENSE). Bundled fonts and dependencies retain their own licenses; [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) contains their notices. Regenerate that file with `python3 tools/generate-notices.py` when changing dependencies or release targets.
