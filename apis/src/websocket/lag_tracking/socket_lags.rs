use actix_web::http::header::HeaderMap;
use dashmap::DashMap;
use db_lib::game_command::MAX_MOVE_COMPENSATION_SECS;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use uuid::Uuid;

const WINDOW: usize = 10;
// A client can get its probes disqualified (see `Probes`), so without an age limit it could
// freeze an old, worse estimate in place.
const SAMPLE_TTL: Duration = Duration::from_secs(30);
// The kernel RTT is sampled once, at connect. The multiplier and slack leave room for jitter and
// for the path drifting (Wi-Fi to cellular) until the client reconnects. A forged header can
// only loosen the cap up to the ceiling, which is what a missing header gets anyway.
const CAP_RTT_MULTIPLIER: f64 = 1.5;
const CAP_SLACK_MS: f64 = 50.0;
const CEILING_MS: f64 = MAX_MOVE_COMPENSATION_SECS * 1000.0;
const TCP_RTT_HEADER: &str = "x-tcp-rtt";

#[derive(Debug, Default)]
pub struct SocketLags {
    sockets: DashMap<Uuid, SocketLag>,
}

#[derive(Debug)]
struct SocketLag {
    rtts_ms: VecDeque<(Instant, f64)>,
    tcp_rtt: Option<Duration>,
}

// Round trips, not one-way delays: a mover's clock starts when the previous move is committed,
// so it pays for the position travelling out and for the move travelling back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProtocolLag {
    pub estimate_ms: Option<f64>,
    pub cap_ms: f64,
}

impl ProtocolLag {
    pub fn compensable_ms(&self) -> f64 {
        self.estimate_ms
            .map_or(0.0, |estimate| estimate.min(self.cap_ms))
    }

    pub fn capped(&self) -> bool {
        self.estimate_ms
            .is_some_and(|estimate| estimate > self.cap_ms)
    }
}

impl SocketLags {
    pub fn connect(&self, socket_id: Uuid, tcp_rtt: Option<Duration>) {
        self.sockets.insert(
            socket_id,
            SocketLag {
                rtts_ms: VecDeque::with_capacity(WINDOW),
                tcp_rtt,
            },
        );
    }

    pub fn record_round_trip(&self, socket_id: Uuid, rtt: Duration) {
        if let Some(mut lag) = self.sockets.get_mut(&socket_id) {
            if lag.rtts_ms.len() == WINDOW {
                lag.rtts_ms.pop_front();
            }
            lag.rtts_ms.push_back((Instant::now(), rtt_ms(rtt)));
        }
    }

    pub fn lag(&self, socket_id: Uuid) -> ProtocolLag {
        self.lag_at(socket_id, Instant::now())
    }

    fn lag_at(&self, socket_id: Uuid, now: Instant) -> ProtocolLag {
        let (estimate_ms, tcp_rtt) = self.sockets.get(&socket_id).map_or((None, None), |lag| {
            let fresh = lag
                .rtts_ms
                .iter()
                .filter(|(measured, _)| now.saturating_duration_since(*measured) < SAMPLE_TTL)
                .map(|(_, rtt)| *rtt);
            (lower_median(fresh), lag.tcp_rtt)
        });
        ProtocolLag {
            estimate_ms,
            cap_ms: cap_ms(tcp_rtt),
        }
    }

    pub fn disconnect(&self, socket_id: Uuid) {
        self.sockets.remove(&socket_id);
    }

    #[cfg(test)]
    pub fn sample_count(&self, socket_id: Uuid) -> usize {
        self.sockets
            .get(&socket_id)
            .map_or(0, |lag| lag.rtts_ms.len())
    }
}

// The lower of the two middle samples, so holding back up to half the window's pongs cannot lift
// the estimate past an honest one.
fn lower_median(samples: impl Iterator<Item = f64>) -> Option<f64> {
    let mut sorted: Vec<f64> = samples.collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    Some(sorted[(sorted.len() - 1) / 2])
}

fn cap_ms(tcp_rtt: Option<Duration>) -> f64 {
    let Some(rtt) = tcp_rtt else {
        return CEILING_MS;
    };
    let rtt = rtt_ms(rtt);
    (CAP_RTT_MULTIPLIER * rtt)
        .max(rtt + CAP_SLACK_MS)
        .min(CEILING_MS)
}

pub fn rtt_ms(rtt: Duration) -> f64 {
    rtt.as_micros() as f64 / 1000.0
}

pub fn tcp_rtt_from_headers(headers: &HeaderMap) -> Option<Duration> {
    let micros = headers
        .get(TCP_RTT_HEADER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|&micros| micros > 0)?;
    Some(Duration::from_micros(micros))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket_with(lags: &SocketLags, tcp_rtt: Option<Duration>, rtts_ms: &[u64]) -> Uuid {
        let socket_id = Uuid::new_v4();
        lags.connect(socket_id, tcp_rtt);
        for rtt in rtts_ms {
            lags.record_round_trip(socket_id, Duration::from_millis(*rtt));
        }
        socket_id
    }

    #[test]
    fn the_estimate_is_the_lower_median_of_round_trips() {
        let lags = SocketLags::default();
        let odd = socket_with(&lags, None, &[100, 60, 80]);
        let even = socket_with(&lags, None, &[100, 60, 80, 120]);

        assert_eq!(lags.lag(odd).estimate_ms, Some(80.0));
        assert_eq!(lags.lag(even).estimate_ms, Some(80.0));
    }

    #[test]
    fn a_socket_without_samples_has_no_estimate_and_gets_nothing() {
        let lags = SocketLags::default();
        let fresh = socket_with(&lags, None, &[]);

        assert_eq!(lags.lag(fresh).estimate_ms, None);
        assert_eq!(lags.lag(fresh).compensable_ms(), 0.0);
        assert_eq!(lags.lag(Uuid::new_v4()).compensable_ms(), 0.0);
    }

    #[test]
    fn holding_back_half_the_pongs_cannot_lift_the_estimate_past_honest_ones() {
        let lags = SocketLags::default();
        let mut rtts = vec![40; WINDOW / 2];
        rtts.extend(vec![2000; WINDOW / 2]);
        let socket_id = socket_with(&lags, None, &rtts);

        assert_eq!(lags.lag(socket_id).estimate_ms, Some(40.0));
    }

    #[test]
    fn the_window_keeps_only_the_latest_samples() {
        let lags = SocketLags::default();
        let mut rtts = vec![2000; WINDOW];
        rtts.extend(vec![40; WINDOW / 2 + 1]);
        let socket_id = socket_with(&lags, None, &rtts);

        assert_eq!(lags.lag(socket_id).estimate_ms, Some(40.0));
    }

    #[test]
    fn the_estimate_follows_a_lasting_change_once_it_fills_half_the_window() {
        let lags = SocketLags::default();
        let socket_id = socket_with(&lags, None, &[40; WINDOW]);
        let half = WINDOW / 2;

        for _ in 0..half {
            lags.record_round_trip(socket_id, Duration::from_millis(300));
        }
        assert_eq!(
            lags.lag(socket_id).estimate_ms,
            Some(40.0),
            "half the window is not enough"
        );
        lags.record_round_trip(socket_id, Duration::from_millis(300));
        assert_eq!(lags.lag(socket_id).estimate_ms, Some(300.0));

        for _ in 0..=half {
            lags.record_round_trip(socket_id, Duration::from_millis(40));
        }
        assert_eq!(
            lags.lag(socket_id).estimate_ms,
            Some(40.0),
            "and back when it recovers"
        );
    }

    #[test]
    fn old_samples_stop_counting() {
        let lags = SocketLags::default();
        let socket_id = socket_with(&lags, None, &[40]);
        let now = Instant::now();

        assert_eq!(
            lags.lag_at(socket_id, now + SAMPLE_TTL - Duration::from_secs(1))
                .estimate_ms,
            Some(40.0)
        );
        assert_eq!(lags.lag_at(socket_id, now + SAMPLE_TTL).estimate_ms, None);
    }

    #[test]
    fn sockets_of_one_user_are_measured_apart() {
        let lags = SocketLags::default();
        let fast = socket_with(&lags, None, &[20]);
        let slow = socket_with(&lags, None, &[600]);

        assert_eq!(lags.lag(fast).estimate_ms, Some(20.0));
        assert_eq!(lags.lag(slow).estimate_ms, Some(600.0));
    }

    #[test]
    fn disconnect_forgets_the_socket() {
        let lags = SocketLags::default();
        let socket_id = socket_with(&lags, Some(Duration::from_millis(40)), &[40]);
        lags.disconnect(socket_id);
        lags.record_round_trip(socket_id, Duration::from_millis(40));

        assert_eq!(
            lags.lag(socket_id),
            ProtocolLag {
                estimate_ms: None,
                cap_ms: CEILING_MS,
            }
        );
    }

    #[test]
    fn absent_garbage_and_zero_headers_mean_no_tcp_rtt() {
        use actix_web::http::header::{HeaderName, HeaderValue};

        let headers = |value: Option<&'static str>| {
            let mut headers = HeaderMap::new();
            if let Some(value) = value {
                headers.insert(
                    HeaderName::from_static(TCP_RTT_HEADER),
                    HeaderValue::from_static(value),
                );
            }
            headers
        };
        for value in [
            None,
            Some(""),
            Some("abc"),
            Some("-5"),
            Some("1.5"),
            Some("0"),
        ] {
            assert_eq!(tcp_rtt_from_headers(&headers(value)), None, "{value:?}");
        }
        assert_eq!(
            tcp_rtt_from_headers(&headers(Some("12345"))),
            Some(Duration::from_micros(12345))
        );
    }

    #[test]
    fn the_cap_follows_the_tcp_rtt_formula() {
        let cap = |rtt_ms: u64| cap_ms(Some(Duration::from_millis(rtt_ms)));

        assert_eq!(cap(40), 90.0);
        assert_eq!(cap(100), 150.0);
        assert_eq!(cap(400), 600.0);
        assert_eq!(cap(800), CEILING_MS);
    }

    #[test]
    fn the_ceiling_applies_with_and_without_the_header() {
        assert_eq!(cap_ms(None), CEILING_MS);
        assert_eq!(cap_ms(Some(Duration::from_secs(5))), CEILING_MS);

        let lags = SocketLags::default();
        let unmeasured = socket_with(&lags, None, &[5000]);
        assert_eq!(lags.lag(unmeasured).compensable_ms(), CEILING_MS);
        assert!(lags.lag(unmeasured).capped());
    }

    #[test]
    fn the_estimate_is_capped_by_the_connections_tcp_rtt() {
        let lags = SocketLags::default();
        let honest = socket_with(&lags, Some(Duration::from_millis(40)), &[44]);
        let holding_back = socket_with(&lags, Some(Duration::from_millis(40)), &[1000]);

        assert_eq!(lags.lag(honest).compensable_ms(), 44.0);
        assert!(!lags.lag(honest).capped());
        assert_eq!(lags.lag(holding_back).compensable_ms(), 90.0);
        assert!(lags.lag(holding_back).capped());
    }
}
