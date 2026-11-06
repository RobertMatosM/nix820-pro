//! Capture-backed AK820 wired transactions. Pure encoders; no device access here.
mod reset;
pub use reset::{ResetKind, apply_reset};

#[cfg(test)]
use serde::Deserialize;
use std::time::Duration;

pub type Report = [u8; 64];
/// Observed feature-report pacing. Successful writes include this processing interval.
pub const FEATURE_INTERVAL: Duration = Duration::from_millis(35);

#[cfg(test)]
#[derive(Deserialize)]
pub struct SettingsCaptureReview {
    pub transaction_count: usize,
    pub successful_completions: usize,
    pub verified_command_replies: usize,
}

#[cfg(test)]
pub fn settings_capture() -> &'static SettingsCaptureReview {
    static REVIEW: std::sync::OnceLock<SettingsCaptureReview> = std::sync::OnceLock::new();
    REVIEW.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../tests/fixtures/system-settings-review.json"
        ))
        .expect("bundled settings capture review")
    })
}

/// Fields derived from the user's ordered settings sweep. Sleep: 0/1/2/3 = never/1/5/30 min.
pub fn settings(
    level: usize,
    sleep: usize,
    win: bool,
    f4: bool,
    tab: bool,
) -> Result<Report, String> {
    if !(1..=5).contains(&level) || sleep > 3 {
        return Err("Choose response level 1–5 and a captured sleep time.".into());
    }
    let mut p = [0; 64];
    p[1] = 1;
    p[2..5].copy_from_slice(&[u8::from(tab), u8::from(f4), u8::from(win)]);
    p[6] = sleep as u8;
    p[8] = level as u8;
    p[62..].copy_from_slice(&[0xaa, 0x55]);
    Ok(p)
}

#[cfg(test)]
#[derive(Deserialize)]
pub struct CaptureReview {
    pub set_report_count: usize,
    pub successful_completions: usize,
    pub table_count: usize,
    pub unique_tables: usize,
    pub verified_command_replies: usize,
    #[serde(default)]
    pub source_slot_count: usize,
    #[serde(default)]
    pub repeated_updates: usize,
    #[serde(default)]
    pub source_slots: Vec<SourceSlot>,
}

#[cfg(test)]
#[derive(Deserialize)]
pub struct SourceSlot {
    pub slot: usize,
    pub offset: usize,
}

/// Offline findings, kept separate from the maps approved for exact replay.
#[cfg(test)]
pub fn latest_map_capture() -> &'static CaptureReview {
    static REVIEW: std::sync::OnceLock<CaptureReview> = std::sync::OnceLock::new();
    REVIEW.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-physical-test-review.json"
        ))
        .expect("bundled map capture review")
    })
}

#[cfg(test)]
#[derive(Deserialize)]
struct Fixtures {
    maps: Vec<Vec<u8>>,
    #[cfg(test)]
    lighting: Vec<String>,
}

#[cfg(test)]
pub fn captured_maps() -> Vec<Vec<u8>> {
    serde_json::from_str::<Fixtures>(include_str!("../tests/fixtures/protocol-fixtures.json"))
        .expect("bundled capture fixtures")
        .maps
}

pub fn command(op: u8) -> Report {
    let mut p = [0; 64];
    p[0] = 4;
    p[1] = op;
    p
}

pub fn lighting(
    mode: usize,
    rgb: [u8; 3],
    colorful: bool,
    brightness: f32,
    speed: f32,
) -> Result<Report, String> {
    lighting_variant(mode, rgb, colorful, brightness, speed, 0)
}

// Byte 11 toggles for Flowing in the full sweep; its meaning is unknown.
// The UI uses the observed default (zero), retaining this variant for fixture validation.
fn lighting_variant(
    mode: usize,
    rgb: [u8; 3],
    colorful: bool,
    brightness: f32,
    speed: f32,
    variant: u8,
) -> Result<Report, String> {
    if mode >= 20 || variant > 1 || (variant != 0 && mode != 15) {
        return Err("Invalid lighting mode or variant.".into());
    }
    if !brightness.is_finite()
        || !speed.is_finite()
        || !(1.0..=5.0).contains(&brightness)
        || !(1.0..=5.0).contains(&speed)
    {
        return Err("Brightness and speed must be between 1 and 5.".into());
    }
    let mut p = [0; 64];
    p[0] = if mode == 19 { 0 } else { (mode + 1) as u8 };
    p[1..4].copy_from_slice(&rgb);
    p[8] = u8::from(colorful);
    p[9] = brightness.round() as u8;
    p[10] = speed.round() as u8;
    p[11] = variant;
    // LED Off has an all-zero body in the sweep, regardless of local color controls.
    if mode == 19 {
        p[..14].fill(0);
    }
    p[14..16].copy_from_slice(&[0xaa, 0x55]);
    Ok(p)
}

pub fn map_reports(map: &[u8]) -> Result<Vec<Report>, String> {
    map_reports_for_layer(crate::remap::Layer::Top, map)
}

pub fn map_reports_for_layer(
    layer: crate::remap::Layer,
    map: &[u8],
) -> Result<Vec<Report>, String> {
    crate::remap::KeyMap::from_layer_bytes(layer, map)?;
    Ok(map.as_chunks::<64>().0.to_vec())
}

pub fn tft_image(rgba: &[u8], width: u32, height: u32) -> Result<Vec<Vec<u8>>, String> {
    tft_animation(&[rgba.to_vec()], &[50], width, height)
}

pub fn tft_animation(
    frames: &[Vec<u8>],
    delays: &[u32],
    width: u32,
    height: u32,
) -> Result<Vec<Vec<u8>>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0
        || height == 0
        || frames.is_empty()
        || frames.len() > 140
        || frames.len() != delays.len()
        || frames.iter().any(|f| Some(f.len()) != expected)
        || delays.iter().any(|d| !(10..=510).contains(d))
    {
        return Err("TFT upload needs 1–140 frames with each delay between 10 and 510 ms. Longer delays can still preview and export locally.".into());
    }
    let size = 256 + frames.len() * 128 * 128 * 2;
    let mut stream = vec![0xff; size.div_ceil(4096) * 4096];
    stream[0] = frames.len() as u8;
    for (i, (frame, delay)) in frames.iter().zip(delays).enumerate() {
        // Static capture: [1,25]. GIF capture: [140,20 x 140]. Time units inferred as 2 ms.
        stream[i + 1] = delay.div_ceil(2) as u8;
        let image =
            image::RgbaImage::from_raw(width, height, frame.clone()).ok_or("Invalid image")?;
        let fitted =
            image::imageops::resize(&image, 128, 128, image::imageops::FilterType::Triangle);
        for (j, p) in fitted.pixels().enumerate() {
            let rgb565 =
                ((p[0] as u16 >> 3) << 11) | ((p[1] as u16 >> 2) << 5) | (p[2] as u16 >> 3);
            let start = 256 + i * 32768 + j * 2;
            stream[start..start + 2].copy_from_slice(&rgb565.to_le_bytes());
        }
    }
    Ok(stream
        .as_chunks::<4096>()
        .0
        .iter()
        .map(|chunk| chunk.to_vec())
        .collect())
}

/// Calendar fields inferred from the labelled 2026-11-05 17:56:11 Thursday capture.
pub fn time_report(fields: [u8; 7]) -> Result<Report, String> {
    let [year, month, day, hour, minute, second, weekday] = fields;
    let leap = year % 4 == 0;
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if year > 99
        || day == 0
        || day > days
        || hour > 23
        || minute > 59
        || second > 59
        || !(1..=7).contains(&weekday)
    {
        return Err("Invalid local clock fields.".into());
    }
    let mut report = [0; 64];
    report[1..3].copy_from_slice(&[1, 0x5a]);
    report[3..9].copy_from_slice(&fields[..6]);
    report[10] = weekday;
    report[62..].copy_from_slice(&[0xaa, 0x55]);
    Ok(report)
}
pub fn sync_time(t: &mut impl Transport, report: Report) -> Result<(), String> {
    if report[1..3] != [1, 0x5a] || report[62..] != [0xaa, 0x55] {
        return Err("Invalid clock report.".into());
    }
    exchange(t, command(0x18))?;
    let mut begin = command(0x28);
    begin[8] = 1;
    exchange(t, begin)?;
    t.set_feature(&report)?;
    if t.get_feature()? != report {
        return Err("Keyboard did not echo the clock data; stopped.".into());
    }
    finish(t, false)
}

pub trait Transport {
    /// Send a report and allow at least FEATURE_INTERVAL for device processing.
    fn set_feature(&mut self, p: &Report) -> Result<(), String>;
    fn get_feature(&mut self) -> Result<Report, String>;
    /// Additional settling time at transaction boundaries, injectable for tests.
    fn wait(&mut self, duration: Duration);
    fn output(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn input(&mut self) -> Result<Vec<u8>, String>;
}

fn exchange(t: &mut impl Transport, p: Report) -> Result<(), String> {
    t.set_feature(&p)?;
    let r = t.get_feature()?;
    if r[..3] != p[..3] || r[3] != 1 {
        return Err(format!(
            "Keyboard rejected command {:02x} (reply {:02x?}); stopped.",
            p[1],
            &r[..4]
        ));
    }
    Ok(())
}
fn finish(t: &mut impl Transport, refresh: bool) -> Result<(), String> {
    exchange(t, command(2))?;
    if refresh {
        exchange(t, command(0xf0))?;
    }
    Ok(())
}
pub fn apply_lighting(t: &mut impl Transport, report: Report) -> Result<(), String> {
    exchange(t, command(0x18))?;
    let mut begin = command(0x13);
    begin[8] = 1;
    exchange(t, begin)?;
    t.set_feature(&report)?;
    finish(t, true)
}
pub fn apply_settings(t: &mut impl Transport, report: Report) -> Result<(), String> {
    // Reject malformed data before opening a transaction; no reset opcodes here.
    if report
        != settings(
            report[8] as usize,
            report[6] as usize,
            report[4] != 0,
            report[3] != 0,
            report[2] != 0,
        )?
    {
        return Err("Invalid settings report.".into());
    }
    exchange(t, command(0x18))?;
    let mut begin = command(0x17);
    begin[2] = 1;
    begin[8] = 1;
    exchange(t, begin)?;
    t.set_feature(&report)?;
    // Captured sequence has no data echo and no 04 f0 refresh.
    finish(t, false)
}
#[cfg(test)]
pub fn apply_map(t: &mut impl Transport, map: &[u8]) -> Result<(), String> {
    apply_layer_map(t, crate::remap::Layer::Top, map)
}
pub fn apply_layer_map(
    t: &mut impl Transport,
    layer: crate::remap::Layer,
    map: &[u8],
) -> Result<(), String> {
    let reports = map_reports_for_layer(layer, map)?;
    let name = layer.name();
    exchange(t, command(0x18)).map_err(|e| format!("{name}-layer setup failed: {e}"))?;
    t.wait(FEATURE_INTERVAL);
    let mut begin = command(layer.command());
    begin[8] = 9;
    exchange(t, begin).map_err(|e| format!("{name}-layer map setup failed: {e}"))?;
    t.wait(FEATURE_INTERVAL);
    for (index, p) in reports.iter().enumerate() {
        t.set_feature(p)
            .map_err(|e| format!("{name}-layer block {}/9 failed: {e}", index + 1))?;
    }
    // All 227 captured map transfers leave ~70 ms from the last block to commit:
    // the write's existing 35 ms interval plus another 35 ms settling period.
    t.wait(FEATURE_INTERVAL);
    exchange(t, command(2)).map_err(|e| {
        format!("{name}-layer commit failed after all 9 blocks; keyboard state is unconfirmed. {e}")
    })?;
    t.wait(FEATURE_INTERVAL);
    exchange(t, command(0xf0))
        .map_err(|e| format!("{name}-layer commit was acknowledged, but refresh failed. {e}"))
}
pub fn upload_tft(t: &mut impl Transport, blocks: &[Vec<u8>]) -> Result<(), String> {
    let count = blocks.first().and_then(|b| b.first()).copied().unwrap_or(0) as usize;
    if !(1..=140).contains(&count)
        || blocks.len() != (256 + count * 32768usize).div_ceil(4096)
        || blocks.iter().any(|b| b.len() != 4096)
    {
        return Err("Invalid TFT header or block count.".into());
    }
    exchange(t, command(0x18))?;
    let mut begin = command(0x72);
    begin[2] = if count == 1 { 2 } else { 3 };
    begin[8..10].copy_from_slice(&(blocks.len() as u16).to_le_bytes());
    exchange(t, begin)?;
    for block in blocks {
        t.output(block)?;
        let ack = t.input()?;
        if ack.len() != 64 || !ack.starts_with(&[1, 0x5a, 2]) {
            return Err("Unexpected TFT acknowledgement; upload stopped.".into());
        }
    }
    finish(t, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        writes: Vec<Vec<u8>>,
        current: Report,
        bad: bool,
        blocks: usize,
        bad_tft: bool,
        bad_clock: bool,
        reject_op: Option<u8>,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                writes: Vec::new(),
                current: [0; 64],
                bad: false,
                blocks: 0,
                bad_tft: false,
                bad_clock: false,
                reject_op: None,
            }
        }
    }
    impl Transport for Fake {
        fn wait(&mut self, _: Duration) {}
        fn set_feature(&mut self, p: &Report) -> Result<(), String> {
            self.current = *p;
            self.writes.push(p.to_vec());
            Ok(())
        }
        fn get_feature(&mut self) -> Result<Report, String> {
            let mut p = self.current;
            if p[0] == 4 {
                p[3] = if self.bad || self.reject_op == Some(p[1]) {
                    0xff
                } else {
                    1
                };
            } else if self.bad_clock {
                p[8] ^= 1;
            }
            Ok(p)
        }
        fn output(&mut self, b: &[u8]) -> Result<(), String> {
            self.blocks += 1;
            self.writes.push(b.to_vec());
            Ok(())
        }
        fn input(&mut self) -> Result<Vec<u8>, String> {
            if self.bad_tft {
                return Ok(vec![1, 0x5a, 2]);
            }
            let mut ack = vec![0; 64];
            ack[..3].copy_from_slice(&[1, 0x5a, 2]);
            Ok(ack)
        }
    }
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
    #[test]
    fn settings_match_all_captured_writes_and_actual_replies() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/system-settings-fixtures.json"
        ))
        .unwrap();
        let transactions = fixture["transactions"].as_array().unwrap();
        assert_eq!(transactions.len(), 12);
        struct Captured {
            writes: Vec<Vec<u8>>,
            replies: std::collections::VecDeque<Report>,
        }
        impl Transport for Captured {
            fn wait(&mut self, _: Duration) {}
            fn set_feature(&mut self, p: &Report) -> Result<(), String> {
                self.writes.push(p.to_vec());
                Ok(())
            }
            fn get_feature(&mut self) -> Result<Report, String> {
                self.replies
                    .pop_front()
                    .ok_or("Unexpected feature read".into())
            }
            fn output(&mut self, _: &[u8]) -> Result<(), String> {
                panic!("Settings use feature reports")
            }
            fn input(&mut self) -> Result<Vec<u8>, String> {
                panic!("Settings have no interrupt input")
            }
        }
        for tx in transactions {
            let report = settings(
                tx["level"].as_u64().unwrap() as usize,
                tx["sleep"].as_u64().unwrap() as usize,
                tx["disable_win"].as_bool().unwrap(),
                tx["disable_alt_f4"].as_bool().unwrap(),
                tx["disable_alt_tab"].as_bool().unwrap(),
            )
            .unwrap();
            let mut transport = Captured {
                writes: vec![],
                replies: tx["replies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| unhex(r["payload"].as_str().unwrap()).try_into().unwrap())
                    .collect(),
            };
            apply_settings(&mut transport, report).unwrap();
            assert_eq!(
                transport.writes,
                tx["writes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| unhex(r.as_str().unwrap()))
                    .collect::<Vec<_>>()
            );
            assert!(transport.replies.is_empty());
        }
        let review = settings_capture();
        assert_eq!(review.transaction_count, 12);
        assert_eq!(review.successful_completions, 48);
        assert_eq!(review.verified_command_replies, 36);
    }
    #[test]
    fn settings_validate_before_writes_and_stop_on_each_rejected_command() {
        assert!(settings(0, 0, false, false, false).is_err());
        assert!(settings(6, 0, false, false, false).is_err());
        assert!(settings(5, 4, false, false, false).is_err());
        let valid = settings(5, 3, true, true, true).unwrap();
        for offset in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 61, 62, 63] {
            let mut invalid = valid;
            invalid[offset] = 0xff;
            let mut t = Fake::default();
            assert!(apply_settings(&mut t, invalid).is_err());
            assert!(t.writes.is_empty());
        }
        for (op, writes) in [(0x18, 1), (0x17, 2), (0x02, 4)] {
            let mut t = Fake {
                reject_op: Some(op),
                ..Default::default()
            };
            assert!(apply_settings(&mut t, valid).is_err());
            assert_eq!(t.writes.len(), writes);
        }
        // All separately established field values can be combined, including disabling locks again.
        for level in 1..=5 {
            for sleep in 0..4 {
                for mask in 0..8 {
                    let p = settings(level, sleep, mask & 1 != 0, mask & 2 != 0, mask & 4 != 0)
                        .unwrap();
                    assert_eq!(
                        &p[2..5],
                        &[
                            u8::from(mask & 4 != 0),
                            u8::from(mask & 2 != 0),
                            u8::from(mask & 1 != 0)
                        ]
                    );
                    apply_settings(&mut Fake::default(), p).unwrap();
                }
            }
        }
    }
    #[test]
    fn all_lighting_changes_match_capture_bytes() {
        let fixture: Fixtures =
            serde_json::from_str(include_str!("../tests/fixtures/protocol-fixtures.json")).unwrap();
        assert_eq!(fixture.lighting.len(), 111);
        let mut modes = std::collections::BTreeSet::new();
        for raw in fixture.lighting {
            let b = unhex(&raw);
            let mode = if b[0] == 0 { 19 } else { b[0] as usize - 1 };
            modes.insert(mode);
            assert_eq!(
                lighting_variant(
                    mode,
                    b[1..4].try_into().unwrap(),
                    b[8] != 0,
                    b[9].max(1) as f32,
                    b[10].max(1) as f32,
                    b[11]
                )
                .unwrap()
                .as_slice(),
                b
            );
        }
        assert_eq!(modes.len(), 20);
        let off = lighting(19, [255, 128, 99], true, 5., 3.).unwrap();
        assert_eq!(&off[..14], &[0; 14]);
        assert_eq!(&off[14..16], &[0xaa, 0x55]);
        assert!(lighting(20, [0; 3], false, 5., 1.).is_err());
        assert!(lighting(8, [0; 3], false, f32::NAN, 1.).is_err());
    }
    #[test]
    fn captured_map_transactions_and_unknown_maps() {
        let maps = captured_maps();
        assert_eq!(maps.len(), 4);
        for map in maps {
            let mut t = Fake::default();
            apply_map(&mut t, &map).unwrap();
            assert_eq!(t.writes.len(), 13);
            assert_eq!(t.writes[2..11].concat(), map);
        }
        let mut t = Fake::default();
        assert!(apply_map(&mut t, &[0; 576]).is_err());
        assert!(t.writes.is_empty());
    }
    #[test]
    fn destination_sweep_validates_known_keyboard_modifier_and_media_records() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-updated-fixtures.json"
        ))
        .unwrap();
        let tables = fixture["tables"].as_array().unwrap();
        let review: CaptureReview = serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-updated-review.json"
        ))
        .unwrap();
        assert_eq!(tables.len(), 132);
        assert_eq!(review.table_count, tables.len());
        assert_eq!(review.set_report_count, 13 * tables.len());
        assert_eq!(review.successful_completions, review.set_report_count);
        assert_eq!(review.verified_command_replies, 4 * tables.len());
        let mut unique = std::collections::BTreeSet::new();
        let mut types = [0; 3];
        for table in tables {
            assert_eq!(table["command"], "0411");
            let body = unhex(table["body"].as_str().unwrap());
            assert_eq!(body.len(), 576);
            assert_eq!(&body[160..164], &[2, 0, 0x1c, 0]);
            assert_eq!(&body[574..], &[0xaa, 0x55]);
            assert!(
                body[..574].iter().enumerate().all(|(i, b)| {
                    (152..156).contains(&i) || (160..164).contains(&i) || *b == 0
                })
            );
            let kind = match body[152..156] {
                [2, 0, _, 0] => 0,
                [2, mask, 0, 0] if mask.is_power_of_two() => 1,
                [3, _, _, 0] => 2,
                _ => panic!("Unexpected destination record"),
            };
            types[kind] += 1;
            let supported = crate::remap::KeyCode::from_record(&body[152..156]).is_some();
            assert_eq!(map_reports(&body).is_ok(), supported);
            unique.insert(body);
        }
        assert_eq!(types, [106, 8, 18]);
        assert_eq!(unique.len(), 129);
        assert_eq!(review.unique_tables, unique.len());
    }
    #[test]
    fn physical_sweep_accumulates_81_slots_and_preserves_fn_restriction() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/exclusive-config-physical-test-fixtures.json"
        ))
        .unwrap();
        let review = latest_map_capture();
        let tables = fixture["tables"].as_array().unwrap();
        assert_eq!(tables.len(), 89);
        assert_eq!(review.table_count, tables.len());
        assert_eq!(review.set_report_count, 1157);
        assert_eq!(review.successful_completions, 1157);
        assert_eq!(review.verified_command_replies, 356);
        assert_eq!(review.source_slot_count, 81);
        assert_eq!(review.repeated_updates, 8);
        let mut previous = vec![0; 576];
        previous[574..].copy_from_slice(&[0xaa, 0x55]);
        let mut unique = std::collections::BTreeSet::new();
        let mut repeats = 0;
        let mut seen = std::collections::BTreeSet::new();
        for table in tables {
            let body = unhex(table["body"].as_str().unwrap());
            assert_eq!(body.len(), 576);
            assert_eq!(&body[572..], &[0, 0, 0xaa, 0x55]);
            let mut changed = 0;
            for (slot, record) in body[..572].as_chunks::<4>().0.iter().enumerate() {
                assert!(*record == [0; 4] || *record == [3, 0x94, 1, 0]);
                if record != &previous[slot * 4..slot * 4 + 4] {
                    assert_eq!(*record, [3, 0x94, 1, 0]);
                    seen.insert(slot);
                    changed += 1;
                }
            }
            assert!(changed <= 1);
            repeats += usize::from(changed == 0);
            assert_eq!(map_reports(&body).is_ok(), body[98 * 4..99 * 4] == [0; 4]);
            unique.insert(body.clone());
            previous = body;
        }
        assert_eq!(unique.len(), review.unique_tables);
        assert_eq!(repeats, review.repeated_updates);
        assert_eq!(seen.len(), review.source_slot_count);
        assert_eq!(review.source_slots.len(), seen.len());
        for source in &review.source_slots {
            assert!(seen.contains(&source.slot));
            assert_eq!(source.offset, source.slot * 4);
        }
    }
    #[test]
    fn rejection_stops_before_writing_configuration() {
        let mut t = Fake {
            bad: true,
            ..Default::default()
        };
        assert!(
            apply_lighting(&mut t, lighting(8, [255, 128, 0], false, 5., 1.).unwrap()).is_err()
        );
        assert_eq!(t.writes.len(), 1);
    }
    #[test]
    fn static_tft_header_pixels_padding_and_acknowledgements() {
        let blocks = tft_image(&[255, 0, 0, 255].repeat(128 * 128), 128, 128).unwrap();
        let b = blocks.concat();
        assert_eq!(&b[..2], &[1, 0x19]);
        assert!(b[2..256].iter().all(|x| *x == 255));
        assert!(
            b[256..33024]
                .as_chunks::<2>()
                .0
                .iter()
                .all(|p| *p == [0, 0xf8])
        );
        assert!(b[33024..].iter().all(|x| *x == 255));
        let mut t = Fake::default();
        upload_tft(&mut t, &blocks).unwrap();
        assert_eq!(t.blocks, 9);
        assert_eq!(&t.writes[1][..9], &[4, 0x72, 2, 0, 0, 0, 0, 0, 9]);
        assert!(tft_image(&[], 0, 0).is_err());
    }
    #[test]
    fn tft_short_ack_stops_before_next_block_or_commit() {
        let blocks = tft_image(&[0, 255, 0, 255].repeat(128 * 128), 128, 128).unwrap();
        let mut t = Fake {
            bad_tft: true,
            ..Default::default()
        };
        assert!(upload_tft(&mut t, &blocks).is_err());
        assert_eq!(t.blocks, 1);
        assert_eq!(t.writes.len(), 3); // two setup commands and one image block
        assert!(!t.writes.iter().any(|p| p.len() == 64 && p[..2] == [4, 2]));
    }
    #[test]
    fn animated_upload_matches_captured_header_begin_and_block_count() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/tft-protocol-fixtures.json"))
                .unwrap();
        let blocks =
            tft_animation(&vec![vec![0; 128 * 128 * 4]; 140], &[40; 140], 128, 128).unwrap();
        assert_eq!(blocks.len(), 1121);
        assert_eq!(
            &blocks[0][..256],
            unhex(fixture["gif_header"].as_str().unwrap())
        );
        let mut t = Fake::default();
        upload_tft(&mut t, &blocks).unwrap();
        assert_eq!(t.blocks, 1121);
        assert_eq!(t.writes[1], unhex(fixture["gif_begin"].as_str().unwrap()));
        assert_eq!(t.writes.last().unwrap(), &command(2));
        let mut bad = Fake {
            bad_tft: true,
            ..Default::default()
        };
        assert!(upload_tft(&mut bad, &blocks).is_err());
        assert_eq!(bad.blocks, 1);
        assert!(tft_animation(&vec![vec![0; 128 * 128 * 4]; 141], &[40; 141], 128, 128).is_err());
        assert!(tft_animation(&[vec![0; 128 * 128 * 4]], &[512], 128, 128).is_err());
    }
    #[test]
    fn time_sync_matches_all_captured_writes_and_stops_on_rejection() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/tft-protocol-fixtures.json"))
                .unwrap();
        let report = time_report([26, 11, 5, 17, 56, 11, 4]).unwrap();
        let mut t = Fake::default();
        sync_time(&mut t, report).unwrap();
        let expected: Vec<Vec<u8>> = fixture["time_reports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| unhex(s.as_str().unwrap()))
            .collect();
        assert_eq!(t.writes, expected);
        let mut bad = Fake {
            bad: true,
            ..Default::default()
        };
        assert!(sync_time(&mut bad, report).is_err());
        assert_eq!(bad.writes.len(), 1);
        assert!(time_report([26, 2, 30, 17, 56, 11, 4]).is_err());
        let mut bad_echo = Fake {
            bad_clock: true,
            ..Default::default()
        };
        assert!(sync_time(&mut bad_echo, report).is_err());
        assert_eq!(bad_echo.writes.len(), 3);
        assert!(!bad_echo.writes.contains(&command(2).to_vec()));
    }
}
