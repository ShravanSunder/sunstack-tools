//! Opt-in proof of the shared hold against the real macOS Keychain interaction setting.
//!
//! Both tests are ignored: a default `cargo test` or `nextest` run never selects them. The
//! parent additionally refuses to run, as a failure rather than a pass, unless
//! `SUNSTACK_REAL_KEYCHAIN_POLICY=1` is set. It then runs each control in its own child
//! process, one at a time: this test binary started again on the child test alone, with
//! stdin closed so it is not a terminal. Each child writes its one report record to a file
//! the parent names, apart from the test harness's own output. A child changes only the
//! process-local
//! interaction setting through the production `KeychainSetting` and
//! `KEYCHAIN_NONINTERACTIVE` hold, with lookups that only wait on channels. It opens no
//! keychain and reads no credential.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

/// The opt-in a real-policy run requires in the parent's environment.
const OPT_IN: &str = "SUNSTACK_REAL_KEYCHAIN_POLICY";
/// The control a child runs; set only by the parent.
const CHILD_CASE: &str = "SUNSTACK_KEYCHAIN_POLICY_CASE";
/// The file a child writes its one report record to; named only by the parent.
const CHILD_REPORT: &str = "SUNSTACK_KEYCHAIN_POLICY_REPORT";
/// The child test's full name, as the parent selects it.
const CHILD_TEST: &str = "host_services::keychain_interaction::platform::keychain_policy_child";
/// The longest a child may run before the parent ends it.
const CHILD_DEADLINE: Duration = Duration::from_secs(30);
/// The most output kept from one child.
const OUTPUT_LIMIT: u64 = 64 * 1024;
/// The longest any in-child handshake waits.
const HANDSHAKE: Duration = Duration::from_secs(5);

/// The four controls, by synthetic id.
const CASES: [&str; 4] = [
    "enabled_single",
    "predisabled_single",
    "enabled_overlap",
    "predisabled_overlap",
];

#[test]
#[ignore = "changes the real macOS Keychain interaction setting; run only under an explicit grant"]
fn keychain_policy_controls_in_isolated_children() {
    assert_eq!(
        std::env::var(OPT_IN).as_deref(),
        Ok("1"),
        "real-policy controls run only when {OPT_IN}=1 is set"
    );
    let binary = std::env::current_exe().unwrap();
    let results = CASES
        .iter()
        .map(|case| run_child(&binary, case))
        .collect::<Vec<_>>();
    for result in &results {
        println!("{}", result.line);
    }
    assert!(
        results.iter().all(|result| result.passed),
        "every control must pass; unavailable or failed controls leave the proof incomplete"
    );
}

/// One child's classified result; `line` is for display only.
struct ChildResult {
    passed: bool,
    line: String,
}

/// Run one control in its own child and classify it from its exit and report file.
fn run_child(binary: &std::path::Path, case: &str) -> ChildResult {
    let directory = tempfile::tempdir().unwrap();
    let report_path = directory.path().join("report");
    let mut child = Command::new(binary)
        .args([
            CHILD_TEST,
            "--exact",
            "--ignored",
            "--no-capture",
            "--test-threads=1",
        ])
        .env(CHILD_CASE, case)
        .env(CHILD_REPORT, &report_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let started = Instant::now();
    let exited = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status.success());
        }
        if started.elapsed() > CHILD_DEADLINE {
            // Only this child is ended, and it is always reaped.
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let _stdout = stdout.join().unwrap();
    let _stderr = stderr.join().unwrap();
    let record = std::fs::read_to_string(&report_path).ok();
    classify_child(case, exited, record.as_deref())
}

/// Whether one child passed: it exited successfully (`exited` is `None` on timeout) and its
/// report file holds exactly one record, for this case, with outcome `passed`.
fn classify_child(case: &str, exited: Option<bool>, record: Option<&str>) -> ChildResult {
    let field = |record: &str, key: &str| {
        record
            .split_whitespace()
            .find_map(|part| part.strip_prefix(key)?.strip_prefix('='))
            .map(str::to_string)
    };
    let records = record.map(|record| record.lines().collect::<Vec<_>>());
    let (passed, detail) = match (exited, records.as_deref()) {
        (None, _) => (false, "timeout".to_string()),
        (Some(false), _) => (false, "unsuccessful-exit".to_string()),
        (Some(true), None) => (false, "no-report".to_string()),
        (Some(true), Some([record])) => {
            let reported_case = field(record, "case");
            let outcome = field(record, "outcome");
            (
                reported_case.as_deref() == Some(case) && outcome.as_deref() == Some("passed"),
                format!(
                    "reported case={} outcome={}",
                    reported_case.unwrap_or_default(),
                    outcome.unwrap_or_default()
                ),
            )
        }
        (Some(true), Some(records)) => (false, format!("records={}", records.len())),
    };
    ChildResult {
        passed,
        line: format!(
            "case={case} passed={passed} {detail} record=[{}]",
            record.unwrap_or_default().trim()
        ),
    }
}

/// Read a child's output stream to its end, keeping at most `OUTPUT_LIMIT` bytes.
fn drain(stream: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut kept = String::new();
        let mut limited = stream.take(OUTPUT_LIMIT);
        let _ = limited.read_to_string(&mut kept);
        // Keep reading past the limit so the child never blocks on a full pipe.
        let _ = std::io::copy(&mut limited.into_inner(), &mut std::io::sink());
        kept
    })
}

/// One control's result: the synthetic case, the original setting and what was observed.
struct ControlReport {
    case: String,
    original: Option<bool>,
    observed: Vec<bool>,
    outcome: &'static str,
    detail: &'static str,
}

impl ControlReport {
    /// The one-line record the parent classifies.
    fn record(&self) -> String {
        format!(
            "case={} original={} observed={:?} outcome={} detail={}",
            self.case,
            self.original
                .map_or_else(|| "unread".to_string(), |value| value.to_string()),
            self.observed,
            self.outcome,
            self.detail
        )
        .replace(", ", ",")
    }
}

#[test]
#[ignore = "child of keychain_policy_controls_in_isolated_children; never run directly"]
fn keychain_policy_child() {
    let case = std::env::var(CHILD_CASE)
        .unwrap_or_else(|_| panic!("only the opt-in parent starts this child"));
    let path = std::env::var_os(CHILD_REPORT)
        .unwrap_or_else(|| panic!("only the opt-in parent starts this child"));
    let report = run_control(&case);
    // Written once, to a file only this child creates.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    std::io::Write::write_all(&mut file, format!("{}\n", report.record()).as_bytes()).unwrap();
    assert_ne!(report.outcome, "failed", "{}", report.detail);
}

/// The setting now, or `None` when Security cannot report it.
fn current() -> Option<bool> {
    KeychainSetting.allowed().ok()
}

fn run_control(case: &str) -> ControlReport {
    let mut report = ControlReport {
        case: case.to_string(),
        original: None,
        observed: Vec::new(),
        outcome: "failed",
        detail: "",
    };
    // Read the original setting before anything changes it; a read error changes nothing.
    let Some(original) = current() else {
        report.detail = "original-read-error";
        return report;
    };
    report.original = Some(original);
    let enabled = case.starts_with("enabled");
    if enabled && !original {
        // An enabled control needs interaction on; it is never forced on.
        report.outcome = "unavailable";
        report.detail = "original-false";
        return report;
    }
    // A pre-disabled control turns interaction off first, only if it was on.
    let setup = if !enabled && original {
        match KeychainSetting.disable() {
            Ok(guard) => Some(guard),
            Err(()) => {
                report.detail = "setup-disable-error";
                return report;
            }
        }
    } else {
        None
    };
    let observed = if case.ends_with("overlap") {
        overlapping_hold()
    } else {
        single_hold()
    };
    let Some(observed) = observed else {
        drop(setup);
        report.detail = "hold-or-handshake-failed";
        return report;
    };
    report.observed = observed.clone();
    let while_set_up = current();
    drop(setup);
    let finally = current();
    // Inside the hold interaction is off. Leaving it restores on only when it was on before
    // the hold; a pre-disabled setting stays off until the outer setup guard ends.
    let after_hold = *observed.last().unwrap();
    let expected_after_hold = enabled;
    report.outcome = if observed[..observed.len() - 1].iter().all(|state| !state)
        && after_hold == expected_after_hold
        && while_set_up == Some(expected_after_hold)
        && finally == Some(original)
    {
        "passed"
    } else {
        "failed"
    };
    report.detail = if report.outcome == "passed" {
        "ok"
    } else {
        "unexpected-state"
    };
    report
}

/// One lookup through the production hold: [inside, after].
fn single_hold() -> Option<Vec<bool>> {
    let inside = KEYCHAIN_NONINTERACTIVE.run(current)??;
    Some(vec![inside, current()?])
}

/// Two overlapping lookups through the production hold: [both inside, second alone, after].
fn overlapping_hold() -> Option<Vec<bool>> {
    overlapping_with(&KEYCHAIN_NONINTERACTIVE, current, HANDSHAKE)
}

/// Two overlapping lookups through `hold`, reading the setting with `read`. `None` when a
/// handshake, a lookup's wait for release or a read fails: a lookup that timed out or was
/// disconnected is a failed control, not a completed one.
fn overlapping_with<Setting>(
    hold: &NoninteractiveHold<Setting>,
    read: impl Fn() -> Option<bool> + Sync,
    handshake: Duration,
) -> Option<Vec<bool>>
where
    Setting: InteractionSetting + Sync,
    Setting::Restore: Send,
{
    std::thread::scope(|scope| {
        // The release senders live in this closure: an early return drops them and frees
        // both lookups before the scope joins them.
        let (first_entered, first_inside) = mpsc::channel();
        let (finish_first, first_release) = mpsc::channel::<()>();
        let (second_entered, second_inside) = mpsc::channel();
        let (finish_second, second_release) = mpsc::channel::<()>();
        let first = scope.spawn(move || {
            hold.run(move || {
                let _ = first_entered.send(());
                first_release.recv_timeout(handshake)
            })
        });
        first_inside.recv_timeout(handshake).ok()?;
        let second = scope.spawn(move || {
            hold.run(move || {
                let _ = second_entered.send(());
                second_release.recv_timeout(handshake)
            })
        });
        second_inside.recv_timeout(handshake).ok()?;
        let both_inside = read()?;
        finish_first.send(()).ok()?;
        first.join().ok()??.ok()?;
        let second_alone = read()?;
        finish_second.send(()).ok()?;
        second.join().ok()??.ok()?;
        Some(vec![both_inside, second_alone, read()?])
    })
}

/// A stand-in setting for the collector's own controls; no Security call.
struct FakeSetting(std::sync::Arc<std::sync::atomic::AtomicBool>);

struct FakeRestore(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for FakeRestore {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl InteractionSetting for FakeSetting {
    type Restore = FakeRestore;
    fn allowed(&self) -> Result<bool, ()> {
        Ok(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
    fn disable(&self) -> Result<Self::Restore, ()> {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(FakeRestore(self.0.clone()))
    }
}

#[test]
fn the_collector_requires_a_successful_exit_and_one_passed_record_for_its_case() {
    let passed = "case=enabled_single original=true observed=[false,true] outcome=passed detail=ok";
    let with_newline = format!("{passed}\n");
    let two_records = format!("{passed}\n{passed}\n");
    let control = |exited: Option<bool>, record: Option<&str>| {
        classify_child("enabled_single", exited, record).passed
    };
    assert!(control(Some(true), Some(passed)));
    assert!(control(Some(true), Some(&with_newline)));
    for (name, exited, record) in [
        ("timeout", None, Some(passed)),
        ("unsuccessful exit", Some(false), Some(passed)),
        ("no report", Some(true), None),
        ("empty report", Some(true), Some("")),
        ("two records", Some(true), Some(two_records.as_str())),
        (
            "another case",
            Some(true),
            Some(
                "case=enabled_overlap original=true observed=[false,false,true] outcome=passed detail=ok",
            ),
        ),
        (
            "unavailable",
            Some(true),
            Some(
                "case=enabled_single original=false observed=[] outcome=unavailable detail=original-false",
            ),
        ),
        (
            "failed",
            Some(true),
            Some(
                "case=enabled_single original=true observed=[false,false] outcome=failed detail=unexpected-state",
            ),
        ),
    ] {
        assert!(!control(exited, record), "{name}");
    }
}

#[test]
fn an_overlapping_lookup_whose_release_wait_times_out_fails_the_control() {
    let allowed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let hold = NoninteractiveHold::new(FakeSetting(allowed.clone()));
    let read = || Some(allowed.load(std::sync::atomic::Ordering::SeqCst));
    // Control: the same overlap completes when every release arrives in time.
    assert_eq!(
        overlapping_with(&hold, read, HANDSHAKE),
        Some(vec![false, false, true])
    );
    // A read slower than the lookups' release wait lets the first lookup time out; the
    // control must fail rather than report the states it saw.
    let slow_read = || {
        std::thread::sleep(Duration::from_millis(200));
        read()
    };
    assert_eq!(
        overlapping_with(&hold, slow_read, Duration::from_millis(20)),
        None
    );
    assert!(
        allowed.load(std::sync::atomic::Ordering::SeqCst),
        "the hold restored the setting"
    );
}
