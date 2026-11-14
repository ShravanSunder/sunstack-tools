use super::*;

#[tokio::test]
async fn rules_and_check_bind_each_trailing_marker_to_the_same_standard() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    std::fs::write(root.path().join("rulebook.md"),"One visual per point, next to the text it supports.<!-- K001 --><!-- K002 -->\n\nKeep detail near its point, so nothing becomes a text wall.<!-- K003 --> \t<!-- K004 -->\n").unwrap();
    let mut entries = json!({});
    for marker in ["K001", "K002", "K003", "K004"] {
        entries[marker] = json!({"name":format!("rule_{}",marker.to_lowercase()),"evidence":"skill","decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"description","text":"Use when"},"yes":"pass","no":{"finding":"Describe it."}}}}});
    }
    fixture::entry_file(root.path(), &entries);
    for arguments in [vec!["--rules"], vec!["--check"]] {
        let output = invoke(root.path(), &arguments, 0).await;
        let listed: Value = serde_json::from_slice(&output.stdout).unwrap();
        let listed = listed.as_array().unwrap();
        assert_eq!(listed.len(), 4, "{listed:?}");
        for (row, (marker, line, standard)) in listed.iter().zip([
            (
                "K001",
                1,
                "One visual per point, next to the text it supports.",
            ),
            (
                "K002",
                1,
                "One visual per point, next to the text it supports.",
            ),
            (
                "K003",
                3,
                "Keep detail near its point, so nothing becomes a text wall.",
            ),
            (
                "K004",
                3,
                "Keep detail near its point, so nothing becomes a text wall.",
            ),
        ]) {
            assert_eq!(row["marker"], marker);
            assert_eq!(row["source"]["line"], line);
            assert_eq!(row["standard"], standard);
        }
    }
}
