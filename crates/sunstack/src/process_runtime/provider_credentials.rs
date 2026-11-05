use keyring_core::{Entry, api::CredentialStoreApi};
use std::{
    io::{self, Write},
    process::ExitCode,
    sync::Arc,
};

const CREDENTIAL_SERVICE: &str = "sunstack";
const CREDENTIAL_ACCOUNT: &str = "openrouter-api-key";
pub(crate) const MISSING_CREDENTIAL: &str = "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save with the variable set. This build may need keychain approval: run from a terminal to approve store access.";

fn native_entry() -> keyring_core::Result<Arc<Entry>> {
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(target_os = "linux")]
    let store = zbus_secret_service_keyring_store::Store::new()?;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    return store
        .build(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT, None)
        .map(Arc::new);
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Err(keyring_core::Error::NoDefaultStore)
}

// Only this binary accesses Keychain, once per command. Scope its process-wide
// UI setting to the blocking store access, leaving interactive terminals alone.
fn with_native_access<Output>(operation: impl FnOnce() -> Output) -> Option<Output> {
    #[cfg(target_os = "macos")]
    let _interaction_guard = {
        use std::io::IsTerminal;
        if io::stdin().is_terminal() {
            None
        } else {
            Some(
                security_framework::os::macos::keychain::SecKeychain::disable_user_interaction()
                    .ok()?,
            )
        }
    };
    Some(operation())
}

fn nonempty_key(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}
fn store_enabled() -> bool {
    std::env::var_os("SUNSTACK_CREDENTIAL_STORE").is_none_or(|value| value != "off")
}
fn resolve_with(
    environment: Option<String>,
    store_enabled: bool,
    entry: impl FnOnce() -> keyring_core::Result<Arc<Entry>>,
) -> Option<String> {
    nonempty_key(environment).or_else(|| {
        if !store_enabled {
            return None;
        }
        nonempty_key(entry().and_then(|entry| entry.get_password()).ok())
    })
}

pub(crate) async fn resolve() -> Option<String> {
    if let Some(api_key) = nonempty_key(std::env::var("OPENROUTER_API_KEY").ok()) {
        return Some(api_key);
    }
    if !store_enabled() {
        return None;
    }
    tokio::task::spawn_blocking(|| {
        with_native_access(|| resolve_with(None, true, native_entry)).flatten()
    })
    .await
    .ok()
    .flatten()
}

#[derive(Debug, PartialEq, Eq)]
enum CredentialSaveError {
    MissingEnvironment,
    StoreDisabled,
    StoreUnavailable,
}
impl CredentialSaveError {
    fn message(&self) -> &'static str {
        match self {
            Self::MissingEnvironment => {
                "OPENROUTER_API_KEY is missing or empty. Set it before running sunstack credentials save."
            }
            Self::StoreDisabled => {
                "The OS credential store is disabled by SUNSTACK_CREDENTIAL_STORE=off. Unset it to save the credential."
            }
            Self::StoreUnavailable => {
                "The OS credential store is unavailable or needs approval. Run this from a terminal to approve access, then retry sunstack credentials save."
            }
        }
    }
}
fn save_with(
    environment: Option<String>,
    store_enabled: bool,
    entry: impl FnOnce() -> keyring_core::Result<Arc<Entry>>,
) -> Result<(), CredentialSaveError> {
    let api_key = nonempty_key(environment).ok_or(CredentialSaveError::MissingEnvironment)?;
    if !store_enabled {
        return Err(CredentialSaveError::StoreDisabled);
    }
    entry()
        .and_then(|entry| entry.set_password(&api_key))
        .map_err(|_| CredentialSaveError::StoreUnavailable)
}

async fn save_environment() -> Result<(), CredentialSaveError> {
    let environment = nonempty_key(std::env::var("OPENROUTER_API_KEY").ok())
        .ok_or(CredentialSaveError::MissingEnvironment)?;
    if !store_enabled() {
        return Err(CredentialSaveError::StoreDisabled);
    }
    tokio::task::spawn_blocking(move || {
        with_native_access(|| save_with(Some(environment), true, native_entry))
            .unwrap_or(Err(CredentialSaveError::StoreUnavailable))
    })
    .await
    .unwrap_or(Err(CredentialSaveError::StoreUnavailable))
}

pub(crate) async fn save_command() -> ExitCode {
    match save_environment().await {
        Ok(()) => {
            #[cfg(target_os = "macos")]
            let destination = "macOS login Keychain";
            #[cfg(target_os = "linux")]
            let destination = "Linux Secret Service";
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let destination = "OS credential store";
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "Saved OpenRouter credential to {destination}.")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                return ExitCode::from(3);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "sunstack: {}", error.message());
            ExitCode::from(if error == CredentialSaveError::StoreUnavailable {
                3
            } else {
                2
            })
        }
    }
}

#[cfg(test)]
#[path = "provider_credentials_test.rs"]
mod tests;
