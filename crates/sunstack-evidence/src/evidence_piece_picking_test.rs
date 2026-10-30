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
