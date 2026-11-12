use super::super::contains_whole_token;

#[test]
fn overlapping_whole_tokens_are_checked_after_a_rejected_occurrence() {
    // Arrange: earlier occurrences have invalid boundaries; later ones may overlap.
    for (text, needle, expected) in [
        ("XA-A A-A", "A-A", true),
        ("XA-A-A", "A-A", true),
        ("XA-A-AX", "A-A", false),
        ("X猫-猫-猫", "猫-猫", true),
        ("X猫-猫-猫Z", "猫-猫", false),
    ] {
        // Act: use the same token predicate as the contains_field walker.
        let matched = contains_whole_token(text, needle);

        // Assert: every occurrence participates, with unchanged Unicode boundaries.
        assert_eq!(matched, expected, "text={text:?}, needle={needle:?}");
    }
}
