//! Top and Fn overrides established by labelled source/destination captures.
use serde::{Deserialize, Serialize};

macro_rules! keys {
    (keyboard { $( $name:ident => ($label:literal, $usage:literal) ),+ $(,)? }
     media { $( $media:ident => ($media_label:literal, $consumer:literal) ),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum KeyCode { $( $name ),+, $( $media ),+ }
        impl KeyCode {
            pub const ALL: &'static [Self] = &[$( Self::$name ),+, $( Self::$media ),+];
            pub fn label(self) -> &'static str {
                match self { $( Self::$name => $label ),+, $( Self::$media => $media_label ),+ }
            }
            pub fn record(self) -> [u8; 4] {
                match self {
                    $( Self::$name => {
                        let usage: u8 = $usage;
                        if usage >= 0xe0 { [2, 1 << (usage - 0xe0), 0, 0] }
                        else { [2, 0, usage, 0] }
                    } ),+,
                    $( Self::$media => {
                        let [low, high] = ($consumer as u16).to_le_bytes();
                        [3, low, high, 0]
                    } ),+
                }
            }
        }
    }
}

keys! { keyboard {
    A => ("A", 0x04), B => ("B", 0x05), C => ("C", 0x06), D => ("D", 0x07),
    E => ("E", 0x08), F => ("F", 0x09), G => ("G", 0x0a), H => ("H", 0x0b),
    I => ("I", 0x0c), J => ("J", 0x0d), K => ("K", 0x0e), L => ("L", 0x0f),
    M => ("M", 0x10), N => ("N", 0x11), O => ("O", 0x12), P => ("P", 0x13),
    Q => ("Q", 0x14), R => ("R", 0x15), S => ("S", 0x16), T => ("T", 0x17),
    U => ("U", 0x18), V => ("V", 0x19), W => ("W", 0x1a), X => ("X", 0x1b),
    Y => ("Y", 0x1c), Z => ("Z", 0x1d),
    Digit1 => ("1", 0x1e), Digit2 => ("2", 0x1f), Digit3 => ("3", 0x20),
    Digit4 => ("4", 0x21), Digit5 => ("5", 0x22), Digit6 => ("6", 0x23),
    Digit7 => ("7", 0x24), Digit8 => ("8", 0x25), Digit9 => ("9", 0x26), Digit0 => ("0", 0x27),
    Enter => ("Enter", 0x28), Escape => ("Esc", 0x29), Backspace => ("Back", 0x2a),
    Tab => ("Tab", 0x2b), Space => ("Space", 0x2c), Minus => ("-", 0x2d),
    Equal => ("=", 0x2e), LeftBracket => ("[", 0x2f), RightBracket => ("]", 0x30),
    Backslash => ("\\", 0x31), NonUsHash => ("Non-US #", 0x32), Semicolon => (";", 0x33),
    Quote => ("'", 0x34), Grave => ("~", 0x35), Comma => (",", 0x36),
    Period => (".", 0x37), Slash => ("/", 0x38), CapsLock => ("Caps", 0x39),
    F1 => ("F1", 0x3a), F2 => ("F2", 0x3b), F3 => ("F3", 0x3c), F4 => ("F4", 0x3d),
    F5 => ("F5", 0x3e), F6 => ("F6", 0x3f), F7 => ("F7", 0x40), F8 => ("F8", 0x41),
    F9 => ("F9", 0x42), F10 => ("F10", 0x43), F11 => ("F11", 0x44), F12 => ("F12", 0x45),
    PrintScreen => ("Print", 0x46), ScrollLock => ("Scroll", 0x47), Pause => ("Pause", 0x48),
    Insert => ("Ins", 0x49), Home => ("Home", 0x4a), PageUp => ("PgUp", 0x4b),
    Delete => ("Del", 0x4c), End => ("End", 0x4d), PageDown => ("PgDn", 0x4e),
    ArrowRight => ("→", 0x4f), ArrowLeft => ("←", 0x50), ArrowDown => ("↓", 0x51), ArrowUp => ("↑", 0x52),
    NumLock => ("NumL", 0x53), NumpadDivide => ("Num /", 0x54), NumpadMultiply => ("Num *", 0x55),
    NumpadSubtract => ("Num -", 0x56), NumpadAdd => ("Num +", 0x57), NumpadEnter => ("Num Enter", 0x58),
    Numpad1 => ("Num 1", 0x59), Numpad2 => ("Num 2", 0x5a), Numpad3 => ("Num 3", 0x5b),
    Numpad4 => ("Num 4", 0x5c), Numpad5 => ("Num 5", 0x5d), Numpad6 => ("Num 6", 0x5e),
    Numpad7 => ("Num 7", 0x5f), Numpad8 => ("Num 8", 0x60), Numpad9 => ("Num 9", 0x61),
    Numpad0 => ("Num 0", 0x62), NumpadDecimal => ("Num .", 0x63),
    NonUsBackslash => ("Non-US \\", 0x64), Application => ("App", 0x65),
    LeftCtrl => ("Left Ctrl", 0xe0), LeftShift => ("Left Shift", 0xe1),
    LeftAlt => ("Left Alt", 0xe2), LeftMeta => ("Left Win", 0xe3),
    RightCtrl => ("Right Ctrl", 0xe4), RightShift => ("Right Shift", 0xe5),
    RightAlt => ("Right Alt", 0xe6), RightMeta => ("Right Win", 0xe7),
} media {
    MyComputer => ("My Computer", 0x0194), Calculator => ("Calculator", 0x0192),
    Mail => ("Mail", 0x018a), Multimedia => ("Multimedia", 0x0183),
    PlayPause => ("Play/Pause", 0x00cd), Stop => ("Stop", 0x00b7),
    PreviousTrack => ("Previous Track", 0x00b6), NextTrack => ("Next Track", 0x00b5),
    VolumeUp => ("Volume Up", 0x00e9), VolumeDown => ("Volume Down", 0x00ea), Mute => ("Mute", 0x00e2),
    BrowserHome => ("Homepage", 0x0223), BrowserRefresh => ("Web Refresh", 0x0227),
    BrowserStop => ("Web Stop", 0x0226), BrowserBack => ("Web Back", 0x0224),
    BrowserForward => ("Web Forward", 0x0225), BrowserFavorites => ("Web Favorites", 0x022a),
    BrowserSearch => ("Web Search", 0x0221),
} }

/// Same visual order as the original driver's operator-labelled media sweep.
pub const MEDIA_CHOICES: &[(KeyCode, &str)] = &[
    (KeyCode::MyComputer, "⚙"),
    (KeyCode::Calculator, "±"),
    (KeyCode::Mail, "✉"),
    (KeyCode::Multimedia, "♫"),
    (KeyCode::PlayPause, "▷"),
    (KeyCode::Stop, "▣"),
    (KeyCode::PreviousTrack, "|◀"),
    (KeyCode::NextTrack, "▶|"),
    (KeyCode::VolumeUp, "◖))"),
    (KeyCode::VolumeDown, "◖)"),
    (KeyCode::Mute, "×"),
    (KeyCode::BrowserHome, "▱"),
    (KeyCode::BrowserRefresh, "⟳"),
    (KeyCode::BrowserStop, "⊗"),
    (KeyCode::BrowserBack, "⇐"),
    (KeyCode::BrowserForward, "⇒"),
    (KeyCode::BrowserFavorites, "☆"),
    (KeyCode::BrowserSearch, "?"),
];

impl KeyCode {
    pub fn from_record(record: &[u8]) -> Option<Self> {
        Self::ALL.iter().copied().find(|key| key.record() == record)
    }

    /// Old display labels conflated numpad keys and modifier sides. Do not guess.
    pub fn from_legacy(label: &str) -> Option<Self> {
        if matches!(
            label,
            "0" | "1"
                | "2"
                | "3"
                | "4"
                | "5"
                | "6"
                | "7"
                | "8"
                | "9"
                | "Enter"
                | "Shift"
                | "Ctrl"
                | "Alt"
                | "Win"
                | "-"
                | "/"
                | "."
        ) {
            return None;
        }
        match label {
            "|" => Some(Self::Backslash),
            "<" => Some(Self::Comma),
            ">" => Some(Self::Period),
            "/?" => Some(Self::Slash),
            "*" => Some(Self::NumpadMultiply),
            "+" => Some(Self::NumpadAdd),
            _ => MEDIA_CHOICES
                .iter()
                .find(|(_, icon)| *icon == label)
                .map(|(key, _)| *key)
                .or_else(|| Self::ALL.iter().copied().find(|key| key.label() == label)),
        }
    }
}

/// Names match the physical keyboard drawing; slots come from the operator-labelled sweep.
pub const SOURCES: &[(&str, usize)] = &[
    ("Esc", 1),
    ("F1", 2),
    ("F2", 3),
    ("F3", 4),
    ("F4", 5),
    ("F5", 6),
    ("F6", 7),
    ("F7", 8),
    ("F8", 9),
    ("F9", 10),
    ("F10", 11),
    ("F11", 12),
    ("F12", 13),
    ("Del", 119),
    ("` ~", 19),
    ("1 !", 20),
    ("2 @", 21),
    ("3 #", 22),
    ("4 $", 23),
    ("5 %", 24),
    ("6 ^", 25),
    ("7 &", 26),
    ("8 *", 27),
    ("9 (", 28),
    ("0 )", 29),
    ("- _", 30),
    ("= +", 31),
    ("Back", 103),
    ("Home", 117),
    ("Tab", 37),
    ("Q", 38),
    ("W", 39),
    ("E", 40),
    ("R", 41),
    ("T", 42),
    ("Y", 43),
    ("U", 44),
    ("I", 45),
    ("O", 46),
    ("P", 47),
    ("[ {", 48),
    ("] }", 49),
    ("\\ |", 67),
    ("PgUp", 118),
    ("Caps", 55),
    ("A", 56),
    ("S", 57),
    ("D", 58),
    ("F", 59),
    ("G", 60),
    ("H", 61),
    ("J", 62),
    ("K", 63),
    ("L", 64),
    ("; :", 65),
    ("' \"", 66),
    ("Enter", 85),
    ("PgDn", 121),
    ("Shift", 73),
    ("Z", 74),
    ("X", 75),
    ("C", 76),
    ("V", 77),
    ("B", 78),
    ("N", 79),
    ("M", 80),
    (", <", 81),
    (". >", 82),
    ("/ ?", 83),
    ("Shift R", 84),
    ("↑", 91),
    ("Ctrl", 92),
    ("Win", 93),
    ("Alt", 94),
    ("Space", 95),
    ("Alt R", 96),
    ("Fn", 98),
    ("Ctrl R", 99),
    ("←", 100),
    ("↓", 102),
    ("→", 101),
];

pub fn source_slot(name: &str) -> Option<usize> {
    SOURCES
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, slot)| *slot)
}

/// Normal physical key action. This does not describe firmware Fn shortcuts.
pub fn base_key(source: &str) -> Option<KeyCode> {
    source_slot(source)?;
    Some(match source {
        "Fn" => return None,
        "` ~" => KeyCode::Grave,
        "1 !" => KeyCode::Digit1,
        "2 @" => KeyCode::Digit2,
        "3 #" => KeyCode::Digit3,
        "4 $" => KeyCode::Digit4,
        "5 %" => KeyCode::Digit5,
        "6 ^" => KeyCode::Digit6,
        "7 &" => KeyCode::Digit7,
        "8 *" => KeyCode::Digit8,
        "9 (" => KeyCode::Digit9,
        "0 )" => KeyCode::Digit0,
        "- _" => KeyCode::Minus,
        "= +" => KeyCode::Equal,
        "[ {" => KeyCode::LeftBracket,
        "] }" => KeyCode::RightBracket,
        "\\ |" => KeyCode::Backslash,
        "; :" => KeyCode::Semicolon,
        "' \"" => KeyCode::Quote,
        ", <" => KeyCode::Comma,
        ". >" => KeyCode::Period,
        "/ ?" => KeyCode::Slash,
        "Shift" => KeyCode::LeftShift,
        "Shift R" => KeyCode::RightShift,
        "Ctrl" => KeyCode::LeftCtrl,
        "Ctrl R" => KeyCode::RightCtrl,
        "Alt" => KeyCode::LeftAlt,
        "Alt R" => KeyCode::RightAlt,
        "Win" => KeyCode::LeftMeta,
        _ => {
            return KeyCode::ALL
                .iter()
                .copied()
                .find(|key| key.label() == source);
        }
    })
}

/// The Fn source restrictions come from the original driver's hint.
pub const FN_RESERVED: &[&str] = &["Esc", "Back", "Home", "Win", "Fn", "\\ |"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Top,
    Fn,
}
impl Layer {
    pub fn from_fn(value: bool) -> Self {
        if value { Self::Fn } else { Self::Top }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Top => "Top",
            Self::Fn => "Fn",
        }
    }
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Top => "Base",
            Self::Fn => "Fn",
        }
    }
    pub fn command(self) -> u8 {
        match self {
            Self::Top => 0x11,
            Self::Fn => 0x27,
        }
    }
    pub fn allows(self, source: &str) -> bool {
        source_slot(source).is_some()
            && source != "Fn"
            && (self == Self::Top || !FN_RESERVED.contains(&source))
    }
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Top => "0:",
            Self::Fn => "1:",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    layer: Layer,
    bytes: [u8; 576],
}
impl Default for KeyMap {
    fn default() -> Self {
        Self::new(Layer::Top)
    }
}
impl KeyMap {
    pub fn new(layer: Layer) -> Self {
        let mut bytes = [0; 576];
        bytes[574..].copy_from_slice(&[0xaa, 0x55]);
        Self { layer, bytes }
    }
    pub fn layer(&self) -> Layer {
        self.layer
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::from_layer_bytes(Layer::Top, bytes)
    }
    pub fn from_layer_bytes(layer: Layer, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 576 || bytes[572..] != [0, 0, 0xaa, 0x55] {
            return Err(format!(
                "Invalid {}-layer map length, reserved bytes or marker.",
                layer.name()
            ));
        }
        for (slot, record) in bytes[..572].as_chunks::<4>().0.iter().enumerate() {
            if *record == [0; 4] {
                continue;
            }
            if !SOURCES
                .iter()
                .any(|(name, index)| *index == slot && layer.allows(name))
                || KeyCode::from_record(record).is_none()
            {
                return Err(format!(
                    "Unsupported {}-layer record at slot {slot}. Reserved source keys cannot be remapped.",
                    layer.name()
                ));
            }
        }
        Ok(Self {
            layer,
            bytes: bytes.try_into().unwrap(),
        })
    }
    pub fn set(&mut self, source: &str, destination: Option<KeyCode>) -> Result<(), String> {
        if !self.layer.allows(source) {
            return Err(format!(
                "Unsupported {}-layer source key: {source}. Reserved source keys cannot be remapped.",
                self.layer.name()
            ));
        }
        let slot = source_slot(source).unwrap();
        self.bytes[slot * 4..slot * 4 + 4]
            .copy_from_slice(&destination.map(KeyCode::record).unwrap_or([0; 4]));
        Ok(())
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{FEATURE_INTERVAL, Report, Transport, apply_map};
    use std::collections::VecDeque;
    use std::time::Duration;

    fn unhex(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn normal_key_actions_cover_all_physical_sources_without_modifier_side_aliases() {
        let records: std::collections::BTreeSet<_> = SOURCES
            .iter()
            .filter(|(source, _)| *source != "Fn")
            .map(|(source, _)| base_key(source).unwrap().record())
            .collect();
        assert_eq!(records.len(), SOURCES.len() - 1);
        for (source, expected) in [
            ("Shift", KeyCode::LeftShift),
            ("Shift R", KeyCode::RightShift),
            ("Ctrl", KeyCode::LeftCtrl),
            ("Ctrl R", KeyCode::RightCtrl),
            ("Alt", KeyCode::LeftAlt),
            ("Alt R", KeyCode::RightAlt),
            ("1 !", KeyCode::Digit1),
            ("= +", KeyCode::Equal),
            ("; :", KeyCode::Semicolon),
        ] {
            assert_eq!(base_key(source), Some(expected));
        }
        assert_eq!(base_key("Fn"), None);
        assert_eq!(base_key("Calculator"), None);
    }
    #[test]
    fn fn_i_insert_and_clear_match_all_captured_writes_replies_and_pacing() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/fn-layer-remap-test-fixtures.json"
        ))
        .unwrap();
        for (index, tx) in fixture["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let mut map = KeyMap::new(Layer::Fn);
            if index == 0 {
                map.set("I", Some(KeyCode::Insert)).unwrap();
            }
            assert_eq!(map.as_bytes(), unhex(tx["body"].as_str().unwrap()));
            let mut t = Captured::new(tx);
            t.enforce_settling = true;
            crate::protocol::apply_layer_map(&mut t, map.layer(), map.as_bytes()).unwrap();
            assert_eq!(
                t.writes,
                tx["writes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| unhex(r["payload"].as_str().unwrap()))
                    .collect::<Vec<_>>()
            );
            assert!(t.replies.is_empty());
            assert!(!t.early_commit);
            for i in 1..13 {
                let simulated = t.write_times[i] - t.write_times[i - 1];
                let writes = tx["writes"].as_array().unwrap();
                let observed = Duration::from_secs_f64(
                    writes[i]["time"].as_f64().unwrap() - writes[i - 1]["time"].as_f64().unwrap(),
                );
                assert!(simulated.abs_diff(observed) < Duration::from_millis(6));
            }
        }
    }

    #[test]
    fn fn_failures_stop_at_each_stage_without_refresh_or_retry_after_rejection() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/fn-layer-remap-test-fixtures.json"
        ))
        .unwrap();
        let tx = &fixture["transactions"][0];
        let bytes = unhex(tx["body"].as_str().unwrap());
        for index in 0..13 {
            let mut t = Captured::new(tx);
            t.fail_write = Some(index);
            let error = crate::protocol::apply_layer_map(&mut t, Layer::Fn, &bytes).unwrap_err();
            assert!(error.contains("Fn-layer"));
            assert_eq!(t.writes.len(), index);
        }
        for index in 0..4 {
            for mismatch in [false, true] {
                let mut t = Captured::new(tx);
                if mismatch {
                    t.replies[index][1] ^= 1;
                } else {
                    t.replies[index][3] = 0xff;
                }
                assert!(crate::protocol::apply_layer_map(&mut t, Layer::Fn, &bytes).is_err());
                assert_eq!(t.writes.len(), [1, 2, 12, 13][index]);
                let mut t = Captured::new(tx);
                t.fail_read = Some(index);
                assert!(crate::protocol::apply_layer_map(&mut t, Layer::Fn, &bytes).is_err());
                assert_eq!(t.writes.len(), [1, 2, 12, 13][index]);
            }
        }
    }

    #[test]
    fn fn_source_restrictions_validate_before_transport_and_do_not_restrict_top() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/fn-layer-remap-test-fixtures.json"
        ))
        .unwrap();
        for source in FN_RESERVED {
            let mut map = KeyMap::new(Layer::Fn);
            assert!(map.set(source, Some(KeyCode::A)).is_err());
            assert!(map.set(source, None).is_err());
            let slot = source_slot(source).unwrap();
            let mut bytes = map.as_bytes().to_vec();
            bytes[slot * 4..slot * 4 + 4].copy_from_slice(&KeyCode::A.record());
            let mut t = Captured::new(&fixture["transactions"][0]);
            assert!(crate::protocol::apply_layer_map(&mut t, Layer::Fn, &bytes).is_err());
            assert!(t.writes.is_empty());
            assert_eq!(
                KeyMap::default().set(source, Some(KeyCode::A)).is_ok(),
                *source != "Fn"
            );
        }
        let mut bytes = KeyMap::new(Layer::Fn).as_bytes().to_vec();
        for invalid in [vec![], vec![0; 576], {
            bytes[0..4].copy_from_slice(&KeyCode::A.record());
            bytes
        }] {
            let mut t = Captured::new(&fixture["transactions"][0]);
            assert!(crate::protocol::apply_layer_map(&mut t, Layer::Fn, &invalid).is_err());
            assert!(t.writes.is_empty());
        }
    }

    #[test]
    fn source_slots_and_destination_records_match_capture_evidence() {
        let labels: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-physical-test.actions.json"
        ))
        .unwrap();
        let keys = labels["keys"].as_array().unwrap();
        assert_eq!(keys.len(), SOURCES.len());
        for (entry, (name, slot)) in keys.iter().zip(SOURCES) {
            assert_eq!(entry["ui_key"], *name);
            assert_eq!(entry["slot"], *slot);
            assert_eq!(entry["offset"], slot * 4);
        }
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-updated-fixtures.json"
        ))
        .unwrap();
        let records: Vec<_> = fixture["tables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|table| unhex(table["body"].as_str().unwrap())[152..156].to_vec())
            .collect();
        for key in KeyCode::ALL {
            assert!(
                records.iter().any(|record| *record == key.record()),
                "{key:?} absent from destination sweep"
            );
            assert_eq!(KeyCode::from_record(&key.record()), Some(*key));
        }
        assert_ne!(KeyCode::Digit7.record(), KeyCode::Numpad7.record());
        assert_ne!(KeyCode::Enter.record(), KeyCode::NumpadEnter.record());
        assert_ne!(KeyCode::LeftShift.record(), KeyCode::RightShift.record());
    }

    struct Captured {
        writes: Vec<Vec<u8>>,
        replies: VecDeque<Report>,
        fail_write: Option<usize>,
        fail_read: Option<usize>,
        read_count: usize,
        elapsed: Duration,
        write_times: Vec<Duration>,
        enforce_settling: bool,
        early_commit: bool,
    }
    impl Captured {
        fn new(tx: &serde_json::Value) -> Self {
            Self {
                writes: vec![],
                replies: tx["replies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| unhex(r["payload"].as_str().unwrap()).try_into().unwrap())
                    .collect(),
                fail_write: None,
                fail_read: None,
                read_count: 0,
                elapsed: Duration::ZERO,
                write_times: vec![],
                enforce_settling: false,
                early_commit: false,
            }
        }
    }
    impl Transport for Captured {
        fn wait(&mut self, duration: Duration) {
            self.elapsed += duration;
        }
        fn set_feature(&mut self, p: &Report) -> Result<(), String> {
            if self.fail_write == Some(self.writes.len()) {
                return Err("Injected write failure".into());
            }
            if self.enforce_settling && p[..2] == [4, 2] && self.writes.len() == 11 {
                self.early_commit = self.elapsed - self.write_times[10] < FEATURE_INTERVAL * 2;
            }
            self.writes.push(p.to_vec());
            self.write_times.push(self.elapsed);
            self.elapsed += FEATURE_INTERVAL;
            Ok(())
        }
        fn get_feature(&mut self) -> Result<Report, String> {
            let index = self.read_count;
            self.read_count += 1;
            if self.fail_read == Some(index) {
                return Err("Injected read failure".into());
            }
            let mut reply = self.replies.pop_front().ok_or("Unexpected feature read")?;
            if self.early_commit && reply[..2] == [4, 2] {
                reply[3] = 0xff;
            }
            Ok(reply)
        }
        fn output(&mut self, _: &[u8]) -> Result<(), String> {
            panic!("Maps use feature reports")
        }
        fn input(&mut self) -> Result<Vec<u8>, String> {
            panic!("Maps use feature replies")
        }
    }

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../tests/fixtures/remap-test-single-a-to-b-fixtures.json"
        ))
        .unwrap()
    }

    #[test]
    fn generated_remap_and_default_restore_match_every_captured_write_and_reply() {
        let fixture = fixture();
        let mut map = KeyMap::default();
        for (index, tx) in fixture["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            map.set("A", if index == 0 { Some(KeyCode::B) } else { None })
                .unwrap();
            assert_eq!(map.as_bytes(), unhex(tx["body"].as_str().unwrap()));
            let mut transport = Captured::new(tx);
            apply_map(&mut transport, map.as_bytes()).unwrap();
            let expected: Vec<_> = tx["writes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|write| unhex(write["payload"].as_str().unwrap()))
                .collect();
            assert_eq!(transport.writes, expected);
            assert!(transport.replies.is_empty());
        }
    }

    #[test]
    fn map_transfer_stops_at_every_failed_write_read_or_rejected_command() {
        let fixture = fixture();
        let tx = &fixture["transactions"][0];
        let mut map = KeyMap::default();
        map.set("A", Some(KeyCode::B)).unwrap();
        for index in 0..13 {
            let mut transport = Captured::new(tx);
            transport.fail_write = Some(index);
            assert!(apply_map(&mut transport, map.as_bytes()).is_err());
            assert_eq!(transport.writes.len(), index);
        }
        for (index, expected_writes) in [1, 2, 12, 13].into_iter().enumerate() {
            for kind in 0..3 {
                let mut transport = Captured::new(tx);
                match kind {
                    0 => transport.fail_read = Some(index),
                    1 => transport.replies[index][3] = 0,
                    _ => transport.replies[index][1] ^= 1,
                }
                assert!(apply_map(&mut transport, map.as_bytes()).is_err());
                assert_eq!(transport.writes.len(), expected_writes);
            }
        }
    }

    #[test]
    fn generated_media_maps_match_all_234_captured_writes_and_72_actual_replies() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/media-capture-remap-test-fixtures.json"
        ))
        .unwrap();
        let transactions = fixture["transactions"].as_array().unwrap();
        assert_eq!(transactions.len(), MEDIA_CHOICES.len());
        for (index, tx) in transactions.iter().enumerate() {
            let key: KeyCode = serde_json::from_value(tx["action"].clone()).unwrap();
            assert_eq!(key, MEDIA_CHOICES[index].0);
            assert_eq!(
                KeyCode::from_record(&unhex(tx["record"].as_str().unwrap())),
                Some(key)
            );
            let mut map = KeyMap::default();
            map.set("Esc", Some(key)).unwrap();
            assert_eq!(map.as_bytes(), unhex(tx["body"].as_str().unwrap()));
            assert_eq!(KeyMap::from_bytes(map.as_bytes()).unwrap(), map);
            let mut transport = Captured::new(tx);
            apply_map(&mut transport, map.as_bytes()).unwrap();
            let expected: Vec<_> = tx["writes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|write| unhex(write["payload"].as_str().unwrap()))
                .collect();
            assert_eq!(transport.writes, expected);
            assert!(transport.replies.is_empty());
            assert_eq!(
                transport.write_times[11] - transport.write_times[10],
                FEATURE_INTERVAL * 2
            );
            map.set("Esc", None).unwrap();
            assert_eq!(map, KeyMap::default());
        }
        assert_eq!(KeyCode::VolumeDown.record(), [3, 0xea, 0, 0]);
        assert_eq!(KeyCode::BrowserSearch.record(), [3, 0x21, 2, 0]);
        assert_eq!(KeyCode::MyComputer.record(), [3, 0x94, 1, 0]);
    }

    #[test]
    fn map_processing_intervals_match_captured_write_gaps_without_wall_clock_sleeps() {
        let fixture = fixture();
        for tx in fixture["transactions"].as_array().unwrap() {
            let map = unhex(tx["body"].as_str().unwrap());
            let mut transport = Captured::new(tx);
            apply_map(&mut transport, &map).unwrap();
            let captured = tx["writes"].as_array().unwrap();
            for index in 1..13 {
                let gap = transport.write_times[index] - transport.write_times[index - 1];
                let intervals = if [1, 2, 11, 12].contains(&index) {
                    2
                } else {
                    1
                };
                assert_eq!(gap, FEATURE_INTERVAL * intervals);
                let observed = Duration::from_secs_f64(
                    captured[index]["time"].as_f64().unwrap()
                        - captured[index - 1]["time"].as_f64().unwrap(),
                );
                assert!(
                    gap.abs_diff(observed) < Duration::from_millis(6),
                    "Write {index}: simulated {gap:?}, observed {observed:?}"
                );
            }
        }
    }

    #[test]
    fn simulated_processing_delay_rejects_early_commit_and_accepts_paced_transaction() {
        let fixture = fixture();
        let tx = &fixture["transactions"][0];
        let mut premature = Captured::new(tx);
        premature.enforce_settling = true;
        // Reproduce the old last-block→commit interval while retaining setup pacing.
        for (index, write) in tx["writes"].as_array().unwrap()[..11].iter().enumerate() {
            let report: Report = unhex(write["payload"].as_str().unwrap())
                .try_into()
                .unwrap();
            premature.set_feature(&report).unwrap();
            if index < 2 {
                premature.get_feature().unwrap();
                premature.wait(FEATURE_INTERVAL);
            }
        }
        premature.set_feature(&crate::protocol::command(2)).unwrap();
        assert_eq!(&premature.get_feature().unwrap()[..4], &[4, 2, 0, 0xff]);
        let mut paced = Captured::new(tx);
        paced.enforce_settling = true;
        apply_map(&mut paced, &unhex(tx["body"].as_str().unwrap())).unwrap();
        assert!(!paced.early_commit);
        assert!(paced.replies.is_empty());
    }

    #[test]
    fn commit_ff_is_not_retried_or_hidden_and_refresh_failure_reports_acknowledged_commit() {
        let fixture = fixture();
        let tx = &fixture["transactions"][0];
        let map = unhex(tx["body"].as_str().unwrap());
        let mut commit = Captured::new(tx);
        commit.replies[2][3] = 0xff;
        let error = apply_map(&mut commit, &map).unwrap_err();
        assert!(error.contains("Top-layer commit failed after all 9 blocks"));
        assert!(error.contains("[04, 02, 00, ff]"));
        assert_eq!(commit.writes.len(), 12); // No retry or refresh after a negative commit.
        let mut refresh = Captured::new(tx);
        refresh.replies[3][3] = 0xff;
        let error = apply_map(&mut refresh, &map).unwrap_err();
        assert!(error.contains("commit was acknowledged, but refresh failed"));
        assert_eq!(refresh.writes.len(), 13);
    }

    #[test]
    fn invalid_maps_are_rejected_before_any_transport_call() {
        let fixture = fixture();
        let default = KeyMap::default();
        let mut bad_maps = vec![vec![], vec![0; 575], vec![0; 576], vec![0; 577]];
        for (offset, record) in [
            (0, KeyCode::B.record()),
            (98 * 4, KeyCode::B.record()),
            (56 * 4, [3, 0xff, 0xff, 0]),
            (56 * 4, [2, 0, 0xff, 0]),
            (56 * 4, [2, 3, 0, 0]),
        ] {
            let mut bytes = default.as_bytes().to_vec();
            bytes[offset..offset + 4].copy_from_slice(&record);
            bad_maps.push(bytes);
        }
        for offset in 572..576 {
            let mut bytes = default.as_bytes().to_vec();
            bytes[offset] ^= 1;
            bad_maps.push(bytes);
        }
        for bytes in bad_maps {
            let mut transport = Captured::new(&fixture["transactions"][0]);
            assert!(apply_map(&mut transport, &bytes).is_err());
            assert!(transport.writes.is_empty());
            assert_eq!(transport.read_count, 0);
            assert_eq!(transport.elapsed, Duration::ZERO);
        }
        let mut map = default.clone();
        assert!(map.set("Fn", Some(KeyCode::B)).is_err());
        assert!(map.set("unknown", Some(KeyCode::B)).is_err());
        assert_eq!(map, default);
    }
}
