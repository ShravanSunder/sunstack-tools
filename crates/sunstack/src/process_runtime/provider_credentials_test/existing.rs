#![allow(clippy::unwrap_used, clippy::panic)]
use super::super::*;

fn mock_entry() -> Arc<Entry> {
    Arc::new(
        keyring_core::mock::Store::new()
            .unwrap()
            .build("test", "openrouter", None)
            .unwrap(),
    )
}

#[test]
fn environment_wins_without_opening_the_store() {
    for enabled in [true, false] {
        assert_eq!(
            resolve_with(Some("environment-key".into()), enabled, || panic!(
                "environment must not touch store"
            )),
            Some("environment-key".into())
        );
    }
}

#[test]
fn absent_or_blank_environment_uses_the_store_without_writing() {
    let entry = mock_entry();
    entry.set_password("stored-key").unwrap();
    for environment in [None, Some(String::new()), Some(" \t".into())] {
        assert_eq!(
            resolve_with(environment, true, || Ok(entry.clone())),
            Some("stored-key".into())
        );
        assert_eq!(entry.get_password().unwrap(), "stored-key");
    }
}

#[test]
fn disabled_store_never_opens_an_entry() {
    assert!(resolve_with(None, false, || panic!("disabled read")).is_none());
    assert_eq!(
        save_with(Some("one-off-key".into()), false, || panic!(
            "disabled save"
        )),
        Err(CredentialSaveError::StoreDisabled)
    );
}

#[test]
fn missing_empty_or_unavailable_store_has_guidance_for_both_sources() {
    for entry in [
        Err(keyring_core::Error::NoEntry),
        Err(keyring_core::Error::NoDefaultStore),
        Ok(mock_entry()),
    ] {
        assert!(resolve_with(None, true, || entry).is_none());
    }
    let entry = mock_entry();
    entry.set_password(" \t").unwrap();
    assert!(resolve_with(None, true, || Ok(entry)).is_none());
    assert!(MISSING_CREDENTIAL.contains("OPENROUTER_API_KEY"));
    assert!(MISSING_CREDENTIAL.contains("sunstack credentials save"));
}

#[test]
fn explicit_save_writes_the_environment_value_and_can_replace_it() {
    let entry = mock_entry();
    for value in ["first-synthetic-key", "replacement-synthetic-key"] {
        assert_eq!(
            save_with(Some(value.into()), true, || Ok(entry.clone())),
            Ok(())
        );
        assert_eq!(entry.get_password().unwrap(), value);
    }
}

#[test]
fn explicit_save_requires_a_nonempty_environment_key_before_opening_store() {
    for environment in [None, Some(String::new()), Some(" \t".into())] {
        assert_eq!(
            save_with(environment, true, || panic!(
                "missing environment must not touch store"
            )),
            Err(CredentialSaveError::MissingEnvironment)
        );
    }
}

#[test]
fn explicit_save_reports_store_failure_without_returning_secret_errors() {
    let error = save_with(Some("synthetic-save-key".into()), true, || {
        Err(keyring_core::Error::NoDefaultStore)
    })
    .unwrap_err();
    assert_eq!(error, CredentialSaveError::StoreUnavailable);
    assert!(error.message().contains("Run this from a terminal"));
    assert!(!error.message().contains("synthetic-save-key"));
    let entry = mock_entry();
    entry
        .as_any()
        .downcast_ref::<keyring_core::mock::Cred>()
        .unwrap()
        .set_error(keyring_core::Error::NoDefaultStore);
    assert_eq!(
        save_with(Some("synthetic-save-key".into()), true, || Ok(entry)),
        Err(CredentialSaveError::StoreUnavailable)
    );
}
