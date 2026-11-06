use super::*;

impl Studio {
    pub(super) fn remember_screen(&mut self) {
        self.delays[self.frame_index] = self.delay;
        self.presets[self.preset_index] = model::ScreenPreset {
            frames: self.pixels.clone(),
            delays: self.delays.clone(),
            selected: self.frame_index,
        };
    }
    pub(super) fn select_preset(&mut self, index: usize) {
        self.remember_screen();
        self.preset_index = index;
        let preset = &self.presets[index];
        self.pixels = preset.frames.clone();
        self.delays = preset.delays.clone();
        self.frame_index = preset.selected;
        self.delay = self.delays[self.frame_index];
        self.reset_screen_view();
    }
    pub(super) fn reset_screen_view(&mut self) {
        self.preset_textures.clear();
        self.thumb_offset = 0.;
        self.undo.clear();
        self.preview = false;
        self.dirty_texture = true;
        self.started = Instant::now();
    }
    pub(super) fn new_preset(&mut self) {
        if self.presets.len() >= 32 {
            self.notify("Up to 32 screen presets can be saved.");
            return;
        }
        self.presets.push(model::ScreenPreset::blank(1));
        self.select_preset(self.presets.len() - 1);
        self.preset_scroll = (self.preset_index as f32 * 148. - 425.).max(0.);
    }
    pub(super) fn insert_frame(&mut self, duplicate: bool) {
        if self.pixels.len() >= 140 {
            self.notify("A preset can hold up to 140 frames.");
            return;
        }
        self.delays[self.frame_index] = self.delay;
        let frame = if duplicate {
            self.pixels[self.frame_index].clone()
        } else {
            vec![0; 160 * 128 * 4]
        };
        self.frame_index += 1;
        self.pixels.insert(self.frame_index, frame);
        self.delays.insert(self.frame_index, self.delay);
        self.undo.clear();
        self.preview = false;
        self.dirty_texture = true;
        let left = self.frame_index as f32 * 205.;
        self.thumb_offset = self.thumb_offset.clamp((left + 240. - 684.).max(0.), left);
    }
    pub(super) fn load_screens(&mut self) {
        if let Ok(bytes) = std::fs::read(self.paths.data.join("screen-presets.json"))
            && let Ok((index, presets)) =
                serde_json::from_slice::<(usize, Vec<model::ScreenPreset>)>(&bytes)
            && !presets.is_empty()
            && presets.len() <= 32
            && index < presets.len()
            && presets.iter().all(model::ScreenPreset::valid)
        {
            self.presets = presets;
            self.preset_index = index;
            let preset = &self.presets[index];
            self.pixels = preset.frames.clone();
            self.delays = preset.delays.clone();
            self.frame_index = preset.selected;
            self.delay = self.delays[self.frame_index];
            return;
        }
        if let Ok(bytes) = std::fs::read(self.paths.data.join("screen-project.bin"))
            && bytes.len() >= 8
            && &bytes[..4] == b"KSC1"
        {
            let count = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
            if (1..=140).contains(&count) && bytes.len() == 8 + count * 160 * 128 * 4 {
                self.pixels = bytes[8..]
                    .as_chunks::<{ 160 * 128 * 4 }>()
                    .0
                    .iter()
                    .map(|frame| frame.to_vec())
                    .collect();
                self.delay = u32::from(u16::from_le_bytes([bytes[6], bytes[7]])).max(10);
                self.delays = vec![self.delay; count];
            }
        }
        self.remember_screen();
    }
}
impl Drop for Studio {
    fn drop(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        self.remember_screen();
        let result = serde_json::to_vec(&(self.preset_index, &self.presets))
            .map_err(|e| e.to_string())
            .and_then(|data| {
                std::fs::create_dir_all(&self.paths.data).map_err(|e| e.to_string())?;
                let temp = self.paths.data.join("screen-presets.tmp");
                std::fs::write(&temp, data).map_err(|e| e.to_string())?;
                std::fs::rename(temp, self.paths.data.clone().join("screen-presets.json"))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            eprintln!("Could not save screen presets: {e}");
        }
    }
}
