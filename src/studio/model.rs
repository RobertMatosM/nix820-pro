use super::*;
use crate::remap::{KeyCode, KeyMap, Layer, source_slot};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Profile {
    pub(super) name: String,
    pub(super) mode: usize,
    pub(super) brightness: f32,
    pub(super) speed: f32,
    pub(super) rgb: [u8; 3],
    pub(super) colorful: bool,
    #[serde(default)]
    pub(super) captured_map: Option<Vec<u8>>,
    pub(super) maps: BTreeMap<String, String>,
    /// Top-layer destinations use stable key identifiers. None explicitly restores a default.
    #[serde(default)]
    pub(super) top_keys: BTreeMap<String, Option<KeyCode>>,
    #[serde(default)]
    pub(super) fn_keys: BTreeMap<String, Option<KeyCode>>,
}
impl Profile {
    pub(super) fn valid(&self) -> bool {
        self.mode < 20
            && self.brightness.is_finite()
            && self.speed.is_finite()
            && (1.0..=5.0).contains(&self.brightness)
            && (1.0..=5.0).contains(&self.speed)
            && self
                .captured_map
                .as_ref()
                .is_none_or(|m| crate::protocol::map_reports(m).is_ok())
            && self.top_keys.keys().all(|key| Layer::Top.allows(key))
            && self.fn_keys.keys().all(|key| Layer::Fn.allows(key))
    }

    pub(super) fn new(i: usize) -> Self {
        Self {
            name: format!("Profile {i}"),
            mode: 8,
            brightness: 5.,
            speed: 1.,
            rgb: [255, 128, 0],
            colorful: false,
            maps: BTreeMap::new(),
            captured_map: None,
            top_keys: BTreeMap::new(),
            fn_keys: BTreeMap::new(),
        }
    }

    pub(super) fn migrate_key_maps(&mut self) {
        for layer in [Layer::Top, Layer::Fn] {
            let migrated: Vec<_> = self
                .maps
                .iter()
                .filter_map(|(key, label)| {
                    let source = key.strip_prefix(layer.prefix())?;
                    if !layer.allows(source) {
                        return None;
                    }
                    let destination = self
                        .choices(layer)
                        .get(source)
                        .copied()
                        .or_else(|| KeyCode::from_legacy(label).map(Some))?;
                    Some((key.clone(), source.to_owned(), destination))
                })
                .collect();
            for (legacy, source, destination) in migrated {
                self.choices_mut(layer).insert(source, destination);
                self.maps.remove(&legacy);
            }
        }
    }
    fn choices(&self, layer: Layer) -> &BTreeMap<String, Option<KeyCode>> {
        match layer {
            Layer::Top => &self.top_keys,
            Layer::Fn => &self.fn_keys,
        }
    }
    fn choices_mut(&mut self, layer: Layer) -> &mut BTreeMap<String, Option<KeyCode>> {
        match layer {
            Layer::Top => &mut self.top_keys,
            Layer::Fn => &mut self.fn_keys,
        }
    }
    pub(super) fn map_for_layer(&self, layer: Layer) -> Result<KeyMap, String> {
        let other = match layer {
            Layer::Top => Layer::Fn,
            Layer::Fn => Layer::Top,
        };
        if let Some((key, label)) = self
            .maps
            .iter()
            .find(|(key, _)| !key.starts_with(other.prefix()))
        {
            let source = key.strip_prefix(layer.prefix()).unwrap_or(key);
            return Err(format!(
                "Reselect the saved destination '{label}' for {source}, or restore the {} layer to default. Its old label or source is ambiguous or unsupported.",
                layer.name()
            ));
        }
        let mut map = match (layer, &self.captured_map) {
            (Layer::Top, Some(bytes)) => KeyMap::from_bytes(bytes)?,
            _ => KeyMap::new(layer),
        };
        for (source, destination) in self.choices(layer) {
            map.set(source, *destination)?;
        }
        Ok(map)
    }
    /// Read only the selected source; unrelated legacy overrides are not sent.
    pub(super) fn destination_for_key(
        &self,
        layer: Layer,
        source: &str,
    ) -> Result<Option<KeyCode>, String> {
        if !layer.allows(source) {
            return Err(format!(
                "Unsupported {}-layer source: {source}.",
                layer.display_name()
            ));
        }
        if let Some(destination) = self.choices(layer).get(source) {
            return Ok(*destination);
        }
        if self
            .maps
            .contains_key(&format!("{}{source}", layer.prefix()))
        {
            return Err(format!(
                "Reselect the saved destination for {source}; its old label is ambiguous."
            ));
        }
        if layer == Layer::Top
            && let Some(bytes) = &self.captured_map
        {
            let offset = source_slot(source).unwrap() * 4;
            let record = bytes
                .get(offset..offset + 4)
                .ok_or("Invalid saved keymap length.")?;
            if record != [0; 4] {
                return KeyCode::from_record(record)
                    .map(Some)
                    .ok_or("Unsupported saved destination.".into());
            }
        }
        Ok(None)
    }
    #[cfg(test)]
    pub(super) fn top_map(&self) -> Result<KeyMap, String> {
        self.map_for_layer(Layer::Top)
    }
    pub(super) fn assign_layer(
        &mut self,
        layer: Layer,
        source: &str,
        destination: Option<KeyCode>,
    ) -> Result<(), String> {
        KeyMap::new(layer).set(source, destination)?;
        self.maps.remove(&format!("{}{source}", layer.prefix()));
        self.choices_mut(layer).insert(source.into(), destination);
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn assign_top(
        &mut self,
        source: &str,
        destination: Option<KeyCode>,
    ) -> Result<(), String> {
        self.assign_layer(Layer::Top, source, destination)
    }
    pub(super) fn restore_layer(&mut self, layer: Layer) {
        self.choices_mut(layer).clear();
        let other = match layer {
            Layer::Top => Layer::Fn,
            Layer::Fn => Layer::Top,
        };
        self.maps.retain(|key, _| key.starts_with(other.prefix()));
        if layer == Layer::Top {
            self.captured_map = None;
        }
    }
    #[cfg(test)]
    pub(super) fn restore_top(&mut self) {
        self.restore_layer(Layer::Top);
    }
    pub(super) fn key_label(&self, source: &str, fn_layer: bool) -> Option<String> {
        let layer = Layer::from_fn(fn_layer);
        if let Some(destination) = self.choices(layer).get(source) {
            return destination.map(|key| key.label().to_owned());
        }
        if let Some(label) = self.maps.get(&format!("{}{source}", layer.prefix())) {
            return Some(label.clone());
        }
        if fn_layer {
            return None;
        }
        let slot = source_slot(source)?;
        let bytes = self.captured_map.as_ref()?;
        KeyCode::from_record(bytes.get(slot * 4..slot * 4 + 4)?).map(|key| key.label().into())
    }
}
#[cfg(test)]
mod remap_tests {
    use super::*;

    #[test]
    fn selected_key_preserves_captured_destinations_and_ignores_unrelated_legacy_data() {
        let mut profile = Profile::new(1);
        let mut captured = KeyMap::new(Layer::Top);
        captured.set("A", Some(KeyCode::Calculator)).unwrap();
        profile.captured_map = Some(captured.as_bytes().to_vec());
        profile.maps.insert("0:W".into(), "ambiguous".into());
        assert_eq!(
            profile.destination_for_key(Layer::Top, "A").unwrap(),
            Some(KeyCode::Calculator)
        );
        assert!(profile.destination_for_key(Layer::Top, "W").is_err());
        profile.assign_layer(Layer::Top, "A", None).unwrap();
        assert_eq!(profile.destination_for_key(Layer::Top, "A").unwrap(), None);
        profile
            .assign_layer(Layer::Fn, "I", Some(KeyCode::Insert))
            .unwrap();
        assert_eq!(
            profile.destination_for_key(Layer::Fn, "I").unwrap(),
            Some(KeyCode::Insert)
        );
        assert!(profile.destination_for_key(Layer::Fn, "Esc").is_err());
    }
    #[test]
    fn legacy_migration_keeps_ambiguous_choices_and_fn_edits_until_reselected() {
        let mut p = Profile::new(1);
        p.maps.insert("0:A".into(), "B".into());
        p.maps.insert("0:W".into(), "7".into());
        p.maps.insert("1:A".into(), "C".into());
        p.migrate_key_maps();
        assert_eq!(p.top_keys.get("A"), Some(&Some(KeyCode::B)));
        assert!(!p.maps.contains_key("0:A"));
        assert_eq!(p.maps.get("0:W").map(String::as_str), Some("7"));
        assert_eq!(p.fn_keys.get("A"), Some(&Some(KeyCode::C)));
        assert!(p.top_map().is_err());
        assert!(p.valid()); // Keep this editable local profile instead of losing old data.
        p.assign_top("W", Some(KeyCode::Numpad7)).unwrap();
        let map = p.top_map().unwrap();
        assert_eq!(&map.as_bytes()[56 * 4..57 * 4], &KeyCode::B.record());
        assert_eq!(&map.as_bytes()[39 * 4..40 * 4], &KeyCode::Numpad7.record());
        let mut restored: Profile =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        restored.migrate_key_maps();
        assert_eq!(restored.top_map().unwrap(), map);
        assert_eq!(restored.fn_keys, p.fn_keys);
    }

    #[test]
    fn restore_one_key_preserves_other_assignments_and_restore_all_preserves_fn() {
        let mut p = Profile::new(1);
        p.captured_map = crate::protocol::captured_maps().last().cloned();
        p.maps.insert("1:A".into(), "C".into());
        let original = p.top_map().unwrap();
        p.assign_top("A", Some(KeyCode::B)).unwrap();
        let with_a = p.top_map().unwrap();
        assert_eq!(&with_a.as_bytes()[56 * 4..57 * 4], &KeyCode::B.record());
        assert_eq!(
            &with_a.as_bytes()[38 * 4..41 * 4],
            &original.as_bytes()[38 * 4..41 * 4]
        );
        p.assign_top("A", None).unwrap();
        assert_eq!(p.top_map().unwrap(), original);
        p.assign_top("Q", None).unwrap();
        let map = p.top_map().unwrap();
        assert_eq!(&map.as_bytes()[38 * 4..39 * 4], &[0; 4]);
        assert_eq!(
            &map.as_bytes()[39 * 4..41 * 4],
            &original.as_bytes()[39 * 4..41 * 4]
        );
        let decoded: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(decoded.top_map().unwrap(), map);
        p.restore_top();
        assert_eq!(p.top_map().unwrap(), KeyMap::default());
        assert_eq!(p.maps.get("1:A").map(String::as_str), Some("C"));
    }

    #[test]
    fn media_choices_and_legacy_icons_roundtrip_restore_and_preserve_keyboard_and_fn_edits() {
        let mut profile = Profile::new(1);
        profile.maps.insert("0:Esc".into(), "◖)".into());
        profile.maps.insert("1:Esc".into(), "×".into());
        profile.assign_top("A", Some(KeyCode::B)).unwrap();
        profile.migrate_key_maps();
        assert_eq!(
            profile.top_keys.get("Esc"),
            Some(&Some(KeyCode::VolumeDown))
        );
        assert_eq!(
            profile.key_label("Esc", false).as_deref(),
            Some("Volume Down")
        );
        let map = profile.top_map().unwrap();
        assert_eq!(&map.as_bytes()[4..8], &KeyCode::VolumeDown.record());
        assert_eq!(&map.as_bytes()[224..228], &KeyCode::B.record());
        let encoded = serde_json::to_string(&profile).unwrap();
        assert!(encoded.contains("volume_down"));
        let mut restored: Profile = serde_json::from_str(&encoded).unwrap();
        assert!(restored.valid());
        assert_eq!(restored.top_map().unwrap(), map);
        restored.assign_top("Esc", None).unwrap();
        let map = restored.top_map().unwrap();
        assert_eq!(&map.as_bytes()[4..8], &[0; 4]);
        assert_eq!(&map.as_bytes()[224..228], &KeyCode::B.record());
        assert_eq!(restored.maps.get("1:Esc").map(String::as_str), Some("×"));
        assert!(restored.assign_top("Fn", Some(KeyCode::Mute)).is_err());
    }

    #[test]
    fn fn_profiles_migrate_roundtrip_restore_and_keep_top_baseline() {
        let mut p = Profile::new(1);
        p.captured_map = crate::protocol::captured_maps().last().cloned();
        p.assign_top("A", Some(KeyCode::B)).unwrap();
        let top = p.top_map().unwrap();
        p.maps.insert("1:I".into(), "Ins".into());
        p.maps.insert("1:W".into(), "7".into()); // Ambiguous legacy label is preserved.
        p.migrate_key_maps();
        assert_eq!(p.fn_keys.get("I"), Some(&Some(KeyCode::Insert)));
        assert!(p.map_for_layer(Layer::Fn).is_err());
        assert_eq!(p.top_map().unwrap(), top);
        p.assign_layer(Layer::Fn, "W", Some(KeyCode::Numpad7))
            .unwrap();
        let mut p: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(p.valid());
        let map = p.map_for_layer(Layer::Fn).unwrap();
        assert_eq!(map.layer(), Layer::Fn);
        assert_eq!(&map.as_bytes()[180..184], &KeyCode::Insert.record());
        assert_eq!(&map.as_bytes()[156..160], &KeyCode::Numpad7.record());
        p.assign_layer(Layer::Fn, "I", None).unwrap();
        assert_eq!(
            &p.map_for_layer(Layer::Fn).unwrap().as_bytes()[180..184],
            &[0; 4]
        );
        assert_eq!(
            &p.map_for_layer(Layer::Fn).unwrap().as_bytes()[156..160],
            &KeyCode::Numpad7.record()
        );
        p.restore_layer(Layer::Fn);
        assert_eq!(p.map_for_layer(Layer::Fn).unwrap(), KeyMap::new(Layer::Fn));
        assert_eq!(p.top_map().unwrap(), top);
        p.assign_layer(Layer::Fn, "I", Some(KeyCode::Insert))
            .unwrap();
        let fn_map = p.map_for_layer(Layer::Fn).unwrap();
        p.restore_top();
        assert_eq!(p.map_for_layer(Layer::Fn).unwrap(), fn_map);
    }

    #[test]
    fn fn_reserved_imports_are_rejected_and_reserved_legacy_choices_stay_editable() {
        for source in crate::remap::FN_RESERVED {
            let mut p = Profile::new(1);
            assert!(p.assign_layer(Layer::Fn, source, Some(KeyCode::A)).is_err());
            assert!(p.fn_keys.is_empty());
            p.fn_keys.insert((*source).into(), Some(KeyCode::A));
            assert!(!p.valid());
            assert!(p.map_for_layer(Layer::Fn).is_err());
            p.fn_keys.clear();
            p.maps.insert(format!("1:{source}"), "A".into());
            p.migrate_key_maps();
            assert!(p.valid()); // Preserve old data so it can be reset explicitly.
            assert_eq!(p.maps.len(), 1);
            assert!(p.map_for_layer(Layer::Fn).is_err());
            assert!(p.top_map().is_ok());
            p.restore_layer(Layer::Fn);
            assert!(p.map_for_layer(Layer::Fn).is_ok());
        }
    }

    #[test]
    fn unsupported_sources_destinations_and_legacy_actions_never_become_uploads() {
        let mut p = Profile::new(1);
        assert!(p.assign_top("Fn", Some(KeyCode::B)).is_err());
        assert!(p.assign_top("missing", Some(KeyCode::B)).is_err());
        assert!(p.top_keys.is_empty());
        p.maps.insert("0:A".into(), "unknown macro".into());
        p.migrate_key_maps();
        assert!(p.top_map().is_err());
        p.assign_top("A", None).unwrap();
        assert_eq!(p.top_map().unwrap(), KeyMap::default());
        assert!(serde_json::from_str::<KeyCode>("\"media_play\"").is_err());
        for source in ["missing", "Fn"] {
            let mut imported = p.clone();
            imported.top_keys.insert(source.into(), Some(KeyCode::B));
            assert!(!imported.valid());
            assert!(imported.top_map().is_err());
        }
        for label in ["Shift", "Ctrl", "Alt", "Win", "Enter", "7", "-", "/", "."] {
            assert!(KeyCode::from_legacy(label).is_none());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct Saved {
    pub(super) profiles: Vec<Profile>,
    pub(super) selected: usize,
    #[serde(default)]
    pub(super) extras: Extras,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Extras {
    pub response: usize,
    pub sleep: usize,
    #[serde(default)]
    pub settings_v2: bool,
    pub disable_win: bool,
    pub disable_alt_f4: bool,
    pub disable_alt_tab: bool,
    // Retain inactive legacy editors on roundtrip without exposing them in release UI.
    #[serde(flatten)]
    pub legacy: BTreeMap<String, serde_json::Value>,
}
impl Default for Extras {
    fn default() -> Self {
        Self {
            response: 1,
            sleep: 1,
            settings_v2: true,
            disable_win: false,
            disable_alt_f4: false,
            disable_alt_tab: false,
            legacy: BTreeMap::new(),
        }
    }
}

impl Extras {
    pub(super) fn migrate_settings(&mut self) -> bool {
        if self.settings_v2 {
            return false;
        }
        let unsupported = !matches!(self.sleep, 0 | 2 | 4);
        self.sleep = match self.sleep {
            0 => 1,
            2 => 2,
            4 => 0,
            _ => 1,
        };
        self.settings_v2 = true;
        unsupported
    }
}

#[cfg(test)]
mod settings_tests {
    use super::Extras;

    #[test]
    fn legacy_sleep_migrates_by_meaning_and_only_once() {
        for (old, new, unsupported) in [
            (0, 1, false),
            (1, 1, true),
            (2, 2, false),
            (3, 1, true),
            (4, 0, false),
        ] {
            let mut extras: Extras = serde_json::from_str(&format!(
                "{{\"sleep\":{old},\"response\":3,\"disable_win\":true}}"
            ))
            .unwrap();
            assert!(!extras.settings_v2);
            assert_eq!(extras.migrate_settings(), unsupported);
            assert_eq!(extras.sleep, new);
            assert_eq!(extras.response, 3);
            assert!(extras.disable_win);
            assert!(!extras.migrate_settings());
            assert_eq!(extras.sleep, new);
            let mut restored: Extras =
                serde_json::from_str(&serde_json::to_string(&extras).unwrap()).unwrap();
            assert!(!restored.migrate_settings());
            assert_eq!(restored.sleep, new);
        }
        let mut new = Extras::default();
        assert!(!new.migrate_settings());
        assert_eq!(new.sleep, 1);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ScreenPreset {
    pub frames: Vec<Vec<u8>>,
    pub delays: Vec<u32>,
    pub selected: usize,
}
impl ScreenPreset {
    pub fn blank(count: usize) -> Self {
        Self {
            frames: vec![vec![0; 160 * 128 * 4]; count],
            delays: vec![50; count],
            selected: 0,
        }
    }
    pub fn valid(&self) -> bool {
        (1..=140).contains(&self.frames.len())
            && self.selected < self.frames.len()
            && self.delays.len() == self.frames.len()
            && self.frames.iter().all(|f| f.len() == 160 * 128 * 4)
            && self.delays.iter().all(|d| (10..=655350).contains(d))
    }
}
