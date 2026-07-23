use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::{mapref::entry::Entry, DashMap};
use shared_types::{ConversationKey, GameId};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// What a parked notification is waiting to be superseded by: a game-turn/
/// control ack (`NotificationSeen`), or a chat read receipt covering the
/// message that triggered it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AckKey {
    Game(GameId),
    Chat(ConversationKey),
}

type Key = (Uuid, AckKey);

const SEEN_TTL: Duration = Duration::from_secs(2);

#[derive(Debug)]
enum Slot {
    Parked(Arc<CancellationToken>),
    Seen(Instant),
}

#[derive(Default, Debug)]
pub struct PendingNotifications {
    inner: DashMap<Key, Slot>,
}

impl PendingNotifications {
    pub fn register(&self, recipient: Uuid, key: AckKey) -> Option<Arc<CancellationToken>> {
        match self.inner.entry((recipient, key)) {
            Entry::Occupied(mut e) => {
                if let Slot::Seen(at) = e.get() {
                    if at.elapsed() < SEEN_TTL {
                        e.remove();
                        return None;
                    }
                }
                let token = Arc::new(CancellationToken::new());
                if let Slot::Parked(old) = e.insert(Slot::Parked(token.clone())) {
                    old.cancel();
                }
                Some(token)
            }
            Entry::Vacant(e) => {
                let token = Arc::new(CancellationToken::new());
                e.insert(Slot::Parked(token.clone()));
                Some(token)
            }
        }
    }

    pub fn mark_seen(&self, user: Uuid, key: &AckKey) {
        match self.inner.entry((user, key.clone())) {
            Entry::Occupied(mut e) => {
                if let Slot::Parked(token) = e.get() {
                    token.cancel();
                }
                e.insert(Slot::Seen(Instant::now()));
            }
            Entry::Vacant(e) => {
                e.insert(Slot::Seen(Instant::now()));
            }
        }
    }

    pub fn clear(&self, recipient: Uuid, key: &AckKey, token: &Arc<CancellationToken>) {
        self.inner.remove_if(
            &(recipient, key.clone()),
            |_, slot| matches!(slot, Slot::Parked(t) if Arc::ptr_eq(t, token)),
        );
    }

    pub fn sweep(&self) {
        self.inner.retain(|_, slot| match slot {
            Slot::Seen(at) => at.elapsed() < SEEN_TTL,
            Slot::Parked(_) => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> (Uuid, AckKey) {
        (Uuid::nil(), AckKey::Game(GameId("g1".into())))
    }

    #[test]
    fn chat_ack_key_parks_and_cancels_independently_of_game_key() {
        let p = PendingNotifications::default();
        let u = Uuid::nil();
        let chat_key = AckKey::Chat(ConversationKey::Direct(Uuid::new_v4()));
        let token = p.register(u, chat_key.clone()).expect("first park");
        assert!(!token.is_cancelled());
        p.mark_seen(u, &chat_key);
        assert!(token.is_cancelled());
    }

    #[test]
    fn mark_seen_cancels_the_parked_token() {
        let p = PendingNotifications::default();
        let (u, g) = key();
        let token = p.register(u, g.clone()).expect("first park");
        assert!(!token.is_cancelled());
        p.mark_seen(u, &g);
        assert!(token.is_cancelled());
    }

    #[test]
    fn ack_before_register_suppresses_the_park() {
        let p = PendingNotifications::default();
        let (u, g) = key();
        p.mark_seen(u, &g);
        assert!(
            p.register(u, g.clone()).is_none(),
            "fresh tombstone suppresses"
        );
    }

    #[test]
    fn re_register_cancels_the_previous_token() {
        let p = PendingNotifications::default();
        let (u, g) = key();
        let first = p.register(u, g.clone()).expect("first");
        let second = p.register(u, g.clone()).expect("second");
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
    }

    #[test]
    fn clear_only_removes_its_own_entry() {
        let p = PendingNotifications::default();
        let (u, g) = key();
        let first = p.register(u, g.clone()).expect("first");
        let second = p.register(u, g.clone()).expect("second");
        p.clear(u, &g, &first);
        assert!(!second.is_cancelled());
        p.mark_seen(u, &g);
        assert!(second.is_cancelled());
    }
}
