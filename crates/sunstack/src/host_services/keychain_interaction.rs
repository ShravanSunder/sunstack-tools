//! The process-wide Keychain interaction setting, shared by overlapping credential lookups.
//!
//! macOS keeps one "user interaction allowed" setting per process. A noninteractive
//! credential lookup must turn it off while it reads the store, but the SDK runs many
//! lookups in one host process, and the host may have chosen the setting itself. So the
//! lookups share one hold: the first to enter samples the setting and turns interaction
//! off only if it was on; later lookups join; the last to leave restores what the first
//! one sampled. No lookup can end another's noninteractive policy, and a setting the host
//! had already turned off is left off.
//!
//! The hold coordinates only Sunstack's own lookups through this owner. Another library in
//! the same process that changes the setting at the same time is not coordinated with it.
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The process's Keychain user-interaction setting.
pub(crate) trait InteractionSetting {
    /// Dropping it turns interaction back on.
    type Restore;
    /// Whether Keychain functions may display user interaction now.
    fn allowed(&self) -> Result<bool, ()>;
    /// Turn interaction off until the returned value is dropped.
    fn disable(&self) -> Result<Self::Restore, ()>;
}

/// One noninteractive hold shared by every overlapping lookup in the process.
pub(crate) struct NoninteractiveHold<Setting: InteractionSetting> {
    setting: Setting,
    state: Mutex<HoldState<Setting::Restore>>,
}

struct HoldState<Restore> {
    lookups: usize,
    /// Present only when the first lookup turned interaction off itself.
    restore: Option<Restore>,
}

impl<Setting: InteractionSetting> NoninteractiveHold<Setting> {
    pub(crate) const fn new(setting: Setting) -> Self {
        Self {
            setting,
            state: Mutex::new(HoldState {
                lookups: 0,
                restore: None,
            }),
        }
    }

    /// Run one lookup with Keychain interaction off; `None` when the setting cannot be
    /// read or changed, so the lookup does not run interactively.
    pub(crate) fn run<Output>(&self, lookup: impl FnOnce() -> Output) -> Option<Output> {
        let _lookup = self.enter().ok()?;
        Some(lookup())
    }

    fn enter(&self) -> Result<Lookup<'_, Setting>, ()> {
        let mut state = self.state();
        if state.lookups == 0 && self.setting.allowed()? {
            state.restore = Some(self.setting.disable()?);
        }
        state.lookups += 1;
        Ok(Lookup { hold: self })
    }

    fn state(&self) -> MutexGuard<'_, HoldState<Setting::Restore>> {
        // The counter stays consistent across a panic elsewhere; keep using it.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One active lookup inside the shared hold.
struct Lookup<'hold, Setting: InteractionSetting> {
    hold: &'hold NoninteractiveHold<Setting>,
}

impl<Setting: InteractionSetting> Drop for Lookup<'_, Setting> {
    fn drop(&mut self) {
        let mut state = self.hold.state();
        state.lookups -= 1;
        if state.lookups == 0 {
            // The last lookup out restores what the first one found.
            drop(state.restore.take());
        }
    }
}

/// The real setting, through Security.framework.
#[cfg(target_os = "macos")]
pub(crate) struct KeychainSetting;

#[cfg(target_os = "macos")]
impl InteractionSetting for KeychainSetting {
    type Restore = security_framework::os::macos::keychain::KeychainUserInteractionLock;
    fn allowed(&self) -> Result<bool, ()> {
        security_framework::os::macos::keychain::SecKeychain::user_interaction_allowed()
            .map_err(drop)
    }
    fn disable(&self) -> Result<Self::Restore, ()> {
        security_framework::os::macos::keychain::SecKeychain::disable_user_interaction()
            .map_err(drop)
    }
}

/// The one hold every Sunstack credential lookup in this process shares.
#[cfg(target_os = "macos")]
pub(crate) static KEYCHAIN_NONINTERACTIVE: NoninteractiveHold<KeychainSetting> =
    NoninteractiveHold::new(KeychainSetting);

#[cfg(test)]
#[path = "keychain_interaction_test.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "keychain_interaction_platform_test.rs"]
mod platform;
