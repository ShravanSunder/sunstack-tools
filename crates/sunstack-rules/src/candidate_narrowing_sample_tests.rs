use super::select_evenly_spaced_sample_indices;

#[test]
fn selects_the_independent_r10_indices_for_fifty_drops() {
    assert_eq!(
        select_evenly_spaced_sample_indices(50, 20),
        vec![
            0, 2, 5, 7, 10, 12, 15, 17, 20, 22, 25, 27, 30, 32, 35, 37, 40, 42, 45, 47,
        ]
    );
}

#[test]
fn smaller_and_zero_limits_keep_the_existing_floor_policy() {
    assert_eq!(select_evenly_spaced_sample_indices(50, 3), vec![0, 16, 33]);
    assert!(select_evenly_spaced_sample_indices(50, 0).is_empty());
    assert_eq!(select_evenly_spaced_sample_indices(4, 20), vec![0, 1, 2, 3]);
}
