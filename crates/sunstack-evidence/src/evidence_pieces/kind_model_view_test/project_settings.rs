use super::*;

#[test]
fn independent_projects_keep_their_own_attributes_in_one_process() {
    let settings = serde_json::from_value(json!({"html":{
        "text_attributes":{"custom-diagram":["caption"]}
    }}))
    .unwrap();
    let configured = project_with_settings(&settings);
    let standard = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let definition = serde_json::from_value(json!({
        "rule":"html_kind","fields":{"body":{"from":"first_line"}}
    }))
    .unwrap();
    let raw = "<custom-diagram caption='Flow' alt='Picture'>Body</custom-diagram>";
    let fields = BTreeMap::from([(field_name("body"), text_field(raw))]);

    for (project, expected) in [
        (&standard, "[alt: Picture] Body"),
        (&configured, "[caption: Flow] [alt: Picture] Body"),
        (&standard, "[alt: Picture] Body"),
        (&configured, "[caption: Flow] [alt: Picture] Body"),
    ] {
        let view = serde_json::to_value(project_kind_model_view(
            project,
            &kind,
            &definition,
            &fields,
            1000,
        ))
        .unwrap();
        assert_eq!(view["fields"]["body"], expected);
        assert_eq!(view["cut"], json!([]));
        assert_eq!(fields[&field_name("body")], text_field(raw));
    }
}

#[test]
fn selection_candidates_and_shared_views_use_the_same_project_conventions() {
    let settings = serde_json::from_value(json!({"html":{
        "text_attributes":{"custom-diagram":["caption"]}
    }}))
    .unwrap();
    let configured = project_with_settings(&settings);
    let standard = project();
    let root = tempfile::tempdir().unwrap();
    let raw = "<custom-diagram caption='Flow' alt='Picture'>Body</custom-diagram>";
    std::fs::write(root.path().join("page.html"), raw).unwrap();
    let kinds = serde_json::from_value(json!({"html_kind":{
        "rule":"html_kind","fields":{
            "body":{"from":"remainder"},
            "shared_intro":{"from":"same_file","rule":"html_kind","text":"full"}
        },
        "select":{"question":{"type":"noul","instructions":{"check":"body","context":["shared_intro"]}},
            "yes_at_or_above":0.7,"no_at_or_below":0.3,"shared":["shared_intro"]}
    }})).unwrap();

    for (project, expected) in [
        (&standard, "[alt: Picture] Body"),
        (&configured, "[caption: Flow] [alt: Picture] Body"),
        (&standard, "[alt: Picture] Body"),
    ] {
        let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
        let mut build = crate::EvidenceBuild::start(
            project,
            &kinds,
            &["page.html".into()],
            &mut source,
            crate::PieceBuildLimits {
                max_files: 10,
                max_file_bytes: 10000,
                max_total_bytes: 100000,
                max_matches: 100,
                evidence_max_chars: 6000,
            },
        )
        .unwrap();
        let crate::BuildStep::NeedsSelection {
            candidates, shared, ..
        } = build.advance(crate::ChosenCandidates::default(), &mut source)
        else {
            panic!("selected kind must yield real candidate and shared views");
        };
        assert_eq!(candidates.len(), 1);
        assert_eq!(shared.len(), 1);
        assert_eq!(
            serde_json::to_value(&candidates[0].selection_view).unwrap()["fields"]["body"],
            expected
        );
        assert_eq!(
            serde_json::to_value(&shared[0].view).unwrap()["fields"]["shared_intro"],
            json!([expected])
        );
        assert_eq!(
            candidates[0].fields[&field_name("body")],
            FieldValue::Text {
                value: raw.into(),
                spans: candidates[0].fields[&field_name("body")].spans().to_vec()
            }
        );
    }
}
