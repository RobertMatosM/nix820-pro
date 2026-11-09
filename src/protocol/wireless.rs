//! Capture-backed 2.4 GHz receiver messages, separate from wired transactions.
use super::{Report, ResetKind};
use crate::remap::{KeyCode, Layer, source_slot};
use std::time::{Duration, Instant};

pub const REPLY_TIMEOUT: Duration = Duration::from_secs(2);
pub type Packet = [u8; 32];

pub trait Transport {
    /// Clear stale input once before a transaction, never between related writes.
    fn clear_input(&mut self) -> Result<(), String>;
    fn wait(&mut self, duration: Duration);
    fn now(&self) -> Instant {
        Instant::now()
    }
    /// Send one complete interrupt report without discarding pending replies.
    fn send(&mut self, packet: &Packet) -> Result<(), String>;
    fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>, String>;
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))
}

pub fn lighting(report: &Report) -> Result<Packet, String> {
    // Require a canonical encoder output; never silently truncate other commands.
    let mode = match report[0] {
        0 => 19,
        1..=19 => usize::from(report[0] - 1),
        _ => return Err("Invalid receiver lighting mode.".into()),
    };
    let expected = super::lighting(
        mode,
        report[1..4].try_into().unwrap(),
        report[8] != 0,
        f32::from(report[9].max(1)),
        f32::from(report[10].max(1)),
    )?;
    if report != &expected {
        return Err("Invalid receiver lighting payload.".into());
    }
    let mut packet = [0; 32];
    packet[..3].copy_from_slice(&[5, 16, 0]);
    packet[3..19].copy_from_slice(&report[..16]);
    packet[31] = checksum(&packet[..31]);
    Ok(packet)
}

pub fn settings(report: &Report) -> Result<Packet, String> {
    let expected = super::settings(
        usize::from(report[8]),
        usize::from(report[6]),
        report[4] != 0,
        report[3] != 0,
        report[2] != 0,
    )?;
    if report != &expected {
        return Err("Invalid receiver settings payload.".into());
    }
    let mut packet = [0; 32];
    packet[..3].copy_from_slice(&[7, 16, 0]);
    packet[3..17].copy_from_slice(&report[..14]);
    packet[17..19].copy_from_slice(&report[62..]);
    packet[31] = checksum(&packet[..31]);
    Ok(packet)
}

fn reset_packet(kind: ResetKind) -> Packet {
    let mut packet = [0; 32];
    packet[0] = match kind {
        ResetKind::Keyboard => 0x1f,
        ResetKind::Factory => 0x0f,
    };
    packet[31] = packet[0];
    packet
}
fn query_packet() -> Packet {
    let mut packet = [0; 32];
    packet[..3].copy_from_slice(&[0x20, 1, 0]);
    packet[31] = checksum(&packet[..31]);
    packet
}
fn acknowledgement(packet: &Packet) -> Packet {
    let mut expected = [0; 32];
    expected[..3].copy_from_slice(&packet[..3]);
    expected[31] = checksum(&expected[..31]);
    expected
}
fn validate_reply(reply: &[u8]) -> Result<(), String> {
    if reply.len() != 32 {
        return Err("Receiver returned a short or oversized reply; stopped.".into());
    }
    if checksum(&reply[..31]) != reply[31] {
        return Err("Receiver reply checksum is invalid; stopped.".into());
    }
    Ok(())
}
fn check_reply(reply: &[u8], packet: &Packet) -> Result<(), String> {
    validate_reply(reply)?;
    if reply != acknowledgement(packet) {
        return Err(format!(
            "Unexpected receiver reply {:02x?}; keyboard state is unconfirmed.",
            &reply[..4]
        ));
    }
    Ok(())
}
fn exchange(t: &mut impl Transport, packet: &Packet) -> Result<(), String> {
    t.clear_input()?;
    t.send(packet)?;
    check_reply(&t.receive(REPLY_TIMEOUT)?, packet)
}
pub fn apply_lighting(t: &mut impl Transport, report: &Report) -> Result<(), String> {
    exchange(t, &lighting(report)?)
}
pub fn apply_settings(t: &mut impl Transport, report: &Report) -> Result<(), String> {
    exchange(t, &settings(report)?)
}

pub fn remap(layer: Layer, source: &str, destination: KeyCode) -> Result<Packet, String> {
    if !layer.allows(source) {
        return Err(format!(
            "Unsupported {}-layer source key: {source}.",
            layer.display_name()
        ));
    }
    let mut packet = [0; 32];
    packet[0] = match layer {
        Layer::Top => 3,
        Layer::Fn => 0x0d,
    };
    packet[1] = 4;
    packet[2] = u8::try_from(source_slot(source).unwrap())
        .map_err(|_| "Receiver source slot is out of range.")?;
    packet[3..7].copy_from_slice(&destination.record());
    packet[31] = checksum(&packet[..31]);
    Ok(packet)
}

fn query_response() -> Packet {
    let mut reply = query_packet();
    reply[3] = 0xff;
    reply[31] = checksum(&reply[..31]);
    reply
}
pub fn apply_remap(
    t: &mut impl Transport,
    layer: Layer,
    source: &str,
    destination: KeyCode,
) -> Result<(), String> {
    let packet = remap(layer, source, destination)?;
    t.clear_input()?;
    let result = (|| {
        t.send(&packet)?;
        let deadline = t.now() + REPLY_TIMEOUT;
        // Exact previously observed non-remap frames may occur between replies.
        // The 55 fc 04 notification uses a separate format, without this checksum.
        let mut notification = [0; 32];
        notification[..3].copy_from_slice(&[0x55, 0xfc, 4]);
        for _ in 0..8 {
            let remaining = deadline.saturating_duration_since(t.now());
            if remaining.is_zero() {
                return Err("Receiver remap acknowledgement timed out.".into());
            }
            let reply = t.receive(remaining)?;
            if reply == notification || reply == query_response() {
                continue;
            }
            return check_reply(&reply, &packet);
        }
        Err("Too many unrelated receiver messages; no remap acknowledgement received.".into())
    })();
    result
        .map_err(|error| format!("Remap is unconfirmed and may already have taken effect. {error}"))
}
pub fn apply_reset(t: &mut impl Transport, kind: ResetKind) -> Result<(), String> {
    let packet = reset_packet(kind);
    t.clear_input()?;
    let result = (|| {
        t.send(&packet)?;
        if kind == ResetKind::Keyboard {
            // Preserve the two captured queries without draining an early reset ack.
            t.wait(Duration::from_millis(351));
            t.send(&query_packet())?;
            t.wait(Duration::from_millis(921));
            t.send(&query_packet())?;
        }
        // The two queries can each produce their own captured 20 01 00 ff reply.
        // Accept only these or the reset ack, under one deadline and a fixed bound.
        let deadline = t.now() + REPLY_TIMEOUT;
        let query_reply = query_response();
        let attempts = if kind == ResetKind::Keyboard { 3 } else { 1 };
        for _ in 0..attempts {
            let remaining = deadline.saturating_duration_since(t.now());
            if remaining.is_zero() {
                return Err("Receiver reset acknowledgement timed out.".into());
            }
            let reply = t.receive(remaining)?;
            validate_reply(&reply)?;
            if reply == acknowledgement(&packet) {
                return Ok(());
            }
            if kind != ResetKind::Keyboard || reply != query_reply {
                return Err(format!(
                    "Unexpected receiver reset reply {:02x?}.",
                    &reply[..4]
                ));
            }
        }
        Err("Receiver reset acknowledgement was not received.".into())
    })();
    result.map_err(|error| {
        format!(
            "{} is unconfirmed and may already have taken effect. {error}",
            kind.label()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn unhex(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }
    struct Fake {
        writes: Vec<Packet>,
        reply: Result<Vec<u8>, String>,
        write_error: bool,
        reads: usize,
        timeout: Duration,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                writes: vec![],
                reply: Ok(unhex(
                    "0510000000000000000000000000000000000000000000000000000000000015",
                )),
                write_error: false,
                reads: 0,
                timeout: Duration::ZERO,
            }
        }
    }
    impl Transport for Fake {
        fn clear_input(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn wait(&mut self, _: Duration) {}
        fn send(&mut self, packet: &Packet) -> Result<(), String> {
            self.writes.push(*packet);
            if self.write_error {
                Err("write failed".into())
            } else {
                Ok(())
            }
        }
        fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>, String> {
            self.reads += 1;
            self.timeout = timeout;
            self.reply.clone()
        }
    }
    fn report() -> Report {
        super::super::lighting(0, [255, 128, 0], false, 5., 3.).unwrap()
    }

    #[test]
    fn all_47_captured_lighting_pairs_match_encoding_and_exchange() {
        #[derive(Deserialize)]
        struct Fixtures {
            captures: Vec<Capture>,
        }
        #[derive(Deserialize)]
        struct Capture {
            sha256: String,
            transactions: Vec<Transaction>,
        }
        #[derive(Deserialize)]
        struct Transaction {
            write: String,
            reply: String,
            reply_ms: f64,
        }
        let fixtures: Fixtures = serde_json::from_str(include_str!(
            "../../tests/fixtures/wireless-lighting-fixtures.json"
        ))
        .unwrap();
        assert_eq!(
            fixtures
                .captures
                .iter()
                .map(|c| c.transactions.len())
                .collect::<Vec<_>>(),
            [20, 22, 5]
        );
        let mut modes = std::collections::BTreeSet::new();
        let mut speeds = std::collections::BTreeSet::new();
        for capture in fixtures.captures {
            assert_eq!(capture.sha256.len(), 64);
            for tx in capture.transactions {
                let bytes = unhex(&tx.write);
                let mode = if bytes[3] == 0 {
                    19
                } else {
                    usize::from(bytes[3] - 1)
                };
                modes.insert(mode);
                if mode == 1 {
                    speeds.insert(bytes[13]);
                }
                let report = super::super::lighting(
                    mode,
                    bytes[4..7].try_into().unwrap(),
                    bytes[11] != 0,
                    f32::from(bytes[12].max(1)),
                    f32::from(bytes[13].max(1)),
                )
                .unwrap();
                let mut transport = Fake {
                    reply: Ok(unhex(&tx.reply)),
                    ..Default::default()
                };
                apply_lighting(&mut transport, &report).unwrap();
                assert_eq!(transport.writes, vec![bytes.as_slice()]);
                assert_eq!(transport.reads, 1);
                assert!(transport.timeout.as_secs_f64() * 1000. > tx.reply_ms);
            }
        }
        assert_eq!(
            modes.into_iter().collect::<Vec<_>>(),
            (0..20).collect::<Vec<_>>()
        );
        assert_eq!(speeds.into_iter().collect::<Vec<_>>(), [1, 2, 3, 4, 5]);
    }
    #[test]
    fn noncanonical_reports_never_reach_the_transport() {
        for (index, value) in [
            (0, 20),
            (8, 2),
            (9, 0),
            (9, 6),
            (10, 0),
            (10, 6),
            (11, 1),
            (16, 1),
            (63, 1),
            (14, 0),
        ] {
            let mut report = report();
            report[index] = value;
            let mut t = Fake::default();
            assert!(apply_lighting(&mut t, &report).is_err(), "byte {index}");
            assert!(t.writes.is_empty());
            assert_eq!(t.reads, 0);
        }
        let off = super::super::lighting(19, [255; 3], true, 5., 3.).unwrap();
        assert_eq!(&lighting(&off).unwrap()[3..17], &[0; 14]);
        let mut malformed_off = off;
        malformed_off[1] = 1;
        assert!(lighting(&malformed_off).is_err());
    }
    #[test]
    fn malformed_unrelated_or_unknown_status_replies_stop_without_retrying() {
        let good = Fake::default().reply.unwrap();
        let mut checksum_bad = good.clone();
        checksum_bad[31] ^= 1;
        let mut unrelated = good.clone();
        unrelated[0] = 0x20;
        unrelated[31] = checksum(&unrelated[..31]);
        let mut unknown_status = good.clone();
        unknown_status[3] = 0xff;
        unknown_status[31] = checksum(&unknown_status[..31]);
        for reply in [
            vec![],
            good[..31].to_vec(),
            [good.as_slice(), &[0]].concat(),
            checksum_bad,
            unrelated,
            unknown_status,
        ] {
            let mut t = Fake {
                reply: Ok(reply),
                ..Default::default()
            };
            assert!(apply_lighting(&mut t, &report()).is_err());
            assert_eq!(t.writes.len(), 1);
            assert_eq!(t.reads, 1);
        }
    }
    #[test]
    fn write_timeout_and_disconnect_errors_propagate_without_retries() {
        let mut t = Fake {
            write_error: true,
            ..Default::default()
        };
        assert_eq!(
            apply_lighting(&mut t, &report()).unwrap_err(),
            "write failed"
        );
        assert_eq!(t.writes.len(), 1);
        assert_eq!(t.reads, 0);
        for error in ["reply timed out", "receiver disconnected"] {
            let mut t = Fake {
                reply: Err(error.into()),
                ..Default::default()
            };
            assert_eq!(apply_lighting(&mut t, &report()).unwrap_err(), error);
            assert_eq!(t.writes.len(), 1);
            assert_eq!(t.reads, 1);
            assert_eq!(t.timeout, Duration::from_secs(2));
        }
    }
}

#[cfg(test)]
mod settings_reset_tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::VecDeque;

    fn unhex(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }
    #[derive(Default)]
    struct Scripted {
        clock: Duration,
        writes: Vec<(Duration, Packet)>,
        replies: VecDeque<(Duration, Result<Vec<u8>, String>)>,
        clear_count: usize,
        reads: usize,
        fail_write: Option<usize>,
        fail_clear: bool,
    }
    impl Transport for Scripted {
        fn now(&self) -> Instant {
            static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
            *EPOCH.get_or_init(Instant::now) + self.clock
        }
        fn clear_input(&mut self) -> Result<(), String> {
            self.clear_count += 1;
            if self.fail_clear {
                return Err("input busy".into());
            }
            self.replies.retain(|(time, _)| *time > self.clock);
            Ok(())
        }
        fn wait(&mut self, duration: Duration) {
            self.clock += duration;
        }
        fn send(&mut self, packet: &Packet) -> Result<(), String> {
            let index = self.writes.len();
            self.writes.push((self.clock, *packet));
            if self.fail_write == Some(index) {
                return Err("write failed".into());
            }
            Ok(())
        }
        fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>, String> {
            assert!(!timeout.is_zero() && timeout <= REPLY_TIMEOUT);
            self.reads += 1;
            match self.replies.pop_front() {
                Some((at, reply)) if at <= self.clock + timeout => {
                    self.clock = self.clock.max(at);
                    reply
                }
                _ => {
                    self.clock += timeout;
                    Err("reply timed out".into())
                }
            }
        }
    }
    #[derive(Deserialize)]
    struct Fixtures {
        captures: Vec<Capture>,
    }
    #[derive(Deserialize)]
    struct Capture {
        kind: String,
        transactions: Vec<Transaction>,
    }
    #[derive(Deserialize)]
    struct Transaction {
        write: String,
        reply: String,
        reply_ms: f64,
        interleaved_requests: Vec<Query>,
        level: Option<usize>,
        sleep: Option<usize>,
        disable_win: Option<bool>,
        disable_alt_f4: Option<bool>,
        disable_alt_tab: Option<bool>,
    }
    #[derive(Deserialize)]
    struct Query {
        write: String,
        after_ms: f64,
    }

    #[test]
    fn all_18_settings_reset_actions_and_interleaved_queries_match_captures() {
        let fixtures: Fixtures = serde_json::from_str(include_str!(
            "../../tests/fixtures/wireless-settings-review.json"
        ))
        .unwrap();
        let mut count = 0;
        for capture in fixtures.captures {
            for tx in capture.transactions {
                let mut t = Scripted::default();
                t.replies.push_back((
                    Duration::from_secs_f64(tx.reply_ms / 1000.),
                    Ok(unhex(&tx.reply)),
                ));
                if let Some(level) = tx.level {
                    let report = super::super::settings(
                        level,
                        tx.sleep.unwrap(),
                        tx.disable_win.unwrap(),
                        tx.disable_alt_f4.unwrap(),
                        tx.disable_alt_tab.unwrap(),
                    )
                    .unwrap();
                    apply_settings(&mut t, &report).unwrap();
                } else {
                    let kind = if capture.kind == "reset-keyboard" {
                        ResetKind::Keyboard
                    } else {
                        ResetKind::Factory
                    };
                    apply_reset(&mut t, kind).unwrap();
                }
                assert_eq!(t.clear_count, 1);
                assert_eq!(t.reads, 1);
                assert_eq!(t.writes.len(), 1 + tx.interleaved_requests.len());
                assert_eq!(t.writes[0].1.as_slice(), unhex(&tx.write));
                for (actual, query) in t.writes[1..].iter().zip(tx.interleaved_requests) {
                    assert_eq!(actual.1.as_slice(), unhex(&query.write));
                    // Captured USB writes themselves take ~1 ms; round observed gaps.
                    assert!((actual.0.as_secs_f64() * 1000. - query.after_ms).abs() < 3.);
                }
                assert!(t.clock.as_secs_f64() * 1000. >= tx.reply_ms);
                count += 1;
            }
        }
        assert_eq!(count, 18);
    }
    #[test]
    fn invalid_settings_never_clear_or_write_the_transport() {
        for (index, value) in [
            (0, 1),
            (1, 0),
            (2, 2),
            (3, 2),
            (4, 2),
            (6, 4),
            (8, 0),
            (8, 6),
            (14, 1),
            (61, 1),
            (62, 0),
        ] {
            let mut report = super::super::settings(2, 1, false, false, false).unwrap();
            report[index] = value;
            let mut t = Scripted::default();
            assert!(apply_settings(&mut t, &report).is_err(), "byte {index}");
            assert_eq!(t.clear_count, 0);
            assert!(t.writes.is_empty());
        }
    }
    #[test]
    fn early_reset_ack_survives_queries_and_query_replies_are_not_reset_success() {
        let ack = acknowledgement(&reset_packet(ResetKind::Keyboard)).to_vec();
        let mut query_reply = query_packet();
        query_reply[3] = 0xff;
        query_reply[31] = checksum(&query_reply[..31]);
        for replies in [
            vec![(Duration::from_millis(200), Ok(ack.clone()))],
            vec![
                (Duration::from_millis(360), Ok(query_reply.to_vec())),
                (Duration::from_millis(1280), Ok(query_reply.to_vec())),
                (Duration::from_millis(1336), Ok(ack.clone())),
            ],
        ] {
            let mut t = Scripted {
                replies: replies.into(),
                ..Default::default()
            };
            apply_reset(&mut t, ResetKind::Keyboard).unwrap();
            assert_eq!(t.clear_count, 1);
            assert_eq!(t.writes.len(), 3);
            assert_eq!(t.clock.max(Duration::from_millis(1272)), t.clock);
        }
        let mut t = Scripted {
            replies: vec![(Duration::from_millis(360), Ok(query_reply.to_vec()))].into(),
            ..Default::default()
        };
        let error = apply_reset(&mut t, ResetKind::Keyboard).unwrap_err();
        assert!(error.contains("unconfirmed") && error.contains("timed out"));
        assert_eq!(t.writes.len(), 3);
        assert_eq!(t.clear_count, 1);
    }
    #[test]
    fn reset_write_failures_stop_at_each_stage_without_resending_reset() {
        for kind in [ResetKind::Keyboard, ResetKind::Factory] {
            let stages = if kind == ResetKind::Keyboard { 3 } else { 1 };
            for index in 0..stages {
                let mut t = Scripted {
                    fail_write: Some(index),
                    ..Default::default()
                };
                let error = apply_reset(&mut t, kind).unwrap_err();
                assert!(error.contains("unconfirmed") && error.contains("write failed"));
                assert_eq!(t.writes.len(), index + 1);
                assert_eq!(t.reads, 0);
            }
            let mut t = Scripted {
                fail_clear: true,
                ..Default::default()
            };
            assert_eq!(apply_reset(&mut t, kind).unwrap_err(), "input busy");
            assert!(t.writes.is_empty());
        }
    }
    #[test]
    fn query_responses_do_not_extend_the_reset_deadline() {
        let mut reply = query_packet();
        reply[3] = 0xff;
        reply[31] = checksum(&reply[..31]);
        let mut t = Scripted {
            replies: vec![
                (Duration::from_millis(2772), Ok(reply.to_vec())),
                (Duration::from_millis(3772), Ok(reply.to_vec())),
            ]
            .into(),
            ..Default::default()
        };
        assert!(
            apply_reset(&mut t, ResetKind::Keyboard)
                .unwrap_err()
                .contains("timed out")
        );
        assert_eq!(t.clock, Duration::from_millis(3272));
        assert_eq!(t.writes.len(), 3);
    }
    #[test]
    fn settings_and_resets_reject_malformed_or_wrong_command_replies() {
        for kind in [None, Some(ResetKind::Keyboard), Some(ResetKind::Factory)] {
            let expected = acknowledgement(&kind.map(reset_packet).unwrap_or_else(|| {
                settings(&super::super::settings(2, 1, false, false, false).unwrap()).unwrap()
            }));
            let mut bad_sum = expected;
            bad_sum[31] ^= 1;
            let mut bad_status = expected;
            bad_status[3] = 0xff;
            bad_status[31] = checksum(&bad_status[..31]);
            let wrong = acknowledgement(
                &lighting(&super::super::lighting(0, [255, 0, 0], false, 5., 3.).unwrap()).unwrap(),
            );
            for reply in [
                expected[..31].to_vec(),
                [expected.as_slice(), &[0]].concat(),
                bad_sum.to_vec(),
                bad_status.to_vec(),
                wrong.to_vec(),
            ] {
                let mut t = Scripted {
                    replies: vec![(Duration::from_millis(20), Ok(reply))].into(),
                    ..Default::default()
                };
                if let Some(kind) = kind {
                    assert!(
                        apply_reset(&mut t, kind)
                            .unwrap_err()
                            .contains("unconfirmed")
                    );
                } else {
                    assert!(
                        apply_settings(
                            &mut t,
                            &super::super::settings(2, 1, false, false, false).unwrap()
                        )
                        .is_err()
                    );
                }
                assert_eq!(t.reads, 1);
            }
        }
    }
    #[test]
    fn settings_failures_and_reset_disconnects_are_propagated_without_retries() {
        let report = super::super::settings(2, 1, false, false, false).unwrap();
        for error in ["reply timed out", "receiver disconnected"] {
            let mut t = Scripted {
                replies: vec![(Duration::from_millis(20), Err(error.into()))].into(),
                ..Default::default()
            };
            assert_eq!(apply_settings(&mut t, &report).unwrap_err(), error);
            assert_eq!(t.writes.len(), 1);
            let mut t = Scripted {
                replies: vec![(Duration::from_millis(20), Err(error.into()))].into(),
                ..Default::default()
            };
            assert!(
                apply_reset(&mut t, ResetKind::Keyboard)
                    .unwrap_err()
                    .contains(error)
            );
            assert_eq!(t.writes.len(), 3);
        }
        let mut t = Scripted {
            fail_write: Some(0),
            ..Default::default()
        };
        assert!(apply_settings(&mut t, &report).is_err());
        assert_eq!(t.writes.len(), 1);
        assert_eq!(t.reads, 0);
    }
    #[test]
    fn all_ten_captured_remap_writes_and_six_actual_replies_are_preserved() {
        #[derive(Deserialize)]
        struct Evidence {
            captures: Vec<RemapCapture>,
        }
        #[derive(Deserialize)]
        struct RemapCapture {
            layer: String,
            source: String,
            transactions: Vec<RemapTransaction>,
        }
        #[derive(Deserialize)]
        struct RemapTransaction {
            write: String,
            record: String,
            reply: Option<String>,
            reply_ms: Option<f64>,
        }
        let mut writes = 0;
        let mut acknowledgements = 0;
        for json in [
            include_str!("../../tests/fixtures/wireless-remap-review.json"),
            include_str!("../../tests/fixtures/wireless-remap-followup-review.json"),
        ] {
            let evidence: Evidence = serde_json::from_str(json).unwrap();
            for capture in evidence.captures {
                let layer = if capture.layer == "Top" {
                    Layer::Top
                } else {
                    Layer::Fn
                };
                for tx in capture.transactions {
                    let destination = KeyCode::from_record(&unhex(&tx.record)).unwrap();
                    assert_eq!(
                        remap(layer, &capture.source, destination)
                            .unwrap()
                            .as_slice(),
                        unhex(&tx.write)
                    );
                    let mut t = Scripted::default();
                    if let Some(reply) = tx.reply {
                        t.replies.push_back((
                            Duration::from_secs_f64(tx.reply_ms.unwrap() / 1000.),
                            Ok(unhex(&reply)),
                        ));
                        apply_remap(&mut t, layer, &capture.source, destination).unwrap();
                        acknowledgements += 1;
                    } else {
                        assert!(
                            apply_remap(&mut t, layer, &capture.source, destination)
                                .unwrap_err()
                                .contains("unconfirmed")
                        );
                    }
                    assert_eq!(t.clear_count, 1);
                    assert_eq!(t.writes.len(), 1);
                    assert_eq!(t.writes[0].1.as_slice(), unhex(&tx.write));
                    writes += 1;
                }
            }
        }
        assert_eq!(writes, 10);
        assert_eq!(acknowledgements, 6);
    }
    #[test]
    fn invalid_and_reserved_remap_sources_never_touch_the_transport() {
        for (layer, source) in [
            (Layer::Top, "Fn"),
            (Layer::Top, "unknown"),
            (Layer::Fn, "Esc"),
            (Layer::Fn, "Back"),
            (Layer::Fn, "Home"),
            (Layer::Fn, "Win"),
            (Layer::Fn, "Fn"),
            (Layer::Fn, "\\ |"),
            (Layer::Fn, "unknown"),
        ] {
            let mut t = Scripted::default();
            assert!(apply_remap(&mut t, layer, source, KeyCode::A).is_err());
            assert!(t.writes.is_empty());
            assert_eq!(t.clear_count, 0);
        }
    }
    #[test]
    fn remap_waits_for_its_own_reply_across_known_background_frames() {
        let packet = remap(Layer::Fn, "I", KeyCode::B).unwrap();
        let notification = unhex(&format!("55fc04{}", "00".repeat(29)));
        let mut t = Scripted {
            replies: vec![
                (Duration::from_millis(20), Ok(query_response().to_vec())),
                (Duration::from_millis(100), Ok(notification)),
                (
                    Duration::from_micros(238164),
                    Ok(acknowledgement(&packet).to_vec()),
                ),
            ]
            .into(),
            ..Default::default()
        };
        apply_remap(&mut t, Layer::Fn, "I", KeyCode::B).unwrap();
        assert_eq!(t.clock, Duration::from_micros(238164));
        assert_eq!(t.reads, 3);
        assert_eq!(t.writes.len(), 1);
        assert_eq!(t.clear_count, 1);
    }
    #[test]
    fn background_frames_neither_confirm_remap_nor_extend_its_deadline() {
        let mut t = Scripted {
            replies: vec![
                (Duration::from_secs(1), Ok(query_response().to_vec())),
                (Duration::from_millis(2100), Ok(query_response().to_vec())),
            ]
            .into(),
            ..Default::default()
        };
        assert!(
            apply_remap(&mut t, Layer::Top, "A", KeyCode::B)
                .unwrap_err()
                .contains("unconfirmed")
        );
        assert_eq!(t.clock, REPLY_TIMEOUT);
        assert_eq!(t.writes.len(), 1);
        let mut t = Scripted {
            replies: (1..=9)
                .map(|n| (Duration::from_millis(n * 10), Ok(query_response().to_vec())))
                .collect(),
            ..Default::default()
        };
        assert!(
            apply_remap(&mut t, Layer::Top, "A", KeyCode::B)
                .unwrap_err()
                .contains("Too many unrelated")
        );
        assert_eq!(t.reads, 8);
        assert_eq!(t.writes.len(), 1);
    }
    #[test]
    fn remap_rejects_wrong_layer_source_status_checksum_and_length() {
        let good = acknowledgement(&remap(Layer::Top, "A", KeyCode::B).unwrap());
        let mut bad_sum = good;
        bad_sum[31] ^= 1;
        let mut wrong_layer = good;
        wrong_layer[0] = 0x0d;
        wrong_layer[31] = checksum(&wrong_layer[..31]);
        let mut wrong_source = good;
        wrong_source[2] = 45;
        wrong_source[31] = checksum(&wrong_source[..31]);
        let mut bad_status = good;
        bad_status[3] = 0xff;
        bad_status[31] = checksum(&bad_status[..31]);
        for reply in [
            good[..31].to_vec(),
            [good.as_slice(), &[0]].concat(),
            bad_sum.to_vec(),
            wrong_layer.to_vec(),
            wrong_source.to_vec(),
            bad_status.to_vec(),
        ] {
            let mut t = Scripted {
                replies: vec![(Duration::from_millis(20), Ok(reply))].into(),
                ..Default::default()
            };
            assert!(
                apply_remap(&mut t, Layer::Top, "A", KeyCode::B)
                    .unwrap_err()
                    .contains("unconfirmed")
            );
            assert_eq!(t.reads, 1);
            assert_eq!(t.writes.len(), 1);
        }
    }
    #[test]
    fn remap_write_read_and_drain_failures_do_not_retry() {
        let mut t = Scripted {
            fail_clear: true,
            ..Default::default()
        };
        assert_eq!(
            apply_remap(&mut t, Layer::Top, "A", KeyCode::B).unwrap_err(),
            "input busy"
        );
        assert!(t.writes.is_empty());
        let mut t = Scripted {
            fail_write: Some(0),
            ..Default::default()
        };
        assert!(
            apply_remap(&mut t, Layer::Top, "A", KeyCode::B)
                .unwrap_err()
                .contains("write failed")
        );
        assert_eq!(t.writes.len(), 1);
        assert_eq!(t.reads, 0);
        let mut t = Scripted {
            replies: vec![(Duration::from_millis(20), Err("disconnected".into()))].into(),
            ..Default::default()
        };
        assert!(
            apply_remap(&mut t, Layer::Top, "A", KeyCode::B)
                .unwrap_err()
                .contains("disconnected")
        );
        assert_eq!(t.writes.len(), 1);
        assert_eq!(t.reads, 1);
    }
}
