use super::*;
#[test]
fn overlap_and_deletion_points_keep_the_existing_no_neighbor_boundary() {
    for (start, end, expected) in [
        (0, 10, false),
        (9, 11, true),
        (10, 20, true),
        (19, 21, true),
        (20, 30, false),
        (10, 10, false),
        (15, 15, true),
        (20, 20, false),
    ] {
        assert_eq!(
            span_touches_piece(
                &ChangeSpan {
                    byte_start: start,
                    byte_end: end
                },
                10,
                20
            ),
            expected
        );
    }
}

#[test]
fn target_change_identity_normalizes_contained_paths_and_rejects_root_escape() {
    use crate::checked_file_source::contained_target_identity;
    for (raw, expected) in [
        (
            "skills/loader/../target/./SKILL.md",
            Some("skills/target/SKILL.md"),
        ),
        ("skills/target/SKILL.md", Some("skills/target/SKILL.md")),
        ("../target/SKILL.md", None),
        ("skills/../../target/SKILL.md", None),
        ("/target/SKILL.md", None),
    ] {
        assert_eq!(
            contained_target_identity(std::path::Path::new(raw)),
            expected.map(std::path::PathBuf::from)
        );
    }
}
