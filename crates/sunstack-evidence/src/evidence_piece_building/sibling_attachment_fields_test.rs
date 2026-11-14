#![allow(clippy::unwrap_used)]
use super::*;
use crate::{NodeKey, ParentChild, ParentEntry};
use serde_json::json;

#[test]
fn same_file_kind_uses_its_matcher_node_when_inline_and_paragraph_ranges_are_equal() {
    let text = "Lead U1\n\nTail\n";
    let lead = MatchRange { start: 0, end: 7 };
    let tail = MatchRange { start: 9, end: 14 };
    let child = |node, range| ParentChild {
        node: NodeKey(node),
        range,
        continuation: false,
        heading: false,
    };
    let mut index = FileMatchIndex::default();
    let matcher = crate::EvidenceRuleId::new("leads").unwrap();
    index.by_rule.insert(matcher.clone(), vec![lead.clone()]);
    index.match_nodes.insert(matcher, vec![NodeKey(2)]);
    // Equal byte ranges have distinct parser parents. The lower node ID is inline.
    index.parents.insert(
        NodeKey(1),
        ParentEntry {
            parent: NodeKey(2),
            position: 0,
            children: vec![child(1, lead.clone())],
        },
    );
    index.parents.insert(
        NodeKey(2),
        ParentEntry {
            parent: NodeKey(3),
            position: 0,
            children: vec![child(2, lead.clone()), child(4, tail)],
        },
    );
    let file = MatchedFile {
        text: text.into(),
        index,
        native_units: Default::default(),
        source: SourceRef::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "owner.md".into(),
            content_identity: content_digest(text.as_bytes()),
        },
    };
    let kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> =
        serde_json::from_value(json!({"lead_kind":{"rule":"leads","fields":{}}})).unwrap();
    let source:FieldSource=serde_json::from_value(json!({"from":"same_file","kind":"lead_kind","attach":{"until":"boundary","max_siblings":1}})).unwrap();
    let FieldSource::SameFile {
        attach: Some(attach),
        ..
    } = &source
    else {
        unreachable!()
    };
    let input = FieldValue::Texts {
        cut: false,
        values: vec!["Lead U1".into()],
        spans: vec![source_span(&file, &lead)],
    };

    let actual = attach_field(
        &source,
        input,
        attach,
        &super::super::derived_fields::DerivedFieldsContext {
            kinds: &kinds,
            file: &file,
            pieces: &[],
            own_range: &lead,
            contents: None,
            project: None,
            max_matches: 100,
        },
    );

    assert!(matches!(actual,FieldValue::Texts{values,cut:false,..} if values==[text]));
}
