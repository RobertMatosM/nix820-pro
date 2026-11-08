//! Linux hidraw transport. No USB access until an explicit Apply/Upload action.
use crate::protocol::{self, Report, Transport};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Wired,
    Receiver,
}

#[derive(Clone, Debug)]
pub struct Keyboard {
    pub connection: Connection,
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
    Remap {
        layer: crate::remap::Layer,
        source: String,
        destination: crate::remap::KeyCode,
    },
    Tft(Vec<Vec<u8>>),
    TimeSync(Report),
    Settings(Report),
    Reset(protocol::ResetKind),
}
impl Job {
    pub fn check_connection(&self, connection: Connection) -> Result<(), String> {
        if connection == Connection::Wired && matches!(self, Self::Remap { .. }) {
            return Err(
                "Selected-key transfers use the receiver. Use Apply layer over wired USB.".into(),
            );
        }
        if connection == Connection::Receiver
            && !matches!(
                self,
                Self::Lighting(_) | Self::Settings(_) | Self::Reset(_) | Self::Remap { .. }
            )
        {
            return Err(
                "Whole-layer keymaps and display transfers require wired USB. The receiver supports selected-key remapping, lighting, settings and resets."
                    .into(),
            );
        }
        Ok(())
    }
    pub fn run(self, keyboard: &Keyboard) -> Result<String, String> {
        self.check_connection(keyboard.connection)?;
        #[cfg(target_os = "linux")]
        {
            if keyboard.connection == Connection::Receiver {
                // Reject malformed local data before opening any device handle.
                match &self {
                    Self::Lighting(report) => {
                        protocol::wireless::lighting(report)?;
                    }
                    Self::Settings(report) => {
                        protocol::wireless::settings(report)?;
                    }
                    Self::Remap {
                        layer,
                        source,
                        destination,
                    } => {
                        protocol::wireless::remap(*layer, source, *destination)?;
                    }
                    _ => {}
                }
                let mut receiver = linux::Receiver::open(keyboard)?;
                match self {
                    Self::Lighting(report) => {
                        protocol::wireless::apply_lighting(&mut receiver, &report)?
                    }
                    Self::Settings(report) => {
                        protocol::wireless::apply_settings(&mut receiver, &report)?
                    }
                    Self::Remap {
                        layer,
                        source,
                        destination,
                    } => {
                        protocol::wireless::apply_remap(
                            &mut receiver,
                            layer,
                            &source,
                            destination,
                        )?;
                        return Ok(format!(
                            "{} {source} → {} acknowledged over 2.4 GHz. Check the physical key.",
                            layer.display_name(),
                            destination.label()
                        ));
                    }
                    Self::Reset(kind) => {
                        protocol::wireless::apply_reset(&mut receiver, kind)?;
                        return Ok(format!(
                            "{} acknowledged over 2.4 GHz. Check your keyboard; saved profiles and display presets remain on this computer.",
                            kind.label()
                        ));
                    }
                    _ => unreachable!(),
                }
                return Ok("2.4 GHz receiver acknowledged the transfer. Check the result on your keyboard.".into());
            }
            if let Self::Map(map) = &self {
                protocol::map_reports_for_layer(map.layer(), map.as_bytes())?;
            }
            let mut t = linux::Hid::open(keyboard, matches!(self, Self::Tft(_)))?;
            match self {
                Self::Lighting(report) => protocol::apply_lighting(&mut t, report)?,
                Self::Map(map) => protocol::apply_layer_map(&mut t, map.layer(), map.as_bytes())?,
                Self::Remap { .. } => unreachable!("receiver-only job checked before opening"),
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
        time::{Duration, Instant},
    };
    // Constants from linux/hidraw.h; all feature buffers include report ID zero.
    const SET_FEATURE: c_ulong = 0xc0414806;
    const GET_FEATURE: c_ulong = 0xc0414807;
    const FEATURE_DESCRIPTOR: &str = "0613ff0901a101150026ff007508954009028102090391020a00ffb102c0";
    const TFT_DESCRIPTOR: &str =
        "0668ff0961a1010962150026ff009540750881020963150026ff0096001075089102c0";
    const RECEIVER_DESCRIPTOR: &str =
        "0660ff0961a1010962150026ff009520750881020963150026ff00952075089102c0";
    #[derive(Debug, PartialEq, Eq)]
    enum Interface {
        Control(Connection),
        Display,
    }
    fn identify(
        vendor: &str,
        product: &str,
        interface: &str,
        descriptor: &str,
    ) -> Option<Interface> {
        match (vendor, product, interface, descriptor) {
            ("0c45", "8009", "03", FEATURE_DESCRIPTOR) => {
                Some(Interface::Control(Connection::Wired))
            }
            ("0c45", "8009", "02", TFT_DESCRIPTOR) => Some(Interface::Display),
            ("0c45", "fdfd", "03", RECEIVER_DESCRIPTOR) => {
                Some(Interface::Control(Connection::Receiver))
            }
            _ => None,
        }
    }
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
            let descriptor = std::fs::read(device.join("report_descriptor"))
                .unwrap_or_default()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let node = PathBuf::from("/dev").join(entry.file_name());
            match identify(
                &read(usb.join("idVendor")),
                &read(usb.join("idProduct")),
                &read(interface.join("bInterfaceNumber")),
                &descriptor,
            ) {
                Some(Interface::Control(connection)) => {
                    features.push((usb.to_path_buf(), node, connection))
                }
                Some(Interface::Display) => displays.push((usb.to_path_buf(), node)),
                None => {}
            }
        }
        features.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        features
            .into_iter()
            .map(|(usb, feature, connection)| Keyboard {
                connection,
                label: format!(
                    "AK820 {} · {} · {}",
                    if connection == Connection::Wired {
                        "wired"
                    } else {
                        "2.4 GHz"
                    },
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
    fn open_node(p: &Path) -> Result<File, String> {
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
    }
    fn open_control(k: &Keyboard) -> Result<File, String> {
        // Recheck identity and descriptor immediately before opening; don't trust stored paths.
        if !discover().iter().any(|d| {
            d.feature == k.feature && d.display == k.display && d.connection == k.connection
        }) {
            return Err(
                "Keyboard disconnected or its USB interfaces changed. Refresh devices.".into(),
            );
        }
        let feature = open_node(&k.feature)?;
        // Advisory process lock is released when the feature fd closes.
        if unsafe { flock(feature.as_raw_fd(), 6) } != 0 {
            return Err("Another Nix820 Pro transfer is using this keyboard.".into());
        }
        Ok(feature)
    }
    pub(super) struct Hid {
        feature: File,
        display: Option<File>,
    }
    impl Hid {
        pub(super) fn open(k: &Keyboard, needs_display: bool) -> Result<Self, String> {
            let feature = open_control(k)?;
            let display = if needs_display {
                Some(open_node(
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
    pub(super) struct Receiver {
        file: File,
    }
    impl Receiver {
        pub(super) fn open(k: &Keyboard) -> Result<Self, String> {
            Ok(Self {
                file: open_control(k)?,
            })
        }
    }
    impl protocol::wireless::Transport for Receiver {
        fn clear_input(&mut self) -> Result<(), String> {
            // Replies have no sequence number. Clear only before a transaction.
            let mut buffer = [0; 33];
            let mut drained = false;
            for _ in 0..32 {
                match self.file.read(&mut buffer) {
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        drained = true;
                        break;
                    }
                    Ok(0) => return Err("Receiver disconnected before transfer.".into()),
                    Ok(_) => {}
                    Err(error) => return Err(format!("Cannot clear receiver input: {error}")),
                }
            }
            if !drained {
                return Err("Receiver input remained busy; no command was sent. Try again.".into());
            }
            Ok(())
        }
        fn wait(&mut self, duration: Duration) {
            std::thread::sleep(duration);
        }
        fn send(&mut self, packet: &protocol::wireless::Packet) -> Result<(), String> {
            // Unnumbered HID output still takes a leading report ID zero in hidraw.
            let mut bytes = [0; 33];
            bytes[1..].copy_from_slice(packet);
            let written = self
                .file
                .write(&bytes)
                .map_err(|error| format!("Receiver write failed: {error}"))?;
            if written != bytes.len() {
                return Err("Short receiver write; keyboard state is unconfirmed.".into());
            }
            Ok(())
        }
        fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>, String> {
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err("Receiver reply timed out; keyboard state is unconfirmed.".into());
                }
                let mut fd = PollFd {
                    fd: self.file.as_raw_fd(),
                    events: 1,
                    revents: 0,
                };
                let milliseconds = remaining
                    .as_millis()
                    .saturating_add(1)
                    .min(c_int::MAX as u128) as c_int;
                let n = unsafe { poll(&mut fd, 1, milliseconds) };
                if n < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(format!("Receiver poll failed: {error}"));
                }
                if n == 0 {
                    continue;
                }
                if fd.revents & (8 | 16 | 32) != 0 {
                    return Err(
                        "Receiver disconnected during transfer; keyboard state is unconfirmed."
                            .into(),
                    );
                }
                if fd.revents & 1 == 0 {
                    continue;
                }
                // One extra byte exposes oversized packets instead of truncating them.
                let mut bytes = vec![0; 33];
                match self.file.read(&mut bytes) {
                    Ok(0) => return Err("Receiver disconnected during transfer.".into()),
                    Ok(n) => {
                        bytes.truncate(n);
                        return Ok(bytes);
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) =>
                    {
                        continue;
                    }
                    Err(error) => return Err(format!("Receiver read failed: {error}")),
                }
            }
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
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::protocol::wireless::Transport as _;
        use std::os::{
            fd::OwnedFd,
            unix::net::{UnixDatagram, UnixStream},
        };

        fn receiver_pair() -> (Receiver, UnixDatagram) {
            let (socket, peer) = UnixDatagram::pair().unwrap();
            socket.set_nonblocking(true).unwrap();
            let fd: OwnedFd = socket.into();
            (
                Receiver {
                    file: File::from(fd),
                },
                peer,
            )
        }
        #[test]
        fn discovery_requires_correct_identity_interface_and_exact_descriptor() {
            assert_eq!(
                identify("0c45", "8009", "03", FEATURE_DESCRIPTOR),
                Some(Interface::Control(Connection::Wired))
            );
            assert_eq!(
                identify("0c45", "8009", "02", TFT_DESCRIPTOR),
                Some(Interface::Display)
            );
            assert_eq!(
                identify("0c45", "fdfd", "03", RECEIVER_DESCRIPTOR),
                Some(Interface::Control(Connection::Receiver))
            );
            for (vid, pid, interface, descriptor) in [
                ("ffff", "fdfd", "03", RECEIVER_DESCRIPTOR),
                ("0c45", "8009", "03", RECEIVER_DESCRIPTOR),
                ("0c45", "fdfd", "04", RECEIVER_DESCRIPTOR),
                ("0c45", "fdfd", "03", FEATURE_DESCRIPTOR),
                ("0c45", "fdfd", "03", ""),
            ] {
                assert_eq!(identify(vid, pid, interface, descriptor), None);
            }
        }
        #[test]
        fn receiver_writes_report_id_zero_and_discards_queued_input() {
            let (mut receiver, peer) = receiver_pair();
            peer.send(&[7; 32]).unwrap();
            let packet = protocol::wireless::lighting(
                &protocol::lighting(0, [255, 0, 0], false, 5., 3.).unwrap(),
            )
            .unwrap();
            receiver.clear_input().unwrap();
            receiver.send(&packet).unwrap();
            let mut bytes = [0; 64];
            let n = peer.recv(&mut bytes).unwrap();
            assert_eq!(n, 33);
            assert_eq!(bytes[0], 0);
            assert_eq!(&bytes[1..33], packet);
            peer.send(&[9; 32]).unwrap();
            // A later query write must not drain the pending reset reply.
            receiver.send(&[0x20; 32]).unwrap();
            assert_eq!(peer.recv(&mut bytes).unwrap(), 33);
            assert_eq!(
                receiver.receive(Duration::from_millis(20)).unwrap(),
                [9; 32]
            );
        }
        #[test]
        fn stale_ack_cannot_satisfy_new_transfer_and_timeout_is_bounded() {
            let (mut receiver, peer) = receiver_pair();
            peer.send(&[0; 32]).unwrap();
            receiver.clear_input().unwrap();
            receiver.send(&[0; 32]).unwrap();
            let start = Instant::now();
            assert!(
                receiver
                    .receive(Duration::from_millis(10))
                    .unwrap_err()
                    .contains("timed out")
            );
            assert!(start.elapsed() < Duration::from_secs(1));
        }
        #[test]
        fn disconnected_receiver_stops_before_write_or_during_reply_wait() {
            let (socket, peer) = UnixStream::pair().unwrap();
            socket.set_nonblocking(true).unwrap();
            let fd: OwnedFd = socket.into();
            let mut receiver = Receiver {
                file: File::from(fd),
            };
            drop(peer);
            assert!(receiver.clear_input().unwrap_err().contains("disconnected"));
            assert!(
                receiver
                    .receive(Duration::from_millis(20))
                    .unwrap_err()
                    .contains("disconnected")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receiver_rejects_keymap_and_display_jobs_before_opening_a_device() {
        let receiver = Keyboard {
            connection: Connection::Receiver,
            feature: "/nonexistent/receiver".into(),
            display: None,
            label: "test receiver".into(),
        };
        for job in [
            Job::Map(Box::new(crate::remap::KeyMap::new(
                crate::remap::Layer::Top,
            ))),
            Job::Map(Box::new(crate::remap::KeyMap::new(crate::remap::Layer::Fn))),
            Job::Tft(vec![]),
            Job::TimeSync([0; 64]),
        ] {
            assert!(job.check_connection(Connection::Wired).is_ok());
            assert!(
                job.run(&receiver)
                    .unwrap_err()
                    .contains("require wired USB")
            );
        }
        for job in [
            Job::Lighting([0; 64]),
            Job::Settings([0; 64]),
            Job::Reset(protocol::ResetKind::Keyboard),
            Job::Reset(protocol::ResetKind::Factory),
        ] {
            assert!(job.check_connection(Connection::Receiver).is_ok());
        }
        assert!(
            Job::Settings([0; 64])
                .run(&receiver)
                .unwrap_err()
                .contains("response level")
        );
    }
    #[test]
    fn receiver_single_key_job_is_validated_before_open_and_never_uses_wired_transport() {
        let receiver = Keyboard {
            connection: Connection::Receiver,
            feature: "/nonexistent/receiver".into(),
            display: None,
            label: "test".into(),
        };
        let invalid = Job::Remap {
            layer: crate::remap::Layer::Fn,
            source: "Esc".into(),
            destination: crate::remap::KeyCode::B,
        };
        assert!(invalid.run(&receiver).unwrap_err().contains("Unsupported"));
        let valid = Job::Remap {
            layer: crate::remap::Layer::Top,
            source: "A".into(),
            destination: crate::remap::KeyCode::Calculator,
        };
        assert!(valid.check_connection(Connection::Receiver).is_ok());
        let wired = Keyboard {
            connection: Connection::Wired,
            ..receiver
        };
        assert!(
            valid
                .run(&wired)
                .unwrap_err()
                .contains("Selected-key transfers use the receiver")
        );
    }
}
