#![allow(clippy::panic)]
use super::*;

fn heading_build(
    path: &str,
    raw: &str,
    matcher: serde_json::Value,
    selection: bool,
    max_chars: usize,
) -> (
    tempfile::TempDir,
    EvidenceBuild,
    crate::FilesystemCheckedSource,
) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join(path), raw).unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let mut subject = matcher.clone();
    subject["fields"] = json!({
        "heading":{"from":"section","text":"heading"},
        "heading_path":{"from":"enclosing","kind":"outline","text":"heading"},
        "raw_body":{"from":"section","text":"full"}
    });
    if selection {
        subject["select"] = json!({"question":{"type":"noul","instructions":{
            "check":"heading","context":["heading_path"]
        }},"yes_at_or_above":0.7,"no_at_or_below":0.3,"shared":[]});
    }
    let mut outline = matcher;
    outline["fields"] = json!({});
    let kinds = serde_json::from_value(json!({"outline":outline,"subject":subject})).unwrap();
    let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
    let build = EvidenceBuild::start(
        &project,
        &kinds,
        &[path.into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 100000,
            max_total_bytes: 100000,
            max_matches: 100,
            evidence_max_chars: max_chars,
        },
    )
    .unwrap();
    (root, build, source)
}

#[test]
fn heading_html_selection_preserves_decoded_literals_and_local_spans() {
    let raw =
        "猫<h2>Review &lt;T&gt; &amp;lt;</h2><h3>1. <em>Read</em> addressed source</h3><p>Body</p>";
    let (_root, mut build, mut source) = heading_build(
        "page.html",
        raw,
        json!({"html_units":{"mode":"unit","units":["section"]}}),
        true,
        6000,
    );
    let BuildStep::NeedsSelection { candidates, .. } =
        build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!("selection must be reached")
    };
    assert_eq!(candidates.len(), 2);
    for (index, expected, path) in [
        (0, "Review <T> &lt;", json!(["Review <T> &lt;"])),
        (
            1,
            "1. Read addressed source",
            json!(["Review <T> &lt;", "1. Read addressed source"]),
        ),
    ] {
        let candidate = &candidates[index];
        let fields = serde_json::to_value(&candidate.fields).unwrap();
        let view = serde_json::to_value(&candidate.selection_view).unwrap();
        assert_eq!(fields["heading"]["value"], expected);
        assert_eq!(view["fields"]["heading"], expected);
        assert_eq!(fields["heading_path"]["values"], path);
        assert_eq!(view["fields"]["heading_path"], path);
        assert_eq!(view["cut"], json!([]));
        let span = &fields["heading"]["spans"][0];
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let end = span["byte_end"].as_u64().unwrap() as usize;
        assert_eq!(
            &raw[start..end],
            if index == 0 {
                "<h2>Review &lt;T&gt; &amp;lt;</h2>"
            } else {
                "<h3>1. <em>Read</em> addressed source</h3>"
            }
        );
        assert_eq!(
            fields["heading_path"]["spans"].as_array().unwrap().len(),
            index + 1
        );
    }
    let first_view = serde_json::to_value(&candidates[0].selection_view).unwrap();
    assert_eq!(
        first_view["fields"]["raw_body"],
        "Review <T> &lt; 1. Read addressed source Body"
    );
}

#[test]
fn heading_markdown_keeps_inline_source_without_atx_or_setext_markers() {
    let raw = "Root *inline* &amp;\r\n==================\r\n\r\n## Child `code` ###\r\nbody\r\n";
    let (_root, mut build, mut source) = heading_build(
        "page.md",
        raw,
        json!({"markdown_units":{"mode":"unit","units":["section"]}}),
        false,
        6000,
    );
    let BuildStep::Done(catalog) = build.advance(ChosenCandidates::default(), &mut source) else {
        panic!("no selection")
    };
    let pieces = catalog
        .pieces
        .iter()
        .filter(|p| p.kind.as_str() == "subject")
        .collect::<Vec<_>>();
    assert_eq!(pieces.len(), 2);
    for (index, expected) in [(0, "Root *inline* &amp;"), (1, "Child `code`")] {
        let fields = serde_json::to_value(&pieces[index].fields).unwrap();
        let view = serde_json::to_value(&pieces[index].model_view).unwrap();
        assert_eq!(fields["heading"]["value"], expected);
        assert_eq!(view["fields"]["heading"], expected);
        let span = &fields["heading"]["spans"][0];
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let end = span["byte_end"].as_u64().unwrap() as usize;
        assert_eq!(
            &raw[start..end],
            if index == 0 {
                "Root *inline* &amp;\r\n==================\r\n"
            } else {
                "## Child `code` ###\r\n"
            }
        );
    }
}

#[test]
fn heading_shared_view_preserves_decoded_text_without_a_second_parse() {
    let (_root, mut build, mut source) = heading_build(
        "page.html",
        "<h2>&lt;T&gt; &amp;lt;</h2>",
        json!({"html_units":{"mode":"unit","units":["section"]}}),
        true,
        6000,
    );
    let subject = build
        .kinds
        .get_mut(&EvidenceKindName::new("subject").unwrap())
        .unwrap();
    subject.select.as_mut().unwrap().shared = vec![FieldName::new("heading").unwrap()];

    let BuildStep::NeedsSelection { shared, .. } =
        build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!("selection must be reached")
    };

    let raw = serde_json::to_value(&shared[0].fields).unwrap();
    let view = serde_json::to_value(&shared[0].view).unwrap();
    assert_eq!(raw["heading"]["value"], "<T> &lt;");
    assert_eq!(view["fields"]["heading"], "<T> &lt;");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn heading_unbounded_html_is_unavailable_and_empty_bounded_heading_is_text() {
    for (raw, expected) in [
        (
            "<h2>Unclosed<h3>Child</h3>",
            json!({"state":"unavailable","reason":"unreadable"}),
        ),
        ("<h2></h2><h3>Child</h3>", json!("")),
    ] {
        let (_root, mut build, mut source) = heading_build(
            "page.html",
            raw,
            json!({"html_units":{"mode":"unit","units":["section"]}}),
            false,
            6000,
        );
        let BuildStep::Done(catalog) = build.advance(ChosenCandidates::default(), &mut source)
        else {
            panic!("no selection")
        };
        let pieces = catalog
            .pieces
            .iter()
            .filter(|p| p.kind.as_str() == "subject")
            .collect::<Vec<_>>();
        assert_eq!(pieces.len(), 2);
        let fields = serde_json::to_value(&pieces[0].fields).unwrap();
        if expected.is_object() {
            assert_eq!(fields["heading"], expected)
        } else {
            assert_eq!(fields["heading"]["value"], expected)
        }
        assert_eq!(pieces[0].text, raw);
        assert_eq!(
            serde_json::to_value(&pieces[1].fields).unwrap()["heading"]["value"],
            "Child"
        );
    }
}

#[test]
fn heading_empty_context_and_common_budget_cut_preserve_raw_values() {
    let title = "長".repeat(3000);
    let raw = format!("<h2>{title}</h2>");
    let (_root, mut build, mut source) = heading_build(
        "page.html",
        &raw,
        json!({"html_units":{"mode":"unit","units":["section"]}}),
        false,
        1000,
    );
    build
        .kinds
        .get_mut(&EvidenceKindName::new("outline").unwrap())
        .unwrap()
        .matcher = serde_json::from_value::<EvidenceKindDefinition>(json!({
        "html_units":{"mode":"unit","units":["section"],"starts_with":"never"},"fields":{}
    }))
    .unwrap()
    .matcher;
    build = EvidenceBuild::start(
        &build.project,
        &build.kinds,
        &["page.html".into()],
        &mut source,
        build.limits,
    )
    .unwrap();

    let BuildStep::Done(catalog) = build.advance(ChosenCandidates::default(), &mut source) else {
        panic!("no selection")
    };

    let piece = catalog
        .pieces
        .iter()
        .find(|p| p.kind.as_str() == "subject")
        .unwrap();
    let fields = serde_json::to_value(&piece.fields).unwrap();
    let view = serde_json::to_value(&piece.model_view).unwrap();
    assert_eq!(fields["heading"]["value"], title);
    assert_eq!(fields["heading_path"]["values"], json!([]));
    assert_eq!(fields["heading_path"]["spans"], json!([]));
    assert_eq!(view["fields"]["heading_path"], json!([]));
    assert!(
        view["cut"].as_array().unwrap().contains(&json!("heading")),
        "{view}"
    );
    assert!(
        view["fields"]["heading"]
            .as_str()
            .unwrap()
            .contains("[cut]")
    );
}
