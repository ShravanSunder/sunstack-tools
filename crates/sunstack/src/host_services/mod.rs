//! Effectful host services shared by the CLI and SDK adapters.
#[cfg(any(target_os = "macos", test))]
pub(crate) mod keychain_interaction;
pub mod provider_credentials;
