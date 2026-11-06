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
const BG: Color32 = Color32::from_rgb(15, 23, 32);
const FG: Color32 = Color32::from_rgb(231, 238, 245);
const MUTED: Color32 = Color32::from_rgb(151, 168, 185);
const ACCENT: Color32 = Color32::from_rgb(69, 201, 183);
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
    }
    fn create_with_paths(
        ctx: &Context,
        screenshot: Option<PathBuf>,
        page: usize,
        paths: crate::paths::AppPaths,
    ) -> Self {
        let mut style = (*ctx.style_of(Theme::Dark)).clone();
        style.visuals = Visuals::dark();
        style.visuals.override_text_color = Some(FG);
        style.visuals.panel_fill = BG;
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
        let mut saved = screenshot
            .is_none()
            .then(|| std::fs::read(paths.config.join("profiles.json")).ok())
            .flatten()
            .and_then(|x| serde_json::from_slice::<Saved>(&x).ok())
            .filter(|s| {
                !s.profiles.is_empty()
                    && s.selected < s.profiles.len()
                    && s.profiles.iter().all(Profile::valid)
            })
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
            a.load_screens();
        }
        a.scan();
        if unsupported_sleep {
            a.notify("Previous sleep choice is unsupported; changed locally to 1min. Review before Apply settings.");
        }
        a
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
        if self.screenshot.is_some() {
            return;
        }
        match serde_json::to_vec_pretty(&self.saved)
            .map_err(|e| e.to_string())
            .and_then(|x| {
                std::fs::create_dir_all(&self.paths.config).map_err(|e| e.to_string())?;
                let temp = self.paths.config.join("profiles.tmp");
                std::fs::write(&temp, x).map_err(|e| e.to_string())?;
                std::fs::rename(temp, self.paths.config.join("profiles.json"))
                    .map_err(|e| e.to_string())
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
            "Keyboard Studio {}\nIndependent AK820 Pro configurator for Linux\n\n--page home|keymap|lighting|display|device\n--screenshot FILE  Render one page without loading or saving user data\n--migrate-from DIR  Copy legacy profiles/presets into XDG directories, then exit",
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
            .with_title("Keyboard Studio"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let paths =
        crate::paths::AppPaths::discover().map_err(|e| eframe::Error::AppCreation(e.into()))?;
    eframe::run_native(
        "Keyboard Studio",
        opts,
        Box::new(move |cc| {
            Ok(Box::new(Studio::create_with_paths(
                &cc.egui_ctx,
                screenshot,
                page,
                paths,
            )))
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
    let mut files = vec![];
    for (name, directory) in [
        ("profiles.json", &paths.config),
        ("screen-presets.json", &paths.data),
        ("screen-project.bin", &paths.data),
    ] {
        let path = source.join(name);
        if !path.exists() || directory.join(name).exists() {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
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
        files.push((directory.join(name), bytes));
    }
    let mut copied = 0;
    for (target, bytes) in files {
        std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        use std::io::Write;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut file) => {
                file.write_all(&bytes).map_err(|e| e.to_string())?;
                copied += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
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
        let mut app = Studio::create_with_paths(&ctx, None, 1, paths.clone());
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
}
