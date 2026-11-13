//! Opt-in proof of the shared hold against the real macOS Keychain interaction setting.
//!
//! Both tests are ignored: a default `cargo test` or `nextest` run never selects them. The
//! parent additionally refuses to run, as a failure rather than a pass, unless
//! `SUNSTACK_REAL_KEYCHAIN_POLICY=1` is set. It then runs each control in its own child
//! process, one at a time: this test binary started again on the child test alone, with
//! stdin closed so it is not a terminal. A child changes only the process-local
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
/// The child test's full name, as the parent selects it.
const CHILD_TEST: &str = "host_services::keychain_interaction::platform::keychain_policy_child";
/// The longest a child may run before the parent ends it.
const CHILD_DEADLINE: Duration = Duration::from_secs(30);
/// The most output kept from one child.
const OUTPUT_LIMIT: u64 = 64 * 1024;
/// The longest any in-child handshake waits.
const HANDSHAKE: Duration = Duration::from_secs(5);
/// The line a child reports its control on.
const REPORT_PREFIX: &str = "sunstack-keychain-policy:";

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
    let reports = CASES
        .iter()
        .map(|case| run_child(&binary, case))
        .collect::<Vec<_>>();
    for report in &reports {
        println!("{report}");
    }
    assert!(
        reports
            .iter()
            .all(|report| report.contains(" outcome=passed")),
        "every control must pass; unavailable or failed controls leave the proof incomplete"
    );
}

/// Run one control in its own child and return its report line, or the failure seen.
fn run_child(binary: &std::path::Path, case: &str) -> String {
    let mut child = Command::new(binary)
        .args([
            CHILD_TEST,
            "--exact",
            "--ignored",
            "--no-capture",
            "--test-threads=1",
        ])
        .env(CHILD_CASE, case)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if started.elapsed() > CHILD_DEADLINE {
            // Only this child is ended, and it is always reaped.
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = stdout.join().unwrap();
    let _stderr = stderr.join().unwrap();
    let report = stdout
        .lines()
        .find_map(|line| line.strip_prefix(REPORT_PREFIX))
        .map(str::trim)
        .map(str::to_string);
    match (status, report) {
        (None, _) => format!("case={case} outcome=failed detail=timeout"),
        (Some(status), Some(report)) if status.success() => report,
        // A child that did not exit successfully failed, whatever it reported.
        (Some(status), Some(report)) => format!(
            "case={case} outcome=failed detail=nonzero-exit exit={} reported=[{}]",
            status.code().unwrap_or(-1),
            report.replace("outcome=", "child-outcome=")
        ),
        (Some(status), None) => format!(
            "case={case} outcome=failed detail=no-report exit={}",
            status.code().unwrap_or(-1)
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
    fn print(&self) {
        println!(
            "{REPORT_PREFIX} case={} original={} observed={:?} outcome={} detail={}",
            self.case,
            self.original
                .map_or_else(|| "unread".to_string(), |value| value.to_string()),
            self.observed,
            self.outcome,
            self.detail
        );
    }
}

#[test]
#[ignore = "child of keychain_policy_controls_in_isolated_children; never run directly"]
fn keychain_policy_child() {
    let case = std::env::var(CHILD_CASE)
        .unwrap_or_else(|_| panic!("only the opt-in parent starts this child"));
    let report = run_control(&case);
    report.print();
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
    std::thread::scope(|scope| {
        // The release senders live in this closure: an early return drops them and frees
        // both lookups before the scope joins them.
        let (first_entered, first_inside) = mpsc::channel();
        let (finish_first, first_release) = mpsc::channel::<()>();
        let (second_entered, second_inside) = mpsc::channel();
        let (finish_second, second_release) = mpsc::channel::<()>();
        let first = scope.spawn(move || {
            KEYCHAIN_NONINTERACTIVE.run(move || {
                let _ = first_entered.send(());
                let _ = first_release.recv_timeout(HANDSHAKE);
            })
        });
        first_inside.recv_timeout(HANDSHAKE).ok()?;
        let second = scope.spawn(move || {
            KEYCHAIN_NONINTERACTIVE.run(move || {
                let _ = second_entered.send(());
                let _ = second_release.recv_timeout(HANDSHAKE);
            })
        });
        second_inside.recv_timeout(HANDSHAKE).ok()?;
        let both_inside = current()?;
        finish_first.send(()).ok()?;
        first.join().ok()??;
        let second_alone = current()?;
        finish_second.send(()).ok()?;
        second.join().ok()??;
        Some(vec![both_inside, second_alone, current()?])
    })
}
