#![allow(clippy::unwrap_used)]
#[path = "support/code_context_fixture.rs"]
mod fixture;
mod support;
use std::collections::BTreeSet;
use sunstack_evidence::*;

// A multiline credential may be masked, but its original LF/CRLF lines still
// determine the unit's source lines and the pager's omitted source range.
#[tokio::test]
async fn credential_projection_preserves_native_source_lines_and_literal_markers() {
    for newline in ["\n", "\r\n"] {
        let (repository, _, _) = fixture::repository(false);
        let text = "pub fn touched() {\n flagged(\"MATCH\");\n let password=\nabcdefghijklmnopqrst;\n let literal = \"[redacted credential]\";\n}\n".replace('\n', newline);
        repository.write("src/lib.rs", &text);
        repository.stage("src/lib.rs");
        let revision = repository.commit(1200).to_string();
        let mut session = fixture::session(&repository).await;
        let index = session
            .build_code_context(&fixture::code(
                &revision,
                vec![QueryPath::Exact {
                    path: "src/lib.rs".into(),
                }],
            ))
            .await
            .unwrap();
        let found = index
            .discover(Some(&fixture::pass("flagged($VALUE)")), 0)
            .unwrap();
        assert_eq!(found.matches.len(), 1);
        let original = &found.matches[0].context;
        let projected = project_code_context(original, &BTreeSet::new()).unwrap();
        let unit = projected
            .context
            .units
            .iter()
            .find(|unit| unit.text.contains("pub fn touched()"))
            .unwrap();
        let before = original
            .units
            .iter()
            .find(|item| item.piece_id == unit.piece_id)
            .unwrap();
        assert_eq!(
            unit.text.lines().count(),
            before.text.lines().count(),
            "credential replacement collapsed a source line for {newline:?}"
        );
        assert_eq!(
            unit.text.matches(newline).count(),
            before.text.matches(newline).count()
        );
        assert_eq!(
            unit.text.matches("[redacted credential]").count(),
            2,
            "one replacement and one preexisting literal"
        );
        assert!(!unit.text.contains("abcdefghijklmnopqrst"));
        let actual = projected
            .redaction_ranges
            .iter()
            .filter(|range| {
                range.part
                    == CodeTextPart::UnitText {
                        piece_id: unit.piece_id.clone(),
                    }
            })
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), 1, "a literal marker is not a replacement");
        let prefix = "pub fn touched() {\n flagged(\"MATCH\");\n let ".replace('\n', newline);
        assert_eq!(
            (actual[0].byte_start, actual[0].byte_end),
            (prefix.len(), prefix.len() + "[redacted credential]".len())
        );
        assert_eq!(
            &unit.text[actual[0].byte_start..actual[0].byte_end],
            "[redacted credential]"
        );
        let granted =
            project_code_context(original, &BTreeSet::from([PrivateKind::CredentialLike])).unwrap();
        assert!(granted.redaction_ranges.is_empty());
        assert_eq!(granted.context.units, original.units);
        let wire = serde_json::to_value(&projected.context).unwrap();
        assert!(
            wire.get("redaction_ranges").is_none(),
            "native range metadata must not cross the public wire"
        );
        session.close();
    }
}
