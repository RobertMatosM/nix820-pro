mod canvas;
mod files;
mod hardware;
mod home;
mod keyboard;
mod lighting;
mod model;
mod persistence;
mod profiles;
mod settings;
mod tft;
mod window;
use crate::gif_io;
use canvas::Canvas;
use eframe::egui::{self, *};
use model::{Profile, Saved};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
// Starry Grey: charcoal case, warm gray keycaps, restrained yellow accents.
const BG: Color32 = Color32::from_rgb(27, 28, 30);
const SURFACE: Color32 = Color32::from_rgb(41, 43, 46);
const SIDEBAR: Color32 = Color32::from_rgb(32, 34, 36);
const BORDER: Color32 = Color32::from_rgb(71, 74, 79);
const FG: Color32 = Color32::from_rgb(236, 237, 234);
const MUTED: Color32 = Color32::from_rgb(174, 177, 181);
const ACCENT: Color32 = Color32::from_rgb(242, 207, 53);
const SELECTION: Color32 = Color32::from_rgb(69, 63, 35);
const KEY_LIGHT: Color32 = Color32::from_rgb(201, 203, 200);
const KEY_DARK: Color32 = Color32::from_rgb(102, 106, 112);
const INK: Color32 = Color32::from_rgb(27, 28, 30);
const MODES: [&str; 20] = [
    "Static",
    "SingleOn",
    "SingleOff",
    "Glittering",
    "Falling",
    "Colourful",
    "Breath",
    "Spectrum",
    "Outward",
    "Scrolling",
    "Rolling",
    "Rotating",
    "Explode",
    "Launch",
    "Ripples",
    "Flowing",
    "Pulsating",
    "Tilt",
    "Shuttle",
    "LED Off",
];
enum ModeChange {
    Page(usize),
    Lighting(usize),
}
struct Studio {
    saved: Saved,
    paths: crate::paths::AppPaths,
    persistence_ready: bool,
    destination_query: String,
    destination_category: usize,
    about_open: bool,
    page: usize,
    layer: bool,
    key: Option<String>,
    toast: Option<(String, Instant)>,
    screenshot: Option<PathBuf>,
    frames: u32,
    grid: bool,
    preview: bool,
    pixels: Vec<Vec<u8>>,
    frame_index: usize,
    thumb_offset: f32,
    presets: Vec<model::ScreenPreset>,
    preset_index: usize,
    delays: Vec<u32>,
    preset_scroll: f32,
    transition_page: usize,
    pending_reset: Option<crate::protocol::ResetKind>,
    transition_started: Option<Instant>,
    pending_mode: Option<ModeChange>,
    preset_textures: BTreeMap<usize, TextureHandle>,
    thumb_textures: BTreeMap<usize, TextureHandle>,
    delay: u32,
    draw_color: [u8; 3],
    texture: Option<TextureHandle>,
    dirty_texture: bool,
    file_dialog: Option<String>,
    file_path: String,
    image_picker: Option<std::sync::mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    image_browser: Option<PathBuf>,
    image_selection: Option<PathBuf>,
    rename: bool,
    rename_value: String,
    screen_text: String,
    devices: Vec<String>,
    eraser: bool,
    undo: Vec<Vec<u8>>,
    started: Instant,
    keyboards: Vec<crate::device::Keyboard>,
    keyboard_index: usize,
    hardware_open: bool,
    transfer: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    transfer_status: String,
}
impl Studio {
    #[cfg(test)]
    fn create(ctx: &Context, screenshot: Option<PathBuf>, page: usize) -> Self {
        Self::create_with_paths(
            ctx,
            screenshot,
            page,
            crate::paths::AppPaths::discover().expect("user directories"),
        )
        .expect("test app data")
    }
    fn create_with_paths(
        ctx: &Context,
        screenshot: Option<PathBuf>,
        page: usize,
        paths: crate::paths::AppPaths,
    ) -> Result<Self, String> {
        let mut style = (*ctx.style_of(Theme::Dark)).clone();
        style.visuals = Visuals::dark();
        style.visuals.override_text_color = Some(FG);
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = SURFACE;
        style.visuals.extreme_bg_color = BG;
        style.visuals.faint_bg_color = SURFACE;
        style.visuals.selection.bg_fill = SELECTION;
        style.visuals.selection.stroke = Stroke::new(1., ACCENT);
        style.visuals.hyperlink_color = ACCENT;
        style.visuals.widgets.inactive.bg_fill = SURFACE;
        style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
        style.visuals.widgets.hovered.bg_fill = SELECTION;
        style.visuals.widgets.hovered.weak_bg_fill = SELECTION;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., ACCENT);
        style.visuals.widgets.active.bg_fill = SELECTION;
        style.visuals.widgets.active.weak_bg_fill = SELECTION;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1., ACCENT);
        ctx.set_style_of(Theme::Dark, style);

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "interface".into(),
            std::sync::Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/Interface.ttf"
            ))),
        );
        fonts.font_data.insert(
            "symbols".into(),
            std::sync::Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/Symbols.ttf"
            ))),
        );
        let family = fonts.families.entry(FontFamily::Proportional).or_default();
        family.insert(0, "symbols".into());
        family.insert(0, "interface".into());
        fonts.families.insert(
            FontFamily::Monospace,
            vec!["interface".into(), "symbols".into()],
        );
        ctx.set_fonts(fonts);
        let saved_path = paths.config.join("profiles.json");
        let mut saved = if screenshot.is_some() {
            None
        } else {
            crate::paths::read_optional(&saved_path)?
                .map(|bytes| {
                    let saved: Saved = serde_json::from_slice(&bytes).map_err(|error| {
                        format!(
                            "Invalid profiles in {}: {error}. The file was left untouched.",
                            saved_path.display()
                        )
                    })?;
                    if saved.profiles.is_empty()
                        || saved.selected >= saved.profiles.len()
                        || !saved.profiles.iter().all(Profile::valid)
                    {
                        return Err(format!(
                            "Invalid profiles in {}. The file was left untouched.",
                            saved_path.display()
                        ));
                    }
                    Ok(saved)
                })
                .transpose()?
        }
        .unwrap_or(Saved {
            profiles: (1..=3).map(Profile::new).collect(),
            selected: 0,
            extras: model::Extras::default(),
        });
        let unsupported_sleep = saved.extras.migrate_settings();
        for profile in &mut saved.profiles {
            profile.migrate_key_maps();
        }
        let mut a = Self {
            saved,
            paths,
            persistence_ready: false,
            destination_query: String::new(),
            destination_category: 0,
            about_open: false,
            page,
            layer: false,
            key: None,
            toast: None,
            screenshot,
            frames: 0,
            grid: false,
            preview: false,
            pixels: vec![vec![0; 160 * 128 * 4]; 4],
            frame_index: 0,
            thumb_offset: 0.,
            presets: vec![model::ScreenPreset::blank(4)],
            preset_index: 0,
            delays: vec![50; 4],
            preset_scroll: 0.,
            transition_page: page,
            pending_reset: None,
            transition_started: None,
            pending_mode: None,
            preset_textures: BTreeMap::new(),
            delay: 50,
            draw_color: [255, 255, 255],
            texture: None,
            thumb_textures: BTreeMap::new(),
            dirty_texture: true,
            file_dialog: None,
            file_path: String::new(),
            image_picker: None,
            image_browser: None,
            image_selection: None,
            rename: false,
            rename_value: String::new(),
            screen_text: String::new(),
            devices: vec![],
            eraser: false,
            undo: vec![],
            started: Instant::now(),
            keyboards: Vec::new(),
            keyboard_index: 0,
            hardware_open: false,
            transfer: None,
            transfer_status:
                "No transfer yet. Local changes are applied only with Apply or Upload.".into(),
        };
        if a.screenshot.is_none() {
            a.load_screens()?;
        }
        a.persistence_ready = true;
        a.scan();
        if unsupported_sleep {
            a.notify("Previous sleep choice is unsupported; changed locally to 1min. Review before Apply settings.");
        }
        Ok(a)
    }
    fn p(&self) -> &Profile {
        &self.saved.profiles[self.saved.selected]
    }
    fn pm(&mut self) -> &mut Profile {
        &mut self.saved.profiles[self.saved.selected]
    }
    fn notify(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now()));
    }
    fn save(&mut self) {
        if self.screenshot.is_some() || !self.persistence_ready {
            return;
        }
        match serde_json::to_vec_pretty(&self.saved)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                crate::paths::write_atomic(&self.paths.config.join("profiles.json"), &bytes)
            }) {
            Ok(_) => {}
            Err(e) => self.notify(format!("Could not save: {e}")),
        }
    }
    fn scan(&mut self) {
        self.keyboards = crate::device::discover();
        self.keyboard_index = self
            .keyboard_index
            .min(self.keyboards.len().saturating_sub(1));
        self.devices.clear();
        if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
            for e in entries.flatten() {
                let p = e.path();
                let vid = std::fs::read_to_string(p.join("idVendor")).unwrap_or_default();
                let pid = std::fs::read_to_string(p.join("idProduct")).unwrap_or_default();
                if vid.trim() == "0c45" && ["8009", "fdfd"].contains(&pid.trim()) {
                    self.devices.push(format!(
                        "0c45:{}  {}",
                        pid.trim(),
                        if pid.trim() == "8009" {
                            "Wired keyboard"
                        } else {
                            "2.4G receiver"
                        }
                    ));
                }
            }
        }
    }
    fn color(&self) -> Color32 {
        let a = self.p().rgb;
        Color32::from_rgb(a[0], a[1], a[2])
    }
}
pub(crate) fn run() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Nix820 Pro {}\nIndependent AK820 Pro configurator for Linux\n\n--page home|keymap|lighting|display|device\n--screenshot FILE  Render one page without loading or saving user data\n--migrate-from DIR  Copy legacy profiles/presets into XDG directories, then exit",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--migrate-from") {
        let source = args.get(index + 1).ok_or_else(|| {
            eframe::Error::AppCreation("--migrate-from requires a directory".into())
        })?;
        let paths =
            crate::paths::AppPaths::discover().map_err(|e| eframe::Error::AppCreation(e.into()))?;
        let count = migrate_legacy(std::path::Path::new(source), &paths)
            .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        println!("Migrated {count} files. Existing destination files were preserved.");
        return Ok(());
    }
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let page = args
        .iter()
        .position(|a| a == "--page")
        .and_then(|i| args.get(i + 1))
        .map(|page| match page.as_str() {
            "keymap" | "keys" => 1,
            "lighting" => 3,
            "display" | "tft" => 4,
            "device" | "settings" => 8,
            _ => 0,
        })
        .unwrap_or(0);
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1480., 800.])
            .with_min_inner_size([1110., 600.])
            .with_title("Nix820 Pro"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let paths =
        crate::paths::AppPaths::discover().map_err(|e| eframe::Error::AppCreation(e.into()))?;
    let migrated = if screenshot.is_none() {
        migrate_previous_name(&paths).map_err(|e| {
            eframe::Error::AppCreation(
                format!("Could not migrate Keyboard Studio data: {e}").into(),
            )
        })?
    } else {
        0
    };
    eframe::run_native(
        "Nix820 Pro",
        opts,
        Box::new(move |cc| {
            let mut studio = Studio::create_with_paths(&cc.egui_ctx, screenshot, page, paths)
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
            if migrated > 0 {
                studio.notify(format!(
                    "Migrated {migrated} saved files from Keyboard Studio. Originals were kept."
                ));
            }
            Ok(Box::new(studio))
        }),
    )
}

fn migrate_legacy(
    source: &std::path::Path,
    paths: &crate::paths::AppPaths,
) -> Result<usize, String> {
    if !source.is_dir() {
        return Err("Legacy directory does not exist.".into());
    }
    migrate_files([
        (
            source.join("profiles.json"),
            paths.config.join("profiles.json"),
        ),
        (
            source.join("screen-presets.json"),
            paths.data.join("screen-presets.json"),
        ),
        (
            source.join("screen-project.bin"),
            paths.data.join("screen-project.bin"),
        ),
    ])
}
fn migrate_previous_name(paths: &crate::paths::AppPaths) -> Result<usize, String> {
    let previous = paths.previous_name();
    migrate_files([
        (
            previous.config.join("profiles.json"),
            paths.config.join("profiles.json"),
        ),
        (
            previous.data.join("screen-presets.json"),
            paths.data.join("screen-presets.json"),
        ),
        (
            previous.data.join("screen-project.bin"),
            paths.data.join("screen-project.bin"),
        ),
    ])
}
fn migrate_files(sources: impl IntoIterator<Item = (PathBuf, PathBuf)>) -> Result<usize, String> {
    let mut files = vec![];
    for (path, target) in sources {
        if target.exists() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid migration filename")?;
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        match name {
            "profiles.json" => {
                let saved: Saved = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if saved.profiles.is_empty()
                    || saved.selected >= saved.profiles.len()
                    || !saved.profiles.iter().all(Profile::valid)
                {
                    return Err("Invalid legacy profiles; no files copied.".into());
                }
            }
            "screen-presets.json" => {
                let (selected, presets): (usize, Vec<model::ScreenPreset>) =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if presets.is_empty()
                    || presets.len() > 32
                    || selected >= presets.len()
                    || !presets.iter().all(model::ScreenPreset::valid)
                {
                    return Err("Invalid legacy display presets; no files copied.".into());
                }
            }
            _ => {
                if bytes.len() < 8 || &bytes[..4] != b"KSC1" {
                    return Err("Invalid legacy display project; no files copied.".into());
                }
                let count = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
                if !(1..=140).contains(&count) || bytes.len() != 8 + count * 160 * 128 * 4 {
                    return Err("Invalid legacy display frames; no files copied.".into());
                }
            }
        }
        files.push((target, bytes));
    }
    let mut copied = 0;
    for (target, bytes) in files {
        std::fs::create_dir_all(target.parent().unwrap()).map_err(|error| error.to_string())?;
        let temporary = target.with_file_name(format!(
            ".{}.migrate-{}.tmp",
            target.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let result = (|| -> Result<bool, String> {
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| error.to_string())?;
            // Publish a complete file atomically; a concurrent destination is preserved.
            match std::fs::hard_link(&temporary, &target) {
                Ok(()) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
                Err(error) => Err(error.to_string()),
            }
        })();
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        copied += usize::from(result?);
    }
    Ok(copied)
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn folder() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ks-migration-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    fn paths(root: &std::path::Path) -> crate::paths::AppPaths {
        crate::paths::AppPaths {
            config: root.join("config"),
            data: root.join("data"),
        }
    }
    fn profiles() -> Vec<u8> {
        let mut profile = Profile::new(1);
        profile
            .assign_layer(
                crate::remap::Layer::Fn,
                "I",
                Some(crate::remap::KeyCode::Insert),
            )
            .unwrap();
        let saved = Saved {
            profiles: vec![profile],
            selected: 0,
            extras: model::Extras::default(),
        };
        let mut value = serde_json::to_value(saved).unwrap();
        value["extras"]["macros"] = serde_json::json!([{"name":"Old macro","actions":[]}]);
        serde_json::to_vec(&value).unwrap()
    }
    #[test]
    fn migration_preserves_originals_existing_destinations_profiles_presets_and_inactive_legacy_data()
     {
        let root = folder();
        let source = root.join("legacy");
        std::fs::create_dir(&source).unwrap();
        let original = profiles();
        std::fs::write(source.join("profiles.json"), &original).unwrap();
        let presets = serde_json::to_vec(&(0, vec![model::ScreenPreset::blank(1)])).unwrap();
        std::fs::write(source.join("screen-presets.json"), &presets).unwrap();
        let paths = paths(&root);
        assert_eq!(migrate_legacy(&source, &paths).unwrap(), 2);
        assert_eq!(
            std::fs::read(source.join("profiles.json")).unwrap(),
            original
        );
        let ctx = Context::default();
        let mut app = Studio::create_with_paths(&ctx, None, 1, paths.clone()).unwrap();
        assert_eq!(
            app.p().fn_keys.get("I"),
            Some(&Some(crate::remap::KeyCode::Insert))
        );
        app.save();
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(paths.config.join("profiles.json")).unwrap())
                .unwrap();
        assert_eq!(saved["extras"]["macros"][0]["name"], "Old macro");
        assert_eq!(app.presets.len(), 1);
        assert_eq!(app.pixels.len(), 1);
        app.screenshot = Some(PathBuf::from("unused.png"));
        drop(app);
        let saved = std::fs::read(paths.config.join("profiles.json")).unwrap();
        assert_eq!(migrate_legacy(&source, &paths).unwrap(), 0);
        assert_eq!(
            std::fs::read(paths.config.join("profiles.json")).unwrap(),
            saved
        );
        assert_eq!(
            std::fs::read(paths.data.join("screen-presets.json")).unwrap(),
            presets
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn invalid_legacy_data_is_rejected_before_any_destination_is_written() {
        let root = folder();
        let source = root.join("legacy");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("profiles.json"), profiles()).unwrap();
        std::fs::write(source.join("screen-presets.json"), b"invalid").unwrap();
        let paths = paths(&root);
        assert!(migrate_legacy(&source, &paths).is_err());
        assert!(!paths.config.exists());
        assert!(!paths.data.exists());
        assert!(migrate_legacy(&root.join("missing"), &paths).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    fn renamed_paths(root: &std::path::Path) -> crate::paths::AppPaths {
        crate::paths::AppPaths {
            config: root.join("config/nix820-pro"),
            data: root.join("data/nix820-pro"),
        }
    }
    fn previous_files(root: &std::path::Path) -> (crate::paths::AppPaths, Vec<u8>, Vec<u8>) {
        let paths = renamed_paths(root);
        let old = paths.previous_name();
        std::fs::create_dir_all(&old.config).unwrap();
        std::fs::create_dir_all(&old.data).unwrap();
        let profiles = profiles();
        let presets = serde_json::to_vec(&(0, vec![model::ScreenPreset::blank(1)])).unwrap();
        std::fs::write(old.config.join("profiles.json"), &profiles).unwrap();
        std::fs::write(old.data.join("screen-presets.json"), &presets).unwrap();
        (paths, profiles, presets)
    }
    #[test]
    fn previous_name_migration_copies_separate_xdg_folders_and_is_idempotent() {
        let root = folder();
        let (paths, profiles, presets) = previous_files(&root);
        assert_eq!(migrate_previous_name(&paths).unwrap(), 2);
        assert_eq!(
            std::fs::read(paths.config.join("profiles.json")).unwrap(),
            profiles
        );
        assert_eq!(
            std::fs::read(paths.data.join("screen-presets.json")).unwrap(),
            presets
        );
        assert_eq!(migrate_previous_name(&paths).unwrap(), 0);
        let old = paths.previous_name();
        assert_eq!(
            std::fs::read(old.config.join("profiles.json")).unwrap(),
            profiles
        );
        assert_eq!(
            std::fs::read(old.data.join("screen-presets.json")).unwrap(),
            presets
        );
        assert_eq!(std::fs::read_dir(&paths.config).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(&paths.data).unwrap().count(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn previous_name_migration_fills_only_missing_files_and_preserves_newer_profiles() {
        let root = folder();
        let (paths, _, presets) = previous_files(&root);
        std::fs::create_dir_all(&paths.config).unwrap();
        let mut newer: serde_json::Value = serde_json::from_slice(&profiles()).unwrap();
        newer["profiles"][0]["name"] = "Existing Nix820 Pro profile".into();
        let newer = serde_json::to_vec(&newer).unwrap();
        std::fs::write(paths.config.join("profiles.json"), &newer).unwrap();
        assert_eq!(migrate_previous_name(&paths).unwrap(), 1);
        assert_eq!(
            std::fs::read(paths.config.join("profiles.json")).unwrap(),
            newer
        );
        assert_eq!(
            std::fs::read(paths.data.join("screen-presets.json")).unwrap(),
            presets
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn previous_name_migration_validates_both_folders_before_copying_any_file() {
        let root = folder();
        let (paths, _, _) = previous_files(&root);
        std::fs::write(
            paths.previous_name().data.join("screen-presets.json"),
            b"invalid",
        )
        .unwrap();
        assert!(migrate_previous_name(&paths).is_err());
        assert!(!paths.config.exists());
        assert!(!paths.data.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_profiles_abort_startup_without_changing_any_saved_file() {
        let root = folder();
        let paths = paths(&root);
        std::fs::create_dir_all(&paths.config).unwrap();
        std::fs::create_dir_all(&paths.data).unwrap();
        let presets = serde_json::to_vec(&(0, vec![model::ScreenPreset::blank(1)])).unwrap();
        std::fs::write(paths.data.join("screen-presets.json"), &presets).unwrap();
        let mut invalid: serde_json::Value = serde_json::from_slice(&profiles()).unwrap();
        invalid["selected"] = 999.into();
        for bytes in [
            b"broken JSON".to_vec(),
            serde_json::to_vec(&invalid).unwrap(),
        ] {
            std::fs::write(paths.config.join("profiles.json"), &bytes).unwrap();
            let result = Studio::create_with_paths(&Context::default(), None, 1, paths.clone());
            assert!(result.is_err());
            drop(result);
            assert_eq!(
                std::fs::read(paths.config.join("profiles.json")).unwrap(),
                bytes
            );
            assert_eq!(
                std::fs::read(paths.data.join("screen-presets.json")).unwrap(),
                presets
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_presets_abort_startup_without_falling_back_or_saving_defaults() {
        let root = folder();
        let paths = paths(&root);
        std::fs::create_dir_all(&paths.config).unwrap();
        std::fs::create_dir_all(&paths.data).unwrap();
        let profiles = profiles();
        std::fs::write(paths.config.join("profiles.json"), &profiles).unwrap();
        // A valid older-format file must not mask a damaged current preset file.
        let mut legacy = b"KSC1".to_vec();
        legacy.extend_from_slice(&[1, 0, 50, 0]);
        legacy.resize(8 + 160 * 128 * 4, 0);
        std::fs::write(paths.data.join("screen-project.bin"), &legacy).unwrap();
        let invalid = serde_json::to_vec(&(9, vec![model::ScreenPreset::blank(1)])).unwrap();
        for bytes in [b"broken JSON".to_vec(), invalid] {
            std::fs::write(paths.data.join("screen-presets.json"), &bytes).unwrap();
            let result = Studio::create_with_paths(&Context::default(), None, 4, paths.clone());
            assert!(result.is_err());
            drop(result);
            assert_eq!(
                std::fs::read(paths.data.join("screen-presets.json")).unwrap(),
                bytes
            );
            assert_eq!(
                std::fs::read(paths.config.join("profiles.json")).unwrap(),
                profiles
            );
            assert_eq!(
                std::fs::read(paths.data.join("screen-project.bin")).unwrap(),
                legacy
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_legacy_display_data_cannot_be_replaced_by_defaults_on_startup_error() {
        let root = folder();
        let paths = paths(&root);
        std::fs::create_dir_all(&paths.data).unwrap();
        std::fs::write(paths.data.join("screen-project.bin"), b"broken").unwrap();
        let result = Studio::create_with_paths(&Context::default(), None, 4, paths.clone());
        assert!(result.is_err());
        drop(result);
        assert!(!paths.data.join("screen-presets.json").exists());
        assert_eq!(
            std::fs::read(paths.data.join("screen-project.bin")).unwrap(),
            b"broken"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
