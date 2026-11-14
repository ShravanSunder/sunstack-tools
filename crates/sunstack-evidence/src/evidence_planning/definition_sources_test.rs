#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

#[derive(Serialize, Deserialize)]
struct LegacyEdge {
    #[serde(with = "cli_path")]
    rulebook: DefinitionOrigin,
}

#[test]
fn legacy_edges_keep_file_paths_and_refuse_in_memory_origins() {
    let file = LegacyEdge {
        rulebook: DefinitionOrigin::file("/project/rulebook.md"),
    };
    let encoded = serde_json::to_string(&file).unwrap();
    assert_eq!(encoded, r#"{"rulebook":"/project/rulebook.md"}"#);
    let decoded: LegacyEdge = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        decoded.rulebook,
        DefinitionOrigin::file("/project/rulebook.md")
    );

    let inline = LegacyEdge {
        rulebook: DefinitionOrigin::Inline {
            id: "standards".into(),
            label: "standards".into(),
        },
    };
    let refused = serde_json::to_string(&inline).unwrap_err();
    assert!(refused.to_string().contains("no CLI report path"));
}

#[test]
fn origins_have_closed_tagged_shapes() {
    let edited = DefinitionOrigin::Edited {
        id: "config#/rulebooks/0/entries".into(),
        based_on: SourceLocator::File {
            path: "/project/rules.json".into(),
        },
    };
    let value = serde_json::to_value(&edited).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"kind":"edited","id":"config#/rulebooks/0/entries","based_on":{"kind":"file","path":"/project/rules.json"}})
    );
    let unknown = serde_json::from_value::<DefinitionOrigin>(
        serde_json::json!({"kind":"file","path":"/a","extra":1}),
    );
    assert!(unknown.is_err());
}

#[test]
fn definition_reads_classify_missing_and_non_utf8_sources() {
    let directory = tempfile::tempdir().unwrap();
    let missing = read_definition_file(&directory.path().join("absent.md")).unwrap_err();
    assert_eq!(missing.reason, SourceUnavailable::NotFound);

    let binary = directory.path().join("binary.md");
    std::fs::write(&binary, [0xff, 0xfe]).unwrap();
    assert_eq!(
        read_definition_file(&binary).unwrap_err().reason,
        SourceUnavailable::Encoding
    );

    let text = directory.path().join("rules.json");
    std::fs::write(&text, "{}").unwrap();
    let captured = read_definition_file(&text).unwrap();
    assert_eq!(captured.text, "{}");
    let snapshot = DefinitionSnapshot::new(
        DefinitionOrigin::file(&captured.path),
        captured.text,
        SnapshotRepresentation::OriginalText,
        SnapshotAcquisition::Read,
    );
    assert_eq!(
        snapshot.sha256,
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}
