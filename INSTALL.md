# Install Nix820 Pro

## Linux package

The first prerelease targets Linux x86_64. Official archives are built in GitHub Actions on Ubuntu 24.04, using glibc and system graphics libraries. A working X11 or Wayland desktop with OpenGL is required. Other architectures and older/incompatible system libraries require a source build. `BUILDINFO.txt` records the compiler, architecture, distribution and glibc version used for each archive.

Download the `.tar.gz` and matching `.tar.gz.sha256` from the [release page](https://github.com/RobertMatosM/nix820-pro/releases/tag/v0.1.0-alpha.1). In the download directory:

```sh
sha256sum -c nix820-pro-v0.1.0-alpha.1-linux-x86_64.tar.gz.sha256
tar -xzf nix820-pro-v0.1.0-alpha.1-linux-x86_64.tar.gz
cd nix820-pro-v0.1.0-alpha.1-linux-x86_64
bash install-user.sh
```

This installs the binary to `~/.local/bin`, a menu entry/icon and license notices to the user data directory. The menu entry uses the binary's absolute path, so it does not depend on your desktop's PATH. No administrator access is needed for this user installation.

Launch from the menu or with:

```sh
~/.local/bin/nix820-pro
```

You can also run `./nix820-pro` directly from the extracted archive. Zenity is optional for image selection; the app has a built-in file browser.

For an isolated/custom installation, use an absolute prefix:

```sh
bash install-user.sh --prefix /absolute/directory
```

This uses `PREFIX/bin` and `PREFIX/share`, ignoring XDG_DATA_HOME for the installation. Use the same prefix for uninstall. Without `--prefix`, an absolute XDG_DATA_HOME is honored; relative/empty overrides fall back to `~/.local/share`.

## USB access

Open **Connection** and refresh. Wired mode is identified as `0c45:8009`; the 2.4 GHz receiver is `0c45:fdfd`. Your user needs access to their matching `/dev/hidraw*` nodes. If access already works, no rule change is needed.

On a desktop using udev and systemd-logind, the supplied rule grants the active local session user access to these two device identities. From the extracted archive:

```sh
sudo install -m 644 70-nix820-pro.rules /etc/udev/rules.d/70-nix820-pro.rules
sudo udevadm control --reload-rules
```

Unplug/replug the cable or receiver, then refresh **Connection**. This is a separate administrator step; the user installer does not install system rules. The rule uses `uaccess` before systemd's seat ACL rule. See the [systemd seat rule](https://github.com/systemd/systemd/blob/main/rules.d/73-seat-late.rules.in) for that ordering.

`uaccess` depends on an active local desktop session. Headless/SSH sessions or systems without logind may need the distribution's device-group/ACL setup. If access still fails, include the exact Connection error in a bug report. Run the GUI as your normal user.

## First connection

1. Select wired or 2.4 GHz mode on the keyboard and attach the matching cable/receiver.
2. In **Connection**, select the device you intend to configure. A visible receiver does not prove that the keyboard is awake or in range.
3. Test a small lighting change, then Apply. Edits do not reach the keyboard until Apply/Upload or a confirmed reset.
4. On wired USB, keymap Apply sends the selected layer's complete local table. On wireless, choose a physical key and destination, then **Apply selected key**.

On wireless Fn, **Use normal key** prepares the printed key as an explicit assignment. It does not restore a built-in Fn shortcut. Use wired USB for factory Fn restoration or whole-layer defaults. TFT uploads and clock sync also require cable mode.

## Saved data and migration

- Profiles/settings: `$XDG_CONFIG_HOME/nix820-pro/profiles.json`, default `~/.config/nix820-pro/profiles.json`.
- Display presets: `$XDG_DATA_HOME/nix820-pro/screen-presets.json`, default `~/.local/share/nix820-pro/screen-presets.json`.

Relative/empty XDG overrides are ignored. Saved files are replaced atomically. Malformed/unreadable files stop startup and are left untouched. With multiple instances, the last save wins.

Missing destination files are automatically migrated from the previous `keyboard-studio` XDG folders. Originals and existing destination files are kept. To copy data from the old prototype's app folder explicitly:

```sh
~/.local/bin/nix820-pro --migrate-from /absolute/path/to/old/app
```

Migration validates files and exits without sending USB commands. Profile import/export is also available in Keymap. Back up your profiles/presets before changing or moving them.

## Uninstall

From the extracted archive, run:

```sh
bash uninstall-user.sh
```

An installed copy is also available at the default location:

```sh
bash ~/.local/share/nix820-pro/uninstall-user.sh
```

For a custom absolute XDG_DATA_HOME, use its `nix820-pro/uninstall-user.sh` path. For a custom prefix, run the archive's uninstaller with the same `--prefix /absolute/directory` argument.

The uninstaller removes only the binary, desktop entry, icon, installed helper scripts and license notices. Profiles and display presets are kept. It does not remove an optional system USB rule. To remove that rule separately:

```sh
sudo rm -- /etc/udev/rules.d/70-nix820-pro.rules
sudo udevadm control --reload-rules
```

Replug the device after removing the rule. This does not reset the keyboard's configuration.

## Build from source

Install a recent stable Rust toolchain, Python 3.11+ and native graphics headers. The Ubuntu package list is maintained in [.github/workflows/ci.yml](https://github.com/RobertMatosM/nix820-pro/blob/main/.github/workflows/ci.yml).

```sh
cargo build --release --locked
./target/release/nix820-pro
```

Development checks additionally need ShellCheck, desktop-file-utils and GLib's `gio`. `scripts/check.sh` runs formatting, Rust tests, strict Clippy, Python capture tests and script/udev checks. `scripts/package.sh` builds a local archive; pass it to `scripts/test-package.sh` to verify its contents, installation, desktop launch and uninstall inside a temporary prefix.

`--help` lists CLI options. `--page home|keymap|lighting|display|device` selects a section. `--screenshot FILE` renders deterministic defaults without loading/saving user data, scanning devices or sending USB commands; it requires a desktop/display connection. `scripts/preview.sh` generates all five pages under ignored `screenshots/`.
