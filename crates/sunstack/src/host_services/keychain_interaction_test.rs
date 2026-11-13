#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

/// A stand-in for the process setting. Like the real lock, dropping its restore value
/// turns interaction on unconditionally.
struct FakeSetting {
    allowed: Arc<AtomicBool>,
    available: bool,
    disables: AtomicUsize,
}

struct FakeRestore(Arc<AtomicBool>);

impl Drop for FakeRestore {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl InteractionSetting for FakeSetting {
    type Restore = FakeRestore;
    fn allowed(&self) -> Result<bool, ()> {
        if self.available {
            Ok(self.allowed.load(Ordering::SeqCst))
        } else {
            Err(())
        }
    }
    fn disable(&self) -> Result<Self::Restore, ()> {
        self.disables.fetch_add(1, Ordering::SeqCst);
        self.allowed.store(false, Ordering::SeqCst);
        Ok(FakeRestore(self.allowed.clone()))
    }
}

fn hold(initially_allowed: bool) -> (NoninteractiveHold<FakeSetting>, Arc<AtomicBool>) {
    let allowed = Arc::new(AtomicBool::new(initially_allowed));
    (
        NoninteractiveHold::new(FakeSetting {
            allowed: allowed.clone(),
            available: true,
            disables: AtomicUsize::new(0),
        }),
        allowed,
    )
}

#[test]
fn a_lookup_turns_interaction_off_and_restores_it_when_it_was_on() {
    let (hold, allowed) = hold(true);
    let inside = hold.run(|| allowed.load(Ordering::SeqCst));
    assert_eq!(inside, Some(false));
    assert!(allowed.load(Ordering::SeqCst));
}

#[test]
fn a_lookup_leaves_interaction_the_host_already_turned_off_off() {
    let (hold, allowed) = hold(false);
    let inside = hold.run(|| allowed.load(Ordering::SeqCst));
    assert_eq!(inside, Some(false));
    assert!(
        !allowed.load(Ordering::SeqCst),
        "the host's setting stays off"
    );
    assert_eq!(hold.setting.disables.load(Ordering::SeqCst), 0);
}

/// The longest any handshake waits: a broken lookup or control fails promptly.
const HANDSHAKE: Duration = Duration::from_secs(5);

/// A lookup that reports entering, then holds until released (or the handshake ends).
fn held_lookup(entered: mpsc::Sender<()>, release: mpsc::Receiver<()>) -> impl FnOnce() {
    move || {
        entered.send(()).unwrap();
        // A dropped sender (a failed control) releases the lookup at once.
        let _ = release.recv_timeout(HANDSHAKE);
    }
}

/// Two lookups overlap: the first finishes while the second is still reading. The
/// second must stay noninteractive until it too finishes.
fn overlapping_lookups(initially_allowed: bool) -> [bool; 3] {
    let (hold, allowed) = hold(initially_allowed);
    let hold = &hold;
    std::thread::scope(|scope| {
        // The release senders live in this closure: if an assertion fails, unwinding drops
        // them and frees both lookups before the scope joins them.
        let (first_entered, first_inside) = mpsc::channel();
        let (finish_first, first_release) = mpsc::channel();
        let (second_entered, second_inside) = mpsc::channel();
        let (finish_second, second_release) = mpsc::channel();
        let first = scope.spawn(move || hold.run(held_lookup(first_entered, first_release)));
        first_inside
            .recv_timeout(HANDSHAKE)
            .expect("the first lookup entered");
        let second = scope.spawn(move || hold.run(held_lookup(second_entered, second_release)));
        second_inside
            .recv_timeout(HANDSHAKE)
            .expect("the second lookup entered while the first was inside");
        let both_inside = allowed.load(Ordering::SeqCst);
        finish_first.send(()).unwrap();
        assert!(first.join().unwrap().is_some());
        // The first lookup has left; the second is still inside.
        let second_alone = allowed.load(Ordering::SeqCst);
        finish_second.send(()).unwrap();
        assert!(second.join().unwrap().is_some());
        [both_inside, second_alone, allowed.load(Ordering::SeqCst)]
    })
}

#[test]
fn an_overlapping_lookup_stays_noninteractive_until_the_last_one_leaves() {
    // [both inside, second still inside, after both]
    assert_eq!(overlapping_lookups(true), [false, false, true]);
}

#[test]
fn overlapping_lookups_leave_a_host_setting_that_was_off_off() {
    assert_eq!(overlapping_lookups(false), [false, false, false]);
}

#[test]
fn a_lookup_does_not_run_when_the_setting_cannot_be_read() {
    let allowed = Arc::new(AtomicBool::new(true));
    let hold = NoninteractiveHold::new(FakeSetting {
        allowed: allowed.clone(),
        available: false,
        disables: AtomicUsize::new(0),
    });
    assert_eq!(hold.run(|| panic!("the lookup must not run")), None::<()>);
    assert!(allowed.load(Ordering::SeqCst));
}
