#![allow(clippy::unwrap_used)]
use super::*;
use crate::configuration_loading::definition_bundle::acquire_file_definitions;
use sunstack_evidence::DefinitionOrigin;

/// A captured file definition with an endpoint the backend refuses while parsing it.
fn refused_file_bundle(root: &std::path::Path) -> DefinitionBundle {
    crate::test_support::lint_fixture::write(root);
    let location = discover_config(root, None).unwrap();
    let mut bundle = acquire_file_definitions(&load_config(&location).unwrap(), &location);
    bundle.settings.decision_model.endpoint = Some("not a url".into());
    bundle
}

/// Both backend constructors refuse with the root definition's own CLI path: the real
/// file for a file origin whether or not its text was captured, and none for an
/// in-memory origin. No origin is ever given an invented file.
#[test]
fn backend_refusals_name_the_root_definition_origin() {
    let root = tempfile::tempdir().unwrap();
    let with_text = refused_file_bundle(root.path());
    let file = with_text.root.origin.file_path().unwrap().to_path_buf();
    assert!(with_text.root.text.is_some());
    let mut without_text = refused_file_bundle(root.path());
    without_text.root.text = None;
    let mut inline = refused_file_bundle(root.path());
    inline.root.text = None;
    inline.root.origin = DefinitionOrigin::Inline {
        id: "inline".into(),
        label: "inline".into(),
    };

    let mut observed = vec![];
    for (name, bundle) in [
        ("file with text", &with_text),
        ("file without text", &without_text),
        ("inline", &inline),
    ] {
        let Err(keyed) = prepare_backend(bundle, "placeholder-never-sent") else {
            panic!("{name}: the endpoint is refused");
        };
        let Err(keyless) = prepare_encoding_backend(bundle) else {
            panic!("{name}: the endpoint is refused without a key");
        };
        for refusal in [keyed, keyless] {
            observed.push((
                name,
                refusal.file.clone(),
                refusal.definition.as_deref() == Some(&bundle.root.origin),
            ));
            assert_eq!(refusal.code, LintInvalidCode::Syntax, "{name}");
        }
    }
    let expected = [
        ("file with text", file.clone()),
        ("file with text", file.clone()),
        ("file without text", file.clone()),
        ("file without text", file.clone()),
        ("inline", std::path::PathBuf::new()),
        ("inline", std::path::PathBuf::new()),
    ]
    .map(|(name, path)| (name, path, true));
    assert_eq!(observed, expected);
}
