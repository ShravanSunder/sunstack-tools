use super::*;
use std::time::Duration;

#[test]
fn clocks_start_once_and_stop_once_for_their_own_candidate() {
    // Arrange: independent candidates share a scheduler, but have different send times.
    let mut timings = CandidateTimings::default();
    let origin = Instant::now();
    // Act: a later page must not restart a clock; unrelated draining must not extend it.
    timings.sent(GroupIndex(0), origin);
    timings.sent(GroupIndex(1), origin + Duration::from_millis(10));
    timings.sent(GroupIndex(0), origin + Duration::from_millis(20));
    timings.settled(GroupIndex(0), origin + Duration::from_millis(23));
    timings.settled(GroupIndex(1), origin + Duration::from_millis(81));
    timings.settled(GroupIndex(0), origin + Duration::from_millis(99));
    timings.settled(GroupIndex(2), origin + Duration::from_millis(99));
    // Assert: these are candidate intervals, never total run time or a synthetic quantile.
    assert!(matches!(
        timings.0.get(&GroupIndex(0)),
        Some(CandidateClock::Settled(23))
    ));
    assert!(matches!(
        timings.0.get(&GroupIndex(1)),
        Some(CandidateClock::Settled(71))
    ));
    assert!(!timings.0.contains_key(&GroupIndex(2)));
}
