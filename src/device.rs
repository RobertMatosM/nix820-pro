//! Linux hidraw transport. No USB access until an explicit Apply/Upload action.
use crate::protocol::{self, Report, Transport};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Keyboard {
    pub feature: PathBuf,
    pub display: Option<PathBuf>,
    pub label: String,
}

pub fn discover() -> Vec<Keyboard> {
    #[cfg(target_os = "linux")]
    {
        linux::discover()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

pub enum Job {
    Lighting(Report),
    Map(Box<crate::remap::KeyMap>),
    Tft(Vec<Vec<u8>>),
    TimeSync(Report),
    Settings(Report),
    Reset(protocol::ResetKind),
}
impl Job {
    pub fn run(self, keyboard: &Keyboard) -> Result<String, String> {
        #[cfg(target_os = "linux")]
        {
            if let Self::Map(map) = &self {
                protocol::map_reports_for_layer(map.layer(), map.as_bytes())?;
            }
            let mut t = linux::Hid::open(keyboard, matches!(self, Self::Tft(_)))?;
            match self {
                Self::Lighting(report) => protocol::apply_lighting(&mut t, report)?,
                Self::Map(map) => protocol::apply_layer_map(&mut t, map.layer(), map.as_bytes())?,
                Self::Tft(blocks) => protocol::upload_tft(&mut t, &blocks)?,
                Self::TimeSync(report) => protocol::sync_time(&mut t, report)?,
                Self::Settings(report) => protocol::apply_settings(&mut t, report)?,
                Self::Reset(kind) => {
                    protocol::apply_reset(&mut t, kind)?;
                    return Ok(format!(
                        "{} acknowledged. Saved profiles remain on this computer; Apply sends them again.",
                        kind.label()
                    ));
                }
            }
            Ok("Keyboard acknowledged the transfer. Check the result on your keyboard.".into())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = keyboard;
            Err("Device writes currently require Linux hidraw.".into())
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::ffi::{c_int, c_ulong, c_void};
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Write},
        os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
        path::Path,
        time::Duration,
    };
    // Constants from linux/hidraw.h; all feature buffers include report ID zero.
    const SET_FEATURE: c_ulong = 0xc0414806;
    const GET_FEATURE: c_ulong = 0xc0414807;
    const FEATURE_DESCRIPTOR: &str = "0613ff0901a101150026ff007508954009028102090391020a00ffb102c0";
    const TFT_DESCRIPTOR: &str =
        "0668ff0961a1010962150026ff009540750881020963150026ff0096001075089102c0";
    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: i16,
        revents: i16,
    }
    unsafe extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, arg: *mut c_void) -> c_int;
        fn poll(fds: *mut PollFd, nfds: c_ulong, timeout: c_int) -> c_int;
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }
    fn descriptor_matches(device: &Path, expected: &str) -> bool {
        std::fs::read(device.join("report_descriptor")).is_ok_and(|bytes| {
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>() == expected
        })
    }
    pub(super) fn discover() -> Vec<Keyboard> {
        let mut features = Vec::new();
        let mut displays = Vec::new();
        let Ok(entries) = std::fs::read_dir("/sys/class/hidraw") else {
            return Vec::new();
        };
        for entry in entries.flatten() {
            let Ok(device) = entry.path().join("device").canonicalize() else {
                continue;
            };
            let Some(interface) = device
                .ancestors()
                .find(|p| p.join("bInterfaceNumber").is_file())
            else {
                continue;
            };
            let Some(usb) = interface.ancestors().find(|p| p.join("idVendor").is_file()) else {
                continue;
            };
            let read = |p: PathBuf| {
                std::fs::read_to_string(p)
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            if read(usb.join("idVendor")) != "0c45" || read(usb.join("idProduct")) != "8009" {
                continue;
            }
            let node = PathBuf::from("/dev").join(entry.file_name());
            if read(interface.join("bInterfaceNumber")) == "03"
                && descriptor_matches(&device, FEATURE_DESCRIPTOR)
            {
                features.push((usb.to_path_buf(), node));
            } else if read(interface.join("bInterfaceNumber")) == "02"
                && descriptor_matches(&device, TFT_DESCRIPTOR)
            {
                displays.push((usb.to_path_buf(), node));
            }
        }
        features.sort();
        features
            .into_iter()
            .map(|(usb, feature)| Keyboard {
                label: format!(
                    "AK820 wired · {} · {}",
                    usb.file_name().unwrap_or_default().to_string_lossy(),
                    feature.display()
                ),
                display: displays
                    .iter()
                    .find(|(p, _)| *p == usb)
                    .map(|(_, p)| p.clone()),
                feature,
            })
            .collect()
    }
    pub(super) struct Hid {
        feature: File,
        display: Option<File>,
    }
    impl Hid {
        pub(super) fn open(k: &Keyboard, needs_display: bool) -> Result<Self, String> {
            // Recheck identity and descriptor immediately before opening; don't trust stored paths.
            if !discover()
                .iter()
                .any(|d| d.feature == k.feature && d.display == k.display)
            {
                return Err(
                    "Keyboard disconnected or its USB interfaces changed. Refresh devices.".into(),
                );
            }
            let open = |p: &Path| {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(0x800)
                    .open(p)
                    .map_err(|e| {
                        format!(
                            "Cannot open {}: {e}. Device access must be granted by the system.",
                            p.display()
                        )
                    })
            };
            let feature = open(&k.feature)?;
            // Advisory process lock is released when the feature fd closes.
            if unsafe { flock(feature.as_raw_fd(), 6) } != 0 {
                return Err("Another Nix820 Pro transfer is using this keyboard.".into());
            }
            let display = if needs_display {
                Some(open(
                    k.display
                        .as_deref()
                        .ok_or("TFT interface 2 is unavailable.")?,
                )?)
            } else {
                None
            };
            if let Some(file) = &display {
                let mut file = file;
                let mut buffer = [0; 64];
                // Discard only already queued replies, with a bounded loop.
                for _ in 0..32 {
                    if file.read(&mut buffer).is_err() {
                        break;
                    }
                }
            }
            Ok(Self { feature, display })
        }
    }
    impl Transport for Hid {
        fn wait(&mut self, duration: Duration) {
            std::thread::sleep(duration);
        }
        fn set_feature(&mut self, p: &Report) -> Result<(), String> {
            let mut b = [0; 65];
            b[1..].copy_from_slice(p);
            let n = unsafe { ioctl(self.feature.as_raw_fd(), SET_FEATURE, b.as_mut_ptr().cast()) };
            if n != 65 {
                return Err(format!(
                    "Feature write failed ({n}): {}",
                    std::io::Error::last_os_error()
                ));
            }
            std::thread::sleep(protocol::FEATURE_INTERVAL);
            Ok(())
        }
        fn get_feature(&mut self) -> Result<Report, String> {
            let mut b = [0; 65];
            let n = unsafe { ioctl(self.feature.as_raw_fd(), GET_FEATURE, b.as_mut_ptr().cast()) };
            if n != 65 {
                return Err(format!(
                    "Feature read failed ({n}): {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(b[1..].try_into().unwrap())
        }
        fn output(&mut self, bytes: &[u8]) -> Result<(), String> {
            let file = self
                .display
                .as_mut()
                .ok_or("TFT interface is unavailable")?;
            // hidraw consumes report ID zero before sending the 4096-byte unnumbered report.
            let mut b = Vec::with_capacity(bytes.len() + 1);
            b.push(0);
            b.extend_from_slice(bytes);
            let n = file
                .write(&b)
                .map_err(|e| format!("TFT write failed: {e}"))?;
            if n != b.len() {
                return Err("Short TFT write; upload stopped.".into());
            }
            Ok(())
        }
        fn input(&mut self) -> Result<Vec<u8>, String> {
            let file = self
                .display
                .as_mut()
                .ok_or("TFT interface is unavailable")?;
            let mut fd = PollFd {
                fd: file.as_raw_fd(),
                events: 1,
                revents: 0,
            };
            let n = unsafe { poll(&mut fd, 1, 2000) };
            if n <= 0 || fd.revents & 1 == 0 {
                return Err("TFT reply timed out or keyboard disconnected; upload stopped.".into());
            }
            let mut b = vec![0; 64];
            let n = file
                .read(&mut b)
                .map_err(|e| format!("TFT read failed: {e}"))?;
            b.truncate(n);
            Ok(b)
        }
    }
}
