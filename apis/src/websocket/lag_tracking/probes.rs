use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const PROBE_TTL: Duration = Duration::from_secs(10);
// The reader handles one thing at a time, so a pong that arrives while it is busy is read late. A
// probe that waited on more than this much reader work says more about us, or about a client
// keeping the reader busy on purpose, than about the network.
const BUSY_TOLERANCE: Duration = Duration::from_millis(5);

// Browsers answer protocol pings in their network stack, out of reach of page code, so a page
// cannot hold its answer back to look slower than it is.
#[derive(Debug, Default)]
pub struct Probes {
    outstanding: HashMap<u64, Probe>,
}

#[derive(Debug)]
struct Probe {
    sent: Instant,
    busy: Duration,
}

impl Probes {
    pub fn sent(&mut self, nonce: u64, now: Instant) {
        self.outstanding
            .retain(|_, probe| now.saturating_duration_since(probe.sent) < PROBE_TTL);
        self.outstanding.insert(
            nonce,
            Probe {
                sent: now,
                busy: Duration::ZERO,
            },
        );
    }

    pub fn round_trip(&mut self, payload: &[u8], now: Instant) -> Option<Duration> {
        let nonce = u64::from_be_bytes(payload.try_into().ok()?);
        let probe = self.outstanding.remove(&nonce)?;
        let rtt = now.saturating_duration_since(probe.sent);
        (rtt < PROBE_TTL && probe.busy <= BUSY_TOLERANCE).then_some(rtt)
    }

    pub fn reader_stalled(&mut self, took: Duration) {
        for probe in self.outstanding.values_mut() {
            probe.busy += took;
        }
    }

    #[cfg(test)]
    fn in_flight(&self) -> usize {
        self.outstanding.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_is_timed_from_the_matching_send() {
        let start = Instant::now();
        let mut probes = Probes::default();
        probes.sent(1, start);
        probes.sent(2, start + Duration::from_millis(1000));

        assert_eq!(
            probes.round_trip(&2u64.to_be_bytes(), start + Duration::from_millis(1040)),
            Some(Duration::from_millis(40))
        );
        assert_eq!(
            probes.round_trip(&1u64.to_be_bytes(), start + Duration::from_millis(1100)),
            Some(Duration::from_millis(1100))
        );
    }

    #[test]
    fn unknown_and_duplicate_payloads_are_ignored() {
        let start = Instant::now();
        let mut probes = Probes::default();
        probes.sent(7, start);
        let later = start + Duration::from_millis(30);

        assert_eq!(probes.round_trip(b"hi", later), None);
        assert_eq!(probes.round_trip(&8u64.to_be_bytes(), later), None);
        assert_eq!(probes.round_trip(&[], later), None);
        assert!(probes.round_trip(&7u64.to_be_bytes(), later).is_some());
        assert_eq!(probes.round_trip(&7u64.to_be_bytes(), later), None);
    }

    #[test]
    fn stale_probes_are_evicted_and_never_answered() {
        let start = Instant::now();
        let mut probes = Probes::default();
        probes.sent(0, start);
        for second in 1..=PROBE_TTL.as_secs() {
            probes.sent(second, start + Duration::from_secs(second));
        }

        assert_eq!(probes.in_flight(), PROBE_TTL.as_secs() as usize);
        assert_eq!(
            probes.round_trip(&0u64.to_be_bytes(), start + PROBE_TTL),
            None
        );
    }

    #[test]
    fn a_pong_slower_than_the_ttl_is_not_a_sample() {
        let start = Instant::now();
        let mut probes = Probes::default();
        probes.sent(7, start);

        assert_eq!(
            probes.round_trip(&7u64.to_be_bytes(), start + PROBE_TTL),
            None
        );
    }

    #[test]
    fn a_pong_read_behind_a_busy_reader_is_not_a_sample() {
        let start = Instant::now();
        let later = start + Duration::from_millis(30);
        let mut probes = Probes::default();
        probes.sent(7, start);
        probes.sent(8, start);
        probes.reader_stalled(Duration::from_millis(1));

        assert!(probes.round_trip(&7u64.to_be_bytes(), later).is_some());
        for _ in 0..BUSY_TOLERANCE.as_millis() {
            probes.reader_stalled(Duration::from_millis(1));
        }
        assert_eq!(probes.round_trip(&8u64.to_be_bytes(), later), None);
    }

    #[test]
    fn only_probes_in_flight_are_charged_for_reader_work() {
        let start = Instant::now();
        let mut probes = Probes::default();
        probes.reader_stalled(Duration::from_secs(1));
        probes.sent(7, start);

        assert!(probes
            .round_trip(&7u64.to_be_bytes(), start + Duration::from_millis(30))
            .is_some());
    }
}
