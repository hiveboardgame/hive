use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, MutexGuard, PoisonError},
};
use uuid::Uuid;

type Arrivals = HashMap<String, Vec<(Uuid, DateTime<Utc>)>>;

// A move still waiting for a pool connection or its game's row lock is invisible to the
// database, so a deadline settled in the meantime looks here.
static RECEIVED: LazyLock<Mutex<Arrivals>> = LazyLock::new(Mutex::default);

pub struct ReceivedMove {
    game: String,
    player: Uuid,
    arrived_at: DateTime<Utc>,
}

impl ReceivedMove {
    // Stamped under the lock, so a deadline check that misses this move ran before it arrived.
    pub fn arriving(game: &str, player: Uuid) -> Self {
        Self::record_with(game, player, Utc::now)
    }

    pub fn record(game: &str, player: Uuid, arrived_at: DateTime<Utc>) -> Self {
        Self::record_with(game, player, || arrived_at)
    }

    pub fn arrived_at(&self) -> DateTime<Utc> {
        self.arrived_at
    }

    fn record_with(game: &str, player: Uuid, now: impl FnOnce() -> DateTime<Utc>) -> Self {
        let mut received = received();
        let arrived_at = now();
        received
            .entry(game.to_owned())
            .or_default()
            .push((player, arrived_at));
        Self {
            game: game.to_owned(),
            player,
            arrived_at,
        }
    }
}

impl Drop for ReceivedMove {
    fn drop(&mut self) {
        let mut received = received();
        let Some(arrivals) = received.get_mut(&self.game) else {
            return;
        };
        if let Some(index) = arrivals
            .iter()
            .position(|arrival| *arrival == (self.player, self.arrived_at))
        {
            arrivals.swap_remove(index);
        }
        if arrivals.is_empty() {
            received.remove(&self.game);
        }
    }
}

// A move settling its own deadline finds itself here, so one entry with its arrival is skipped.
pub(crate) fn awaits_move(
    game: &str,
    player: Uuid,
    arrived_before: DateTime<Utc>,
    mut own_arrival: Option<DateTime<Utc>>,
) -> bool {
    received().get(game).is_some_and(|arrivals| {
        arrivals
            .iter()
            .filter(|(from, _)| *from == player)
            .any(|(_, arrived_at)| {
                own_arrival.take_if(|own| own == arrived_at).is_none()
                    && *arrived_at < arrived_before
            })
    })
}

fn received() -> MutexGuard<'static, Arrivals> {
    RECEIVED.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::{awaits_move, ReceivedMove};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    #[test]
    fn a_move_is_awaited_until_it_is_through_and_only_if_it_arrived_in_time() {
        let game = Uuid::new_v4().to_string();
        let (player, opponent) = (Uuid::new_v4(), Uuid::new_v4());
        let arrived_at = Utc::now();
        let in_time = arrived_at + Duration::milliseconds(1);
        let first = ReceivedMove::record(&game, player, arrived_at);
        let resent = ReceivedMove::record(&game, player, arrived_at);

        assert!(awaits_move(&game, player, in_time, None));
        assert!(
            !awaits_move(&game, player, arrived_at, None),
            "it arrived too late"
        );
        assert!(!awaits_move(&game, opponent, in_time, None));
        assert!(
            awaits_move(&game, player, in_time, Some(arrived_at)),
            "a move skips only itself, not its resent copy"
        );

        drop(first);
        assert!(
            awaits_move(&game, player, in_time, None),
            "the same move received twice is still waiting once"
        );
        assert!(
            !awaits_move(&game, player, in_time, Some(arrived_at)),
            "a move does not wait for itself"
        );
        drop(resent);
        assert!(!awaits_move(&game, player, in_time, None));
    }
}
