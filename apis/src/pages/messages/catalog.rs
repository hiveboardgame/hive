use crate::{
    functions::chat::get_messages_catalog_data,
    providers::{
        chat::{CatalogActivity, Chat},
        websocket::WebsocketContext,
        AuthIdentity,
    },
};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{ConversationKey, GameThread, MessagesCatalogData};
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Default)]
struct CatalogRequestState {
    next_request_id: u64,
    active_request_id: Option<u64>,
    refresh_pending: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ActivityResult {
    Ignored,
    Updated,
    Unknown,
}

#[derive(Copy, Clone, Debug)]
pub(super) struct MessagesCatalog {
    snapshot: RwSignal<Option<Arc<MessagesCatalogData>>>,
    loading: RwSignal<bool>,
    requests: StoredValue<CatalogRequestState>,
}

impl MessagesCatalog {
    pub(super) fn new(chat: Chat) -> Self {
        let websocket = expect_context::<WebsocketContext>();
        let catalog = Self {
            snapshot: RwSignal::new(None),
            loading: RwSignal::new(true),
            requests: StoredValue::new(CatalogRequestState::default()),
        };

        Effect::watch(
            move || {
                (
                    chat.identity(),
                    chat.catalog_refresh_epoch(),
                    websocket.wake_resync_epoch.get(),
                )
            },
            move |(identity, refresh_epoch, wake_epoch), previous, _| {
                let identity_changed =
                    previous.is_some_and(|(previous_identity, _, _)| previous_identity != identity);
                if previous.is_none() || identity_changed {
                    catalog.begin_session();
                }
                match identity {
                    Some(AuthIdentity::Anonymous) => {
                        catalog.replace(MessagesCatalogData::default());
                    }
                    Some(AuthIdentity::User(_)) => {
                        let refresh_requested = previous.is_some_and(
                            |(_, previous_refresh_epoch, previous_wake_epoch)| {
                                previous_refresh_epoch != refresh_epoch
                                    || previous_wake_epoch != wake_epoch
                            },
                        );
                        if previous.is_none() || identity_changed || refresh_requested {
                            catalog.request_refresh(chat);
                        }
                    }
                    None => {}
                }
            },
            true,
        );

        Effect::watch(
            move || chat.catalog_activity(),
            move |activity, _, _| {
                let Some(activity) = activity else {
                    return;
                };
                catalog.handle_activity(activity, || catalog.request_refresh(chat));
            },
            false,
        );

        catalog
    }

    pub(super) fn snapshot(&self) -> ReadSignal<Option<Arc<MessagesCatalogData>>> {
        self.snapshot.read_only()
    }

    pub(super) fn loading(&self) -> ReadSignal<bool> {
        self.loading.read_only()
    }

    pub(super) fn retry(&self, chat: Chat) {
        self.request_refresh(chat);
    }

    fn begin_session(&self) {
        self.snapshot.set(None);
        self.loading.set(true);
        self.requests.update_value(|requests| {
            requests.next_request_id = requests.next_request_id.saturating_add(1);
            requests.active_request_id = None;
            requests.refresh_pending = false;
        });
    }

    fn request_refresh(&self, chat: Chat) {
        let Some(AuthIdentity::User(_)) = chat.identity_untracked() else {
            return;
        };
        let Some(request_id) = self.try_begin_request() else {
            return;
        };
        if self.snapshot.get_untracked().is_none() {
            self.loading.set(true);
        }
        let catalog = *self;
        spawn_local(async move {
            let result = get_messages_catalog_data().await;
            let Some(refresh_pending) = catalog.finish_request(request_id) else {
                return;
            };
            if refresh_pending {
                catalog.request_refresh(chat);
                return;
            }
            match result {
                Ok(data) => catalog.replace(data),
                Err(error) => {
                    log::error!("failed to load messages catalog: {error}");
                    catalog.loading.set(false);
                }
            }
        });
    }

    fn try_begin_request(&self) -> Option<u64> {
        self.requests
            .try_update_value(|requests| {
                if requests.active_request_id.is_some() {
                    requests.refresh_pending = true;
                    return None;
                }
                requests.next_request_id = requests.next_request_id.saturating_add(1);
                requests.active_request_id = Some(requests.next_request_id);
                Some(requests.next_request_id)
            })
            .flatten()
    }

    fn finish_request(&self, request_id: u64) -> Option<bool> {
        self.requests
            .try_update_value(|requests| {
                if requests.active_request_id != Some(request_id) {
                    return None;
                }
                requests.active_request_id = None;
                Some(std::mem::take(&mut requests.refresh_pending))
            })
            .flatten()
    }

    fn replace(&self, data: MessagesCatalogData) {
        self.snapshot.set(Some(Arc::new(data)));
        self.loading.set(false);
    }

    fn handle_activity(
        &self,
        activity: &CatalogActivity,
        request_refresh: impl FnOnce(),
    ) -> ActivityResult {
        let result = self.apply_activity(activity);
        match result {
            ActivityResult::Ignored => {}
            ActivityResult::Updated => self.mark_refresh_pending_if_active(),
            ActivityResult::Unknown => request_refresh(),
        }
        result
    }

    fn mark_refresh_pending_if_active(&self) {
        self.requests.update_value(|requests| {
            if requests.active_request_id.is_some() {
                requests.refresh_pending = true;
            }
        });
    }

    fn apply_activity(&self, activity: &CatalogActivity) -> ActivityResult {
        match &activity.key {
            ConversationKey::Global
            | ConversationKey::Game {
                thread: GameThread::Spectators,
                ..
            } => return ActivityResult::Ignored,
            _ => {}
        }

        let current_message_id = self.snapshot.with_untracked(|snapshot| {
            let data = snapshot.as_deref()?;
            match &activity.key {
                ConversationKey::Direct(other_user_id) => data
                    .dms
                    .iter()
                    .find(|row| row.other_user_id == *other_user_id)
                    .map(|row| row.last_message_id),
                ConversationKey::Tournament(tournament_id) => data
                    .tournaments
                    .iter()
                    .find(|row| row.tournament_id == *tournament_id)
                    .map(|row| row.last_message_id),
                ConversationKey::Game {
                    game_id,
                    thread: GameThread::Players,
                } => data
                    .games
                    .iter()
                    .find(|row| row.game_id == *game_id)
                    .map(|row| row.last_message_id),
                ConversationKey::Global
                | ConversationKey::Game {
                    thread: GameThread::Spectators,
                    ..
                } => None,
            }
        });
        let Some(current_message_id) = current_message_id else {
            return ActivityResult::Unknown;
        };
        if activity.message_id <= current_message_id {
            return ActivityResult::Ignored;
        }

        self.snapshot.update(|snapshot| {
            let Some(snapshot) = snapshot else {
                return;
            };
            let data = Arc::make_mut(snapshot);
            match &activity.key {
                ConversationKey::Direct(other_user_id) => {
                    if let Some(row) = data
                        .dms
                        .iter_mut()
                        .find(|row| row.other_user_id == *other_user_id)
                    {
                        row.last_message_id = activity.message_id;
                        data.dms
                            .sort_by_key(|row| std::cmp::Reverse(row.last_message_id));
                    }
                }
                ConversationKey::Tournament(tournament_id) => {
                    if let Some(row) = data
                        .tournaments
                        .iter_mut()
                        .find(|row| row.tournament_id == *tournament_id)
                    {
                        row.last_message_id = activity.message_id;
                        data.tournaments
                            .sort_by_key(|row| std::cmp::Reverse(row.last_message_id));
                    }
                }
                ConversationKey::Game {
                    game_id,
                    thread: GameThread::Players,
                } => {
                    if let Some(row) = data.games.iter_mut().find(|row| row.game_id == *game_id) {
                        row.last_message_id = activity.message_id;
                        data.games
                            .sort_by_key(|row| std::cmp::Reverse(row.last_message_id));
                    }
                }
                ConversationKey::Global
                | ConversationKey::Game {
                    thread: GameThread::Spectators,
                    ..
                } => {}
            }
        });
        ActivityResult::Updated
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivityResult, CatalogActivity, MessagesCatalog};
    use leptos::prelude::*;
    use shared_types::{ConversationKey, DmConversation, MessagesCatalogData};
    use std::sync::Arc;
    use uuid::Uuid;

    fn catalog(data: MessagesCatalogData) -> MessagesCatalog {
        MessagesCatalog {
            snapshot: RwSignal::new(Some(Arc::new(data))),
            loading: RwSignal::new(false),
            requests: StoredValue::new(Default::default()),
        }
    }

    fn activity(key: ConversationKey, message_id: i64) -> CatalogActivity {
        CatalogActivity { key, message_id }
    }

    #[test]
    fn known_newer_activity_reorders_while_older_activity_is_ignored() {
        let owner = Owner::new();
        owner.set();
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let catalog = catalog(MessagesCatalogData {
            dms: vec![
                DmConversation {
                    other_user_id: first_id,
                    username: "first".to_string(),
                    peer_deleted: false,
                    last_message_id: 10,
                },
                DmConversation {
                    other_user_id: second_id,
                    username: "second".to_string(),
                    peer_deleted: false,
                    last_message_id: 5,
                },
            ],
            tournaments: Vec::new(),
            games: Vec::new(),
        });

        assert_eq!(
            catalog.handle_activity(&activity(ConversationKey::Direct(second_id), 12), || {
                panic!("known activity must not request an idle catalog refresh")
            }),
            ActivityResult::Updated,
        );
        let snapshot = catalog.snapshot.get_untracked().unwrap();
        assert_eq!(snapshot.dms[0].other_user_id, second_id);
        assert_eq!(snapshot.dms[0].last_message_id, 12);
        assert_eq!(
            catalog.handle_activity(&activity(ConversationKey::Direct(second_id), 7), || {
                panic!("older known activity must not request an idle catalog refresh")
            }),
            ActivityResult::Ignored,
        );
        let unchanged_snapshot = catalog.snapshot.get_untracked().unwrap();
        assert_eq!(unchanged_snapshot.dms[0].last_message_id, 12);
    }

    #[test]
    fn known_activity_during_request_queues_exactly_one_follow_up() {
        let owner = Owner::new();
        owner.set();
        let other_user_id = Uuid::new_v4();
        let catalog = catalog(MessagesCatalogData {
            dms: vec![DmConversation {
                other_user_id,
                username: "peer".to_string(),
                peer_deleted: false,
                last_message_id: 10,
            }],
            tournaments: Vec::new(),
            games: Vec::new(),
        });
        let request_id = catalog.try_begin_request().unwrap();
        assert_eq!(
            catalog.handle_activity(
                &activity(ConversationKey::Direct(other_user_id), 11),
                || panic!("known activity must coalesce behind the active request"),
            ),
            ActivityResult::Updated,
        );
        assert_eq!(
            catalog.handle_activity(
                &activity(ConversationKey::Direct(other_user_id), 9),
                || panic!("older known activity must coalesce behind the active request"),
            ),
            ActivityResult::Ignored,
        );
        assert_eq!(catalog.finish_request(request_id), Some(true));

        let follow_up_request_id = catalog.try_begin_request().unwrap();
        assert_eq!(catalog.finish_request(follow_up_request_id), Some(false),);
    }

    #[test]
    fn completion_from_an_old_session_cannot_apply() {
        let owner = Owner::new();
        owner.set();
        let catalog = catalog(MessagesCatalogData {
            dms: Vec::new(),
            tournaments: Vec::new(),
            games: Vec::new(),
        });
        let request_id = catalog.try_begin_request().unwrap();
        catalog.begin_session();
        assert_eq!(catalog.finish_request(request_id), None);
    }
}
