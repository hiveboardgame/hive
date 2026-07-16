use super::Chat;
use leptos::prelude::*;
use shared_types::{ConversationKey, TournamentId};
use std::collections::HashSet;
use uuid::Uuid;

fn set_set_membership<T>(items: &mut HashSet<T>, item: &T, present: bool) -> bool
where
    T: Clone + Eq + std::hash::Hash,
{
    if present {
        items.insert(item.clone())
    } else {
        items.remove(item)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(super) struct ChatPreferences {
    blocked_user_ids: RwSignal<HashSet<Uuid>>,
    muted_tournament_ids: RwSignal<HashSet<TournamentId>>,
}

impl ChatPreferences {
    pub(super) fn clear(&self) {
        self.blocked_user_ids.set(HashSet::new());
        self.muted_tournament_ids.set(HashSet::new());
    }

    pub(super) fn replace_blocked_user_ids(&self, blocked_user_ids: HashSet<Uuid>) {
        self.blocked_user_ids.set(blocked_user_ids);
    }

    pub(super) fn replace_muted_tournament_ids(
        &self,
        muted_tournament_ids: HashSet<TournamentId>,
    ) -> (Vec<TournamentId>, Vec<TournamentId>) {
        self.muted_tournament_ids
            .try_maybe_update(|current| {
                let newly_muted = muted_tournament_ids.difference(current).cloned().collect();
                let newly_unmuted = current.difference(&muted_tournament_ids).cloned().collect();
                let changed = !muted_tournament_ids.eq(current);
                if changed {
                    *current = muted_tournament_ids;
                }
                (changed, (newly_muted, newly_unmuted))
            })
            .unwrap_or_default()
    }

    pub(super) fn with_muted_tournament_ids_untracked<T>(
        &self,
        read: impl FnOnce(&HashSet<TournamentId>) -> T,
    ) -> T {
        self.muted_tournament_ids.with_untracked(read)
    }
}

impl Chat {
    pub(crate) fn is_blocked_user(&self, blocked_user_id: &Uuid) -> bool {
        self.preferences
            .blocked_user_ids
            .with(|ids| ids.contains(blocked_user_id))
    }

    pub(crate) fn apply_blocked_user_update(&self, blocked_user_id: Uuid, blocked: bool) {
        let changed = self
            .preferences
            .blocked_user_ids
            .try_update(|ids| set_set_membership(ids, &blocked_user_id, blocked))
            .unwrap_or(false);
        if changed {
            self.refresh_chat_inbox_snapshot();
        }
    }

    pub(crate) fn set_tournament_muted(&self, tournament_id: &TournamentId, muted: bool) -> bool {
        let changed = self
            .preferences
            .muted_tournament_ids
            .try_maybe_update(|ids| {
                let changed = set_set_membership(ids, tournament_id, muted);
                (changed, changed)
            })
            .unwrap_or(false);
        if muted && changed {
            let key = ConversationKey::tournament(tournament_id);
            self.clear_unread_state(&key);
            self.set_history_unread_count(&key, 0);
        } else if !muted && changed {
            let key = ConversationKey::tournament(tournament_id);
            if let Some(conversation) = self.conversation_if_exists(&key) {
                conversation.reset_history();
            }
            self.sync_unread_display(&key);
        }
        changed
    }

    pub(crate) fn set_tournament_muted_authoritative(
        &self,
        tournament_id: &TournamentId,
        muted: bool,
    ) -> bool {
        let changed = self.set_tournament_muted(tournament_id, muted);
        if changed {
            self.refresh_chat_inbox_snapshot();
            if !muted {
                self.request_catalog_refresh();
            }
        }
        changed
    }

    pub(crate) fn tournament_muted_signal(self, tournament_id: TournamentId) -> Signal<bool> {
        Signal::derive(move || {
            self.preferences
                .muted_tournament_ids
                .with(|ids| ids.contains(&tournament_id))
        })
    }

    pub(super) fn tournament_muted_untracked(&self, tournament_id: &TournamentId) -> bool {
        self.preferences
            .muted_tournament_ids
            .with_untracked(|ids| ids.contains(tournament_id))
    }
}

#[cfg(test)]
mod tests {
    use super::Chat;
    use crate::providers::AuthIdentity;
    use leptos::prelude::{Owner, Set, WithValue};
    use shared_types::{ConversationKey, TournamentId};
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn unrelated_unmute_creates_neither_a_handle_nor_a_history_request() {
        let owner = Owner::new();
        owner.set();
        let tournament_id = TournamentId("unrelated".to_string());
        let chat = Chat::new(
            super::super::test_websocket(),
            Some(AuthIdentity::User(Uuid::new_v4())),
        );
        chat.preferences
            .muted_tournament_ids
            .set(HashSet::from([tournament_id.clone()]));

        assert!(chat.set_tournament_muted(&tournament_id, false));
        chat.conversations.with_value(|registry| {
            assert!(!registry
                .entries
                .contains_key(&ConversationKey::tournament(&tournament_id)));
        });
    }
}
