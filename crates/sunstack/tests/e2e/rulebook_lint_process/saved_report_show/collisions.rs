use super::*;

fn numeric_case_report() -> Value {
    let mut report = completed_cases();
    report["cases"][0]["id"] = json!("27");
    report["cases"][0]["case"]["id"] = json!("27");
    report
}

#[tokio::test]
async fn completed_numeric_case_id_cannot_hide_another_revision_result() {
    // Arrange: case "27" owns one result 27, while another revision has result 27.
    let root = tempfile::tempdir().unwrap();
    save(&root.path().join("saved.json"), &numeric_case_report());

    // Act: the selector belongs to both namespaces in this completed report.
    let output = show_saved(root.path(), "27").await;

    // Assert: collapse the case with its own result, then refuse both candidates.
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    for candidate in [
        "revision 89dc638e3fe6a865cb7c4a00af9c3c4440549259, case id 27, result id 27",
        "revision 9f7b2e283e95dc849f4e271a5287e3ae21959ca9, no case id, result id 27",
    ] {
        assert!(
            message.contains(candidate),
            "missing {candidate}: {message}"
        );
    }
    assert!(message.contains("ambiguous"), "{message}");
    assert_eq!(message.matches("revision ").count(), 2, "{message}");
}

#[tokio::test]
async fn completed_numeric_case_id_and_its_only_result_are_one_candidate() {
    // Arrange: only the numeric case's own revision has result 27.
    let root = tempfile::tempdir().unwrap();
    let mut report = numeric_case_report();
    report["runs"].as_array_mut().unwrap().truncate(1);
    save(&root.path().join("saved.json"), &report);

    // Act: both case and result lookup identify the same saved detail.
    let detail = successful_detail(&show_saved(root.path(), "27").await);

    // Assert: keep the existing case envelope and its exact linked result.
    assert_eq!(detail["case"]["id"], "27");
    assert_eq!(detail["revision"], report["runs"][0]["revision"]);
    assert_eq!(
        detail["detail"]["result"],
        report["runs"][0]["report"]["results"][0]
    );
}

#[tokio::test]
async fn completed_numeric_case_with_different_result_names_all_three_candidates() {
    // Arrange: the numeric case links result 68, alongside two independent results 27.
    let root = tempfile::tempdir().unwrap();
    let mut report = numeric_case_report();
    report["cases"][0]["result_id"] = json!(68);
    save(&root.path().join("saved.json"), &report);

    // Act: numeric case lookup must not suppress unrelated result lookup.
    let output = show_saved(root.path(), "27").await;

    // Assert: all three complete associations appear, without an invented case ID.
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    for candidate in [
        "revision 89dc638e3fe6a865cb7c4a00af9c3c4440549259, case id 27, result id 68",
        "revision 89dc638e3fe6a865cb7c4a00af9c3c4440549259, no case id, result id 27",
        "revision 9f7b2e283e95dc849f4e271a5287e3ae21959ca9, no case id, result id 27",
    ] {
        assert!(
            message.contains(candidate),
            "missing {candidate}: {message}"
        );
    }
    assert_eq!(message.matches("revision ").count(), 3, "{message}");
}

#[tokio::test]
async fn completed_numeric_case_does_not_collapse_duplicate_result_rows() {
    // Arrange: a numeric case points to a revision with two distinct saved rows sharing its ID.
    let root = tempfile::tempdir().unwrap();
    let mut report = numeric_case_report();
    report["runs"].as_array_mut().unwrap().truncate(1);
    let mut duplicate = report["runs"][0]["report"]["results"][0].clone();
    duplicate["outcome"] = json!("finding");
    report["runs"][0]["report"]["results"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    save(&root.path().join("saved.json"), &report);

    // Act: the same case cannot erase a second matching result row.
    let output = show_saved(root.path(), "27").await;

    // Assert: refuse rather than selecting either outcome silently.
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("ambiguous"), "{message}");
    assert_eq!(
        message
            .matches("revision 89dc638e3fe6a865cb7c4a00af9c3c4440549259, case id 27, result id 27")
            .count(),
        2,
        "{message}"
    );
}
