use std::path::Path;
use weezl::{BitOrder, decode::Decoder, encode::Encoder};
/// GIF89a writer with an explicit RGB332 palette and repeating animation.
#[cfg(test)]
pub fn write(path: &Path, frames: &[Vec<u8>], delay: u32) -> Result<(), String> {
    write_animation(path, frames, &vec![delay; frames.len()])
}
pub fn write_animation(path: &Path, frames: &[Vec<u8>], delays: &[u32]) -> Result<(), String> {
    if frames.is_empty() || frames.len() != delays.len() {
        return Err("Invalid animation timing".into());
    }
    let mut out = b"GIF89a".to_vec();
    out.extend_from_slice(&160u16.to_le_bytes());
    out.extend_from_slice(&128u16.to_le_bytes());
    out.extend([0xf7, 0, 0]);
    for i in 0..256u16 {
        out.extend([
            (((i >> 5) & 7) * 255 / 7) as u8,
            (((i >> 2) & 7) * 255 / 7) as u8,
            ((i & 3) * 255 / 3) as u8,
        ]);
    }
    out.extend_from_slice(b"\x21\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00");
    for (frame, delay) in frames.iter().zip(delays) {
        if frame.len() != 160 * 128 * 4 {
            return Err("Invalid screen frame size".into());
        }
        out.extend([0x21, 0xf9, 4, 4]);
        out.extend_from_slice(&((delay / 10).clamp(1, 65535) as u16).to_le_bytes());
        out.extend([0, 0, 0x2c, 0, 0, 0, 0, 160, 0, 128, 0, 0, 8]);
        let pixels: Vec<u8> = frame
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| (c[0] & 0xe0) | ((c[1] & 0xe0) >> 3) | (c[2] >> 6))
            .collect();
        let compressed = Encoder::new(BitOrder::Lsb, 8)
            .encode(&pixels)
            .map_err(|e| e.to_string())?;
        for block in compressed.chunks(255) {
            out.push(block.len() as u8);
            out.extend(block);
        }
        out.push(0);
    }
    out.push(0x3b);
    std::fs::write(path, out).map_err(|e| e.to_string())
}
struct Reader {
    data: Vec<u8>,
    i: usize,
}
impl Reader {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.i.checked_add(n).ok_or("GIF offset overflow")?;
        if end > self.data.len() {
            return Err("Truncated GIF".into());
        }
        let start = self.i;
        self.i = end;
        Ok(&self.data[start..end])
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<usize, String> {
        let a = self.take(2)?;
        Ok(u16::from_le_bytes([a[0], a[1]]) as usize)
    }
    fn blocks(&mut self) -> Result<Vec<u8>, String> {
        let mut v = vec![];
        loop {
            let n = self.byte()? as usize;
            if n == 0 {
                break;
            }
            v.extend_from_slice(self.take(n)?);
        }
        Ok(v)
    }
}
#[cfg(test)]
pub fn read(path: &Path) -> Result<Vec<Vec<u8>>, String> {
    read_animation(path).map(|a| a.0)
}
pub fn read_animation(path: &Path) -> Result<(Vec<Vec<u8>>, Vec<u32>), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() > 64 * 1024 * 1024 {
        return Err("GIF exceeds 64 MB".into());
    }
    let mut r = Reader { data, i: 0 };
    let header = r.take(6)?;
    if header != b"GIF87a" && header != b"GIF89a" {
        return Err("Not a GIF".into());
    }
    let w = r.word()?;
    let h = r.word()?;
    if w == 0 || h == 0 || w * h > 16_000_000 {
        return Err("GIF dimensions are too large".into());
    }
    let flags = r.byte()?;
    r.take(2)?;
    let global = if flags & 128 != 0 {
        r.take(3 * (1 << ((flags & 7) + 1)))?.to_vec()
    } else {
        vec![]
    };
    let mut canvas = vec![0u8; w * h * 4];
    let mut frames = vec![];
    let mut delays = vec![];
    let mut delay = 50;
    let mut transparent = None;
    let mut disposal = 0;
    loop {
        match r.byte()? {
            0x3b => break,
            0x21 => {
                let kind = r.byte()?;
                let b = r.blocks()?;
                if kind == 0xf9 && b.len() >= 4 {
                    delay = (u16::from_le_bytes([b[1], b[2]]) as u32 * 10).max(10);
                    disposal = (b[0] >> 2) & 7;
                    transparent = if b[0] & 1 != 0 { Some(b[3]) } else { None };
                }
            }
            0x2c => {
                let left = r.word()?;
                let top = r.word()?;
                let iw = r.word()?;
                let ih = r.word()?;
                if iw == 0 || ih == 0 || iw * ih > 16_000_000 {
                    return Err("Invalid GIF frame dimensions".into());
                }
                let f = r.byte()?;
                let local = if f & 128 != 0 {
                    r.take(3 * (1 << ((f & 7) + 1)))?.to_vec()
                } else {
                    global.clone()
                };
                if local.is_empty() {
                    return Err("Missing GIF palette".into());
                }
                let size = r.byte()?;
                if !(2..=8).contains(&size) {
                    return Err("Invalid GIF LZW size".into());
                }
                let compressed = r.blocks()?;
                let mut indexed = vec![0; iw * ih];
                let mut decoder = Decoder::new(BitOrder::Lsb, size);
                let mut input = 0;
                let mut output = 0;
                while output < indexed.len() {
                    let result = decoder.decode_bytes(&compressed[input..], &mut indexed[output..]);
                    result.status.map_err(|e| e.to_string())?;
                    input += result.consumed_in;
                    output += result.consumed_out;
                    if result.consumed_in == 0 && result.consumed_out == 0 {
                        break;
                    }
                }
                if output != iw * ih {
                    return Err("Invalid GIF image stream".into());
                }
                let old = if disposal == 3 {
                    Some(canvas.clone())
                } else {
                    None
                };
                let rows: Vec<usize> = if f & 64 != 0 {
                    [(0, 8), (4, 8), (2, 4), (1, 2)]
                        .iter()
                        .flat_map(|&(s, d)| (s..ih).step_by(d))
                        .collect()
                } else {
                    (0..ih).collect()
                };
                for (source, y) in rows.into_iter().enumerate() {
                    for x in 0..iw {
                        let index = indexed[source * iw + x];
                        if Some(index) == transparent || x + left >= w || y + top >= h {
                            continue;
                        }
                        let pi = index as usize * 3;
                        if pi + 2 >= local.len() {
                            return Err("GIF palette index out of bounds".into());
                        }
                        let dst = ((y + top) * w + x + left) * 4;
                        canvas[dst..dst + 4].copy_from_slice(&[
                            local[pi],
                            local[pi + 1],
                            local[pi + 2],
                            255,
                        ]);
                    }
                }
                let im = image::RgbaImage::from_raw(w as u32, h as u32, canvas.clone())
                    .ok_or("Invalid GIF canvas")?;
                frames.push(
                    image::imageops::resize(&im, 160, 128, image::imageops::FilterType::Triangle)
                        .into_raw(),
                );
                delays.push(delay);
                if let Some(old) = old {
                    canvas = old
                } else if disposal == 2 {
                    for y in top..(top + ih).min(h) {
                        for x in left..(left + iw).min(w) {
                            canvas[(y * w + x) * 4..(y * w + x) * 4 + 4].fill(0);
                        }
                    }
                }
                transparent = None;
                disposal = 0;
                delay = 50;
                if frames.len() > 140 {
                    return Err("GIF has more than 140 frames; import a shorter animation.".into());
                }
            }
            _ => return Err("Unsupported GIF block".into()),
        }
    }
    if frames.is_empty() {
        Err("No frames in GIF".into())
    } else {
        Ok((frames, delays))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animated_roundtrip() {
        let p = std::env::temp_dir().join(format!("studio-gif-{}.gif", std::process::id()));
        let frames = vec![
            [255, 0, 0, 255].repeat(160 * 128),
            [0, 255, 0, 255].repeat(160 * 128),
        ];
        write(&p, &frames, 50).unwrap();
        let read = read(&p).unwrap();
        assert_eq!(read, frames);
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn variable_gif_delays_survive_import_and_export() {
        let path = std::env::temp_dir().join(format!("studio-timing-{}.gif", std::process::id()));
        let frames = vec![
            [255, 0, 0, 255].repeat(160 * 128),
            [0, 255, 0, 255].repeat(160 * 128),
        ];
        write_animation(&path, &frames, &[40, 900]).unwrap();
        let (decoded, delays) = read_animation(&path).unwrap();
        assert_eq!(decoded, frames);
        assert_eq!(delays, [40, 900]);
        std::fs::remove_file(path).unwrap();
    }
}
