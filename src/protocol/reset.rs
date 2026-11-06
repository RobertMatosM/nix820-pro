//! Exact vendor reset workflows; the extra factory storage area is still unnamed.
use super::*;
use crate::remap::{KeyMap, Layer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetKind {
    Keyboard,
    Factory,
}
impl ResetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Reset Keyboard",
            Self::Factory => "Factory Reset",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Keyboard => "Clear all Top and Fn remaps on the connected keyboard.",
            Self::Factory => {
                "Run the captured factory reset: clear additional keyboard storage and both remap layers, then restore the captured lighting and system settings defaults."
            }
        }
    }
}

struct Step {
    report: Report,
    read_reply: bool,
    settle: Duration,
    stage: &'static str,
}
impl Step {
    fn new(report: Report, read_reply: bool, intervals: u32, stage: &'static str) -> Self {
        Self {
            report,
            read_reply,
            settle: FEATURE_INTERVAL * intervals,
            stage,
        }
    }
}
fn begin(op: u8, count: u8) -> Report {
    let mut report = command(op);
    report[8] = count;
    report
}
fn clear_map(layer: Layer) -> Vec<Step> {
    let stage = match layer {
        Layer::Top => "clear Top mappings",
        Layer::Fn => "clear Fn mappings",
    };
    let mut steps = vec![
        Step::new(command(0x18), true, 1, stage),
        Step::new(begin(layer.command(), 9), true, 1, stage),
    ];
    let map = KeyMap::new(layer);
    let reports = map_reports_for_layer(layer, map.as_bytes()).expect("constant default map");
    for (index, report) in reports.into_iter().enumerate() {
        steps.push(Step::new(report, false, u32::from(index == 8), stage));
    }
    steps.push(Step::new(command(2), true, 1, stage));
    steps.push(Step::new(command(0xf0), true, 1, stage));
    steps
}
fn steps(kind: ResetKind) -> Vec<Step> {
    let mut steps = vec![];
    if kind == ResetKind::Factory {
        // Do not infer an undocumented standalone reset opcode. Reproduce the
        // observed 0419 + eight-block 0415 clear before the known map clears.
        let stage = "clear factory storage";
        steps.push(Step::new(command(0x19), true, 1, stage));
        steps.push(Step::new(begin(0x15, 8), true, 2, stage));
        for index in 0..8 {
            let mut report = [0; 64];
            if index == 7 {
                report[62..].copy_from_slice(&[0xaa, 0x55]);
            }
            steps.push(Step::new(
                report,
                false,
                if index == 7 { 2 } else { 0 },
                stage,
            ));
        }
        steps.push(Step::new(command(2), true, 1, stage));
    }
    steps.extend(clear_map(Layer::Top));
    steps.extend(clear_map(Layer::Fn));
    if kind == ResetKind::Factory {
        let stage = "restore factory lighting";
        steps.extend([
            Step::new(command(0x18), true, 1, stage),
            Step::new(begin(0x13, 1), true, 1, stage),
            Step::new(
                lighting(10, [255, 0, 0], true, 5., 3.).expect("captured factory lighting"),
                false,
                0,
                stage,
            ),
            Step::new(command(2), true, 1, stage),
            // The factory workflow sends f0 without a GET_FEATURE here.
            Step::new(command(0xf0), false, 0, stage),
        ]);
        let stage = "restore factory settings";
        let mut report = begin(0x17, 1);
        report[2] = 1;
        steps.extend([
            Step::new(command(0x18), true, 1, stage),
            Step::new(report, true, 1, stage),
            Step::new(
                settings(2, 2, false, false, false).expect("captured factory settings"),
                false,
                0,
                stage,
            ),
            Step::new(command(2), true, 0, stage),
        ]);
    }
    steps
}

pub fn apply_reset(t: &mut impl Transport, kind: ResetKind) -> Result<(), String> {
    for (index, step) in steps(kind).into_iter().enumerate() {
        let result = if step.read_reply {
            exchange(t, step.report)
        } else {
            t.set_feature(&step.report)
        };
        result.map_err(|error| {
            format!(
                "{} stopped during {} (write {}); the keyboard may be partially reset. {error}",
                kind.label(),
                step.stage,
                index + 1
            )
        })?;
        if !step.settle.is_zero() {
            t.wait(step.settle);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/reset-keyboard-and-factory-reset-fixtures.json"
        ))
        .unwrap()
    }
    struct Captured {
        writes: Vec<Vec<u8>>,
        times: Vec<Duration>,
        clock: Duration,
        replies: VecDeque<Report>,
        reads: usize,
        fail_write: Option<usize>,
        fail_read: Option<usize>,
    }
    impl Captured {
        fn new(tx: &serde_json::Value) -> Self {
            Self {
                writes: vec![],
                times: vec![],
                clock: Duration::ZERO,
                reads: 0,
                replies: tx["replies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| unhex(r["payload"].as_str().unwrap()).try_into().unwrap())
                    .collect(),
                fail_write: None,
                fail_read: None,
            }
        }
    }
    impl Transport for Captured {
        fn wait(&mut self, time: Duration) {
            self.clock += time;
        }
        fn set_feature(&mut self, report: &Report) -> Result<(), String> {
            if self.fail_write == Some(self.writes.len()) {
                return Err("write failure".into());
            }
            self.times.push(self.clock);
            self.writes.push(report.to_vec());
            self.clock += FEATURE_INTERVAL;
            Ok(())
        }
        fn get_feature(&mut self) -> Result<Report, String> {
            let index = self.reads;
            self.reads += 1;
            if self.fail_read == Some(index) {
                return Err("read failure".into());
            }
            self.replies.pop_front().ok_or("unexpected read".into())
        }
        fn output(&mut self, _: &[u8]) -> Result<(), String> {
            panic!("reset uses feature reports")
        }
        fn input(&mut self) -> Result<Vec<u8>, String> {
            panic!("reset has no interrupt reads")
        }
    }

    #[test]
    fn both_resets_match_every_captured_write_actual_reply_and_processing_gap() {
        let fixture = fixture();
        for (index, kind) in [ResetKind::Keyboard, ResetKind::Factory]
            .into_iter()
            .enumerate()
        {
            let tx = &fixture["transactions"][index];
            let mut transport = Captured::new(tx);
            apply_reset(&mut transport, kind).unwrap();
            let writes = tx["writes"].as_array().unwrap();
            assert_eq!(
                transport.writes,
                writes
                    .iter()
                    .map(|r| unhex(r["payload"].as_str().unwrap()))
                    .collect::<Vec<_>>()
            );
            assert!(transport.replies.is_empty());
            assert_eq!(transport.reads, tx["replies"].as_array().unwrap().len());
            for i in 1..writes.len() {
                let observed = Duration::from_secs_f64(
                    writes[i]["time"].as_f64().unwrap() - writes[i - 1]["time"].as_f64().unwrap(),
                );
                let generated = transport.times[i] - transport.times[i - 1];
                assert!(
                    generated.abs_diff(observed) < Duration::from_millis(6),
                    "{kind:?} write {i}: {observed:?} vs {generated:?}"
                );
            }
        }
    }

    #[test]
    fn every_reset_write_read_rejection_and_wrong_reply_stops_without_retry() {
        let fixture = fixture();
        for (index, kind) in [ResetKind::Keyboard, ResetKind::Factory]
            .into_iter()
            .enumerate()
        {
            let tx = &fixture["transactions"][index];
            let writes = tx["writes"].as_array().unwrap();
            for failure in 0..writes.len() {
                let mut t = Captured::new(tx);
                t.fail_write = Some(failure);
                let error = apply_reset(&mut t, kind).unwrap_err();
                assert!(error.contains(kind.label()));
                assert!(error.contains("partially reset"));
                assert_eq!(t.writes.len(), failure);
            }
            let reply_positions: Vec<_> = writes
                .iter()
                .enumerate()
                .filter(|(_, r)| r["read_reply"] == true)
                .map(|(i, _)| i + 1)
                .collect();
            for (failure, expected_count) in reply_positions.into_iter().enumerate() {
                for mode in 0..3 {
                    let mut t = Captured::new(tx);
                    match mode {
                        0 => t.fail_read = Some(failure),
                        1 => t.replies[failure][3] = 0xff,
                        _ => t.replies[failure][1] ^= 1,
                    }
                    assert!(apply_reset(&mut t, kind).is_err());
                    assert_eq!(t.writes.len(), expected_count);
                    assert_eq!(t.reads, failure + 1);
                }
            }
        }
    }
}
