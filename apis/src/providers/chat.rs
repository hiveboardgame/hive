use crate::{
    functions::{
        blocks_mutes::get_blocked_user_ids,
        chat::{get_chat_history, get_messages_hub_data, mark_chat_read},
    },
    responses::AccountResponse,
};

use super::{
    api_requests::ApiRequests,
    auth_context::AuthContext,
    websocket::{ConnectionReadyState, WebsocketContext},
    AlertType,
    AlertsContext,
    ApiRequestsProvider,
};
use chrono::{DateTime, Utc};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{
    ChatDestination,
    ChatHistoryResponse,
    ChatMessage,
    ChatMessageContainer,
    ConversationKey,
    ConversationUnreadState,
    DmConversation,
    GameId,
    GameThread,
    MessagesHubData,
    TournamentId,
};
use std::collections::{hash_map::Entry, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

const GLOBAL_HISTORY_LIMIT: i64 = 3;
const MAX_STORED_MESSAGES_PER_CHANNEL: usize = 200;
const MESSAGES_HUB_SECTION_LIMIT: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingOutgoingChat {
    client_id: Uuid,
    key: ConversationKey,
    message: String,
    turn: Option<usize>,
}

fn empty_messages_hub_data() -> MessagesHubData {
    MessagesHubData {
        dms: Vec::new(),
        tournaments: Vec::new(),
        games: Vec::new(),
        muted_tournament_ids: Vec::new(),
        unread_states: Vec::new(),
    }
}

fn message_sort_key(message: &ChatMessage) -> (i64, i64) {
    (
        message.id.unwrap_or(0),
        message
            .timestamp
            .map(|timestamp| timestamp.timestamp_millis())
            .unwrap_or(0),
    )
}

type MessageFallbackKey = (Uuid, String, Option<i64>, String, Option<usize>);

fn message_fallback_key(message: &ChatMessage) -> MessageFallbackKey {
    (
        message.user_id,
        message.username.clone(),
        message
            .timestamp
            .map(|timestamp| timestamp.timestamp_millis()),
        message.message.clone(),
        message.turn,
    )
}

fn latest_activity_timestamp(timestamp: Option<DateTime<Utc>>) -> DateTime<Utc> {
    timestamp.unwrap_or_else(Utc::now)
}

fn sort_and_trim_by_activity_keeping_unread<T>(
    rows: &mut Vec<T>,
    last_message_at: impl Fn(&T) -> DateTime<Utc>,
    key_for_row: impl Fn(&T) -> ConversationKey,
    unread_counts: &HashMap<ConversationKey, i64>,
) {
    rows.sort_by_key(|row| std::cmp::Reverse(last_message_at(row)));
    let mut unread = Vec::new();
    let mut read = Vec::new();
    for row in rows.drain(..) {
        if unread_counts.get(&key_for_row(&row)).copied().unwrap_or(0) > 0 {
            unread.push(row);
        } else {
            read.push(row);
        }
    }
    read.truncate(MESSAGES_HUB_SECTION_LIMIT.saturating_sub(unread.len()));
    rows.extend(unread);
    rows.extend(read);
    rows.sort_by_key(|row| std::cmp::Reverse(last_message_at(row)));
}

fn update_max_id(current: &mut i64, id: i64) {
    *current = (*current).max(id);
}

fn bump_generation(generation: RwSignal<u64>) -> u64 {
    let mut next_generation = 0;
    generation.update(|generation| {
        *generation = generation.saturating_add(1);
        next_generation = *generation;
    });
    next_generation
}

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

fn set_vec_membership<T>(items: &mut Vec<T>, item: &T, present: bool) -> bool
where
    T: Clone + PartialEq,
{
    if present {
        if items.contains(item) {
            false
        } else {
            items.push(item.clone());
            true
        }
    } else {
        let previous_len = items.len();
        items.retain(|candidate| candidate != item);
        items.len() != previous_len
    }
}

#[derive(Clone, Debug, Default)]
struct ChannelReadState {
    server: Option<ConversationUnreadState>,
    confirmed_read_through: i64,
    pending_read_through: i64,
    in_flight: bool,
    local_unread_ids: BTreeSet<i64>,
    deferred_visible_ids: BTreeSet<i64>,
    scheduled_read_through: i64,
}

impl ChannelReadState {
    fn read_floor(&self) -> i64 {
        self.confirmed_read_through.max(self.pending_read_through)
    }

    fn server_latest_message_id(&self) -> i64 {
        self.server
            .as_ref()
            .map(|state| state.latest_message_id)
            .unwrap_or(0)
    }

    fn display_count(&self, key: &ConversationKey, muted: &HashSet<TournamentId>) -> i64 {
        if matches!(key, ConversationKey::Tournament(id) if muted.contains(id)) {
            return 0;
        }
        let read_floor = self.read_floor();
        let server_count = self
            .server
            .as_ref()
            .filter(|state| state.count > 0 && state.latest_unread_message_id > read_floor)
            .map(|state| state.count)
            .unwrap_or(0);
        let server_latest_message_id = self.server_latest_message_id();
        let local_count = self
            .local_unread_ids
            .iter()
            .filter(|id| **id > read_floor && **id > server_latest_message_id)
            .count() as i64;
        server_count + local_count
    }

    fn latest_unread_message_id(
        &self,
        key: &ConversationKey,
        muted: &HashSet<TournamentId>,
    ) -> i64 {
        if matches!(key, ConversationKey::Tournament(id) if muted.contains(id)) {
            return 0;
        }
        let read_floor = self.read_floor();
        let server_latest = self
            .server
            .as_ref()
            .filter(|state| state.count > 0 && state.latest_unread_message_id > read_floor)
            .map(|state| state.latest_unread_message_id)
            .unwrap_or(0);
        let server_latest_message_id = self.server_latest_message_id();
        let local_latest = self
            .local_unread_ids
            .iter()
            .filter(|id| **id > read_floor && **id > server_latest_message_id)
            .copied()
            .max()
            .unwrap_or(0);

        server_latest.max(local_latest)
    }

    fn is_empty(&self) -> bool {
        self.server.is_none()
            && self.confirmed_read_through <= 0
            && self.pending_read_through <= 0
            && !self.in_flight
            && self.local_unread_ids.is_empty()
            && self.deferred_visible_ids.is_empty()
            && self.scheduled_read_through <= 0
    }
}

#[derive(Copy, Clone, Debug)]
struct ChatReadState {
    channels: RwSignal<HashMap<ConversationKey, ArcRwSignal<ChannelReadState>>>,
}

impl ChatReadState {
    fn new() -> Self {
        Self {
            channels: RwSignal::new(HashMap::new()),
        }
    }

    fn clear_all(&self) {
        self.channels.set(HashMap::new());
    }

    fn clear_channel(&self, key: &ConversationKey) {
        self.channels.update(|channels| {
            channels.remove(key);
        });
    }

    fn channel_signal_untracked(
        &self,
        key: &ConversationKey,
    ) -> Option<ArcRwSignal<ChannelReadState>> {
        self.channels
            .with_untracked(|channels| channels.get(key).cloned())
    }

    fn ensure_channel_signal(&self, key: &ConversationKey) -> ArcRwSignal<ChannelReadState> {
        self.channels
            .try_maybe_update(|channels| match channels.entry(key.clone()) {
                Entry::Occupied(entry) => (false, entry.get().clone()),
                Entry::Vacant(entry) => {
                    let signal = ArcRwSignal::new(ChannelReadState::default());
                    entry.insert(signal.clone());
                    (true, signal)
                }
            })
            .expect("chat read-state map signal should not be disposed")
    }

    fn prune_channel(&self, key: &ConversationKey, signal: ArcRwSignal<ChannelReadState>) {
        if signal.with_untracked(ChannelReadState::is_empty) {
            self.channels.update(|channels| {
                channels.remove(key);
            });
        }
    }

    fn update_channel(
        &self,
        key: &ConversationKey,
        create: bool,
        update: impl FnOnce(&mut ChannelReadState),
    ) {
        let signal = if create {
            Some(self.ensure_channel_signal(key))
        } else {
            self.channel_signal_untracked(key)
        };
        let Some(signal) = signal else {
            return;
        };
        signal.update(update);
        self.prune_channel(key, signal);
    }

    fn read_floor_untracked(&self, key: &ConversationKey) -> i64 {
        self.channel_signal_untracked(key)
            .map(|signal| signal.with_untracked(ChannelReadState::read_floor))
            .unwrap_or(0)
    }

    fn record_confirmed_read(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= 0 {
            return;
        }
        self.update_channel(key, true, |state| {
            update_max_id(&mut state.confirmed_read_through, read_through_id);
            if state.pending_read_through <= read_through_id {
                state.pending_read_through = 0;
            }
            if state.scheduled_read_through <= read_through_id {
                state.scheduled_read_through = 0;
            }
            state.local_unread_ids.retain(|id| *id > read_through_id);
            state
                .deferred_visible_ids
                .retain(|id| *id > read_through_id);
        });
    }

    fn clear_local_through(&self, key: &ConversationKey, read_through_id: i64) {
        self.update_channel(key, false, |state| {
            state.local_unread_ids.retain(|id| *id > read_through_id);
        });
    }

    fn clear_deferred_visible_through(&self, key: &ConversationKey, read_through_id: i64) {
        self.update_channel(key, false, |state| {
            state
                .deferred_visible_ids
                .retain(|id| *id > read_through_id);
        });
    }

    fn remove_pending_read_at_or_below(&self, key: &ConversationKey, read_through_id: i64) {
        self.update_channel(key, false, |state| {
            if state.pending_read_through <= read_through_id {
                state.pending_read_through = 0;
            }
        });
    }

    fn set_pending_read(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= 0 {
            return;
        }
        self.update_channel(key, true, |state| {
            update_max_id(&mut state.pending_read_through, read_through_id);
        });
    }

    fn set_in_flight(&self, key: &ConversationKey, in_flight: bool) {
        self.update_channel(key, in_flight, |state| {
            state.in_flight = in_flight;
        });
    }

    fn add_local_unread(&self, key: &ConversationKey, message_id: i64) {
        if message_id <= 0 {
            return;
        }
        self.update_channel(key, true, |state| {
            state.local_unread_ids.insert(message_id);
        });
    }

    fn defer_visible_unread(&self, key: &ConversationKey, message_id: i64) {
        if message_id <= 0 {
            return;
        }
        self.update_channel(key, true, |state| {
            state.deferred_visible_ids.insert(message_id);
        });
    }

    fn restore_deferred_visible_unread(&self, key: &ConversationKey) {
        self.update_channel(key, false, |state| {
            let ids = std::mem::take(&mut state.deferred_visible_ids);
            state.local_unread_ids.extend(ids);
        });
    }

    fn apply_server_unread_states(&self, states: Vec<ConversationUnreadState>) -> bool {
        let state_map = states
            .into_iter()
            .map(|state| (state.key.clone(), state))
            .collect::<HashMap<_, _>>();

        let existing_keys = self
            .channels
            .with_untracked(|channels| channels.keys().cloned().collect::<Vec<_>>());
        for key in existing_keys {
            if state_map.contains_key(&key) {
                continue;
            }
            self.update_channel(&key, false, |state| {
                state.server = None;
            });
        }

        for (key, server_state) in state_map {
            let latest_server_id = server_state.latest_message_id;
            let last_read_message_id = server_state.last_read_message_id;
            self.update_channel(&key, true, |state| {
                update_max_id(&mut state.confirmed_read_through, last_read_message_id);
                if !state.in_flight && state.pending_read_through <= last_read_message_id {
                    state.pending_read_through = 0;
                }
                let read_floor = state.read_floor();
                state
                    .local_unread_ids
                    .retain(|id| *id > latest_server_id && *id > read_floor);
                state.deferred_visible_ids.retain(|id| *id > read_floor);
                state.server = Some(server_state);
            });
        }
        !self.channels.with_untracked(|channels| {
            channels
                .values()
                .all(|signal| signal.with_untracked(|state| state.server.is_none()))
        })
    }

    fn schedule_read(&self, key: &ConversationKey, read_through_id: i64) -> bool {
        let mut already_scheduled = false;
        self.update_channel(key, true, |state| {
            already_scheduled = state.scheduled_read_through > 0;
            update_max_id(&mut state.scheduled_read_through, read_through_id);
        });
        already_scheduled
    }

    fn take_scheduled_read(&self, key: &ConversationKey) -> i64 {
        let mut read_through_id = 0;
        self.update_channel(key, false, |state| {
            read_through_id = state.scheduled_read_through;
            state.scheduled_read_through = 0;
        });
        read_through_id
    }

    fn unread_counts_untracked(
        &self,
        muted: &HashSet<TournamentId>,
    ) -> HashMap<ConversationKey, i64> {
        self.channels.with_untracked(|channels| {
            channels
                .iter()
                .filter_map(|(key, state)| {
                    let count = state.with_untracked(|state| state.display_count(key, muted));
                    (count > 0).then_some((key.clone(), count))
                })
                .collect()
        })
    }

    fn total_unread_count(&self, muted: &HashSet<TournamentId>) -> i64 {
        self.channels.with(|channels| {
            channels
                .iter()
                .map(|(key, state)| state.with(|state| state.display_count(key, muted)))
                .sum()
        })
    }

    fn total_unread_count_excluding_game(
        &self,
        suppressed_game_id: Option<&GameId>,
        muted: &HashSet<TournamentId>,
    ) -> i64 {
        self.channels.with(|channels| {
            channels
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        (key, suppressed_game_id),
                        (
                            ConversationKey::Game {
                                game_id,
                                thread: GameThread::Players,
                            },
                            Some(suppressed),
                        ) if game_id == suppressed
                    )
                })
                .map(|(key, state)| state.with(|state| state.display_count(key, muted)))
                .sum()
        })
    }

    fn latest_unread_message_id_excluding_game(
        &self,
        suppressed_game_id: Option<&GameId>,
        muted: &HashSet<TournamentId>,
    ) -> i64 {
        self.channels.with(|channels| {
            channels
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        (key, suppressed_game_id),
                        (
                            ConversationKey::Game {
                                game_id,
                                thread: GameThread::Players,
                            },
                            Some(suppressed),
                        ) if game_id == suppressed
                    )
                })
                .map(|(key, state)| state.with(|state| state.latest_unread_message_id(key, muted)))
                .max()
                .unwrap_or(0)
        })
    }

    fn unread_count_for_channel(
        &self,
        key: &ConversationKey,
        muted: &HashSet<TournamentId>,
    ) -> i64 {
        self.channels.with(|channels| {
            channels
                .get(key)
                .map(|state| state.with(|state| state.display_count(key, muted)))
                .unwrap_or(0)
        })
    }

    fn unread_count_for_channel_untracked(
        &self,
        key: &ConversationKey,
        muted: &HashSet<TournamentId>,
    ) -> i64 {
        self.channels.with_untracked(|channels| {
            channels
                .get(key)
                .map(|state| state.with_untracked(|state| state.display_count(key, muted)))
                .unwrap_or(0)
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Chat {
    messages: RwSignal<HashMap<ConversationKey, Vec<ChatMessage>>>,
    loaded_history: RwSignal<HashSet<ConversationKey>>,
    history_epoch: RwSignal<u64>,
    draft_messages: RwSignal<HashMap<ConversationKey, String>>,
    pending_outgoing: RwSignal<Vec<PendingOutgoingChat>>,
    send_errors: RwSignal<HashMap<ConversationKey, String>>,
    read_state: ChatReadState,
    visible_channels: RwSignal<HashMap<ConversationKey, usize>>,
    pending_chat_subscription: RwSignal<Option<(ConversationKey, u64)>>,
    confirmed_chat_subscriptions: RwSignal<HashSet<(ConversationKey, u64)>>,
    pub blocked_user_ids: RwSignal<HashSet<Uuid>>,
    pub messages_hub_data: RwSignal<Option<MessagesHubData>>,
    pub messages_hub_loading: RwSignal<bool>,
    muted_tournament_ids: RwSignal<HashSet<TournamentId>>,
    messages_hub_refresh_generation: RwSignal<u64>,
    messages_hub_active_refreshes: RwSignal<HashSet<u64>>,
    messages_hub_refresh_after_current: RwSignal<bool>,
    blocked_user_generation: RwSignal<u64>,
    session_epoch: RwSignal<u64>,
    user: Signal<Option<AccountResponse>>,
    api: Signal<ApiRequests>,
}

impl Chat {
    pub fn new(user: Signal<Option<AccountResponse>>, api: Signal<ApiRequests>) -> Self {
        Self {
            messages: RwSignal::new(HashMap::new()),
            loaded_history: RwSignal::new(HashSet::new()),
            history_epoch: RwSignal::new(0),
            draft_messages: RwSignal::new(HashMap::new()),
            pending_outgoing: RwSignal::new(Vec::new()),
            send_errors: RwSignal::new(HashMap::new()),
            read_state: ChatReadState::new(),
            visible_channels: RwSignal::new(HashMap::new()),
            pending_chat_subscription: RwSignal::new(None),
            confirmed_chat_subscriptions: RwSignal::new(HashSet::new()),
            blocked_user_ids: RwSignal::new(HashSet::new()),
            messages_hub_data: RwSignal::new(None),
            messages_hub_loading: RwSignal::new(false),
            muted_tournament_ids: RwSignal::new(HashSet::new()),
            messages_hub_refresh_generation: RwSignal::new(0),
            messages_hub_active_refreshes: RwSignal::new(HashSet::new()),
            messages_hub_refresh_after_current: RwSignal::new(false),
            blocked_user_generation: RwSignal::new(0),
            session_epoch: RwSignal::new(0),
            user,
            api,
        }
    }

    fn clear_session_state(&self) {
        self.messages.set(HashMap::new());
        self.loaded_history.set(HashSet::new());
        bump_generation(self.history_epoch);
        self.draft_messages.set(HashMap::new());
        self.pending_outgoing.set(Vec::new());
        self.send_errors.set(HashMap::new());
        self.read_state.clear_all();
        self.visible_channels.set(HashMap::new());
        self.pending_chat_subscription.set(None);
        self.confirmed_chat_subscriptions.set(HashSet::new());
        self.blocked_user_ids.set(HashSet::new());
        self.messages_hub_data.set(None);
        self.messages_hub_loading.set(false);
        self.muted_tournament_ids.set(HashSet::new());
        bump_generation(self.messages_hub_refresh_generation);
        self.messages_hub_active_refreshes.set(HashSet::new());
        self.messages_hub_refresh_after_current.set(false);
        bump_generation(self.blocked_user_generation);
        bump_generation(self.session_epoch);
    }

    pub fn session_epoch(&self) -> u64 {
        self.session_epoch.get()
    }

    pub fn session_epoch_untracked(&self) -> u64 {
        self.session_epoch.get_untracked()
    }

    pub fn history_epoch(&self) -> u64 {
        self.history_epoch.get()
    }

    pub fn history_epoch_untracked(&self) -> u64 {
        self.history_epoch.get_untracked()
    }

    fn invalidate_cached_history(&self) {
        self.loaded_history.set(HashSet::new());
        bump_generation(self.history_epoch);
    }

    pub fn set_pending_chat_subscription(&self, key: ConversationKey, session_epoch: u64) {
        self.pending_chat_subscription
            .set(Some((key, session_epoch)));
    }

    pub fn confirm_chat_subscription(&self, key: ConversationKey) {
        let session_epoch = self.session_epoch.get_untracked();
        let is_pending = self.pending_chat_subscription.with_untracked(|pending| {
            pending
                .as_ref()
                .is_some_and(|(pending_key, pending_epoch)| {
                    pending_key == &key && *pending_epoch == session_epoch
                })
        });
        if !is_pending {
            return;
        }
        self.pending_chat_subscription.set(None);
        self.confirmed_chat_subscriptions.update(|subscriptions| {
            subscriptions.insert((key, session_epoch));
        });
    }

    pub fn clear_confirmed_chat_subscription(&self, key: &ConversationKey, session_epoch: u64) {
        self.pending_chat_subscription.update(|pending| {
            if pending
                .as_ref()
                .is_some_and(|(pending_key, pending_epoch)| {
                    pending_key == key && *pending_epoch == session_epoch
                })
            {
                *pending = None;
            }
        });
        self.confirmed_chat_subscriptions.update(|subscriptions| {
            subscriptions.remove(&(key.clone(), session_epoch));
        });
    }

    pub fn clear_confirmed_chat_subscriptions(&self) {
        self.pending_chat_subscription.set(None);
        self.confirmed_chat_subscriptions.set(HashSet::new());
    }

    pub fn has_confirmed_chat_subscription(
        &self,
        key: &ConversationKey,
        session_epoch: u64,
    ) -> bool {
        self.confirmed_chat_subscriptions
            .with(|subscriptions| subscriptions.contains(&(key.clone(), session_epoch)))
    }

    pub fn current_user_id_untracked(&self) -> Option<Uuid> {
        self.user.get_untracked().as_ref().map(|user| user.user.uid)
    }

    fn is_current_user_untracked(&self, user_id: Option<Uuid>) -> bool {
        self.current_user_id_untracked() == user_id
    }

    pub fn apply_messages_hub_data(&self, data: MessagesHubData) {
        let muted_tournament_ids: HashSet<TournamentId> =
            data.muted_tournament_ids.iter().cloned().collect();
        for tournament_id in &muted_tournament_ids {
            self.clear_unread_state(&ConversationKey::tournament(tournament_id));
        }
        self.muted_tournament_ids.set(muted_tournament_ids);
        self.apply_server_unread_states(data.unread_states.clone());
        self.messages_hub_loading.set(false);
        self.messages_hub_data.set(Some(data));
    }

    fn next_messages_hub_generation(&self) -> u64 {
        bump_generation(self.messages_hub_refresh_generation)
    }

    fn refresh_messages_hub_inner(&self, show_loading: bool) {
        if self.user.get_untracked().is_none() {
            self.apply_messages_hub_data(empty_messages_hub_data());
            return;
        }

        if show_loading {
            self.messages_hub_loading.set(true);
        }
        let chat = *self;
        let request_user_id = self.current_user_id_untracked();
        let request_generation = self.next_messages_hub_generation();
        self.messages_hub_active_refreshes.update(|active| {
            active.insert(request_generation);
        });
        spawn_local(async move {
            match get_messages_hub_data().await {
                Ok(data)
                    if chat.is_current_user_untracked(request_user_id)
                        && chat.messages_hub_refresh_generation.get_untracked()
                            == request_generation =>
                {
                    chat.apply_messages_hub_data(data);
                }
                Err(_)
                    if chat.is_current_user_untracked(request_user_id)
                        && chat.messages_hub_refresh_generation.get_untracked()
                            == request_generation =>
                {
                    chat.messages_hub_loading.set(false);
                }
                _ => {}
            }
            chat.finish_messages_hub_refresh(
                request_generation,
                chat.is_current_user_untracked(request_user_id),
            );
        });
    }

    fn finish_messages_hub_refresh(&self, request_generation: u64, current_user: bool) {
        let mut still_refreshing = false;
        self.messages_hub_active_refreshes.update(|active| {
            active.remove(&request_generation);
            still_refreshing = !active.is_empty();
        });
        if !current_user || still_refreshing {
            return;
        }
        if self.messages_hub_refresh_after_current.get_untracked() {
            self.messages_hub_refresh_after_current.set(false);
            self.refresh_messages_hub_silent();
        }
    }

    fn refresh_messages_hub_after_current(&self) {
        if self
            .messages_hub_active_refreshes
            .with_untracked(|active| !active.is_empty())
        {
            self.messages_hub_refresh_after_current.set(true);
        }
    }

    pub fn refresh_messages_hub(&self) {
        self.refresh_messages_hub_inner(true);
    }

    pub fn refresh_blocked_user_ids(&self) {
        let chat = *self;
        let request_user_id = self.current_user_id_untracked();
        let request_generation = bump_generation(self.blocked_user_generation);
        spawn_local(async move {
            if let Ok(ids) = get_blocked_user_ids().await {
                if chat.is_current_user_untracked(request_user_id)
                    && chat.blocked_user_generation.get_untracked() == request_generation
                {
                    chat.blocked_user_ids.set(ids.into_iter().collect());
                }
            }
        });
    }

    pub fn set_blocked_user(&self, blocked_user_id: Uuid, blocked: bool) -> bool {
        let mut changed = false;
        self.blocked_user_ids.update(|ids| {
            changed = set_set_membership(ids, &blocked_user_id, blocked);
        });
        changed
    }

    pub fn set_tournament_muted(&self, tournament_id: &TournamentId, muted: bool) -> bool {
        let muted_state_matches = self
            .muted_tournament_ids
            .with_untracked(|ids| ids.contains(tournament_id) == muted);
        let hub_state_matches = self.messages_hub_data.with_untracked(|hub| {
            hub.as_ref()
                .is_none_or(|hub| hub.muted_tournament_ids.contains(tournament_id) == muted)
        });
        if muted_state_matches && hub_state_matches {
            return false;
        }

        let mut changed = false;
        if !muted_state_matches {
            self.muted_tournament_ids.update(|ids| {
                changed = set_set_membership(ids, tournament_id, muted);
            });
        }
        if !hub_state_matches {
            self.messages_hub_data.update(|hub| {
                let Some(hub) = hub.as_mut() else {
                    return;
                };
                changed |= set_vec_membership(&mut hub.muted_tournament_ids, tournament_id, muted);
            });
        }
        if muted && changed {
            self.clear_unread_state(&ConversationKey::tournament(tournament_id));
        }
        changed
    }

    pub fn set_tournament_muted_authoritative(
        &self,
        tournament_id: &TournamentId,
        muted: bool,
    ) -> bool {
        let changed = self.set_tournament_muted(tournament_id, muted);
        if changed {
            bump_generation(self.messages_hub_refresh_generation);
        }
        changed
    }

    pub fn tournament_muted_signal(self, tournament_id: TournamentId) -> Signal<bool> {
        Signal::derive(move || {
            self.muted_tournament_ids
                .with(|ids| ids.contains(&tournament_id))
        })
    }

    fn is_tournament_muted(&self, tournament_id: &TournamentId) -> bool {
        self.muted_tournament_ids
            .with_untracked(|ids| ids.contains(tournament_id))
    }

    pub fn has_cached_history(&self, key: &ConversationKey) -> bool {
        self.loaded_history.get_untracked().contains(key)
    }

    pub fn cached_messages(&self, key: &ConversationKey) -> Vec<ChatMessage> {
        let mut messages = self
            .messages
            .with(|messages| messages.get(key).cloned().unwrap_or_default());
        messages.sort_by_key(message_sort_key);
        messages
    }

    pub fn clear_game_thread(&self, game_id: &GameId) {
        let players_key = ConversationKey::game_players(game_id);
        let spectators_key = ConversationKey::game_spectators(game_id);
        self.messages.update(|messages| {
            messages.remove(&players_key);
            messages.remove(&spectators_key);
        });
        self.loaded_history.update(|loaded| {
            loaded.remove(&players_key);
            loaded.remove(&spectators_key);
        });
        self.clear_unread_state(&players_key);
        self.clear_unread_state(&spectators_key);
        self.draft_messages.update(|drafts| {
            drafts.remove(&players_key);
            drafts.remove(&spectators_key);
        });
        self.send_errors.update(|errors| {
            errors.remove(&players_key);
            errors.remove(&spectators_key);
        });
    }

    pub fn inject_history(&self, key: &ConversationKey, messages: Vec<ChatMessage>) {
        self.loaded_history.update(|loaded| {
            loaded.insert(key.clone());
        });
        self.merge_messages(key, messages);
        if self.is_channel_visible(key) {
            self.open_channel(key);
        }
    }

    fn merge_messages(&self, key: &ConversationKey, incoming: Vec<ChatMessage>) -> bool {
        let mut inserted_any = false;
        self.messages.update(|stored| {
            let entry = stored.entry(key.clone()).or_default();
            let mut persisted_ids = entry
                .iter()
                .filter_map(|message| message.id)
                .collect::<HashSet<_>>();
            let mut fallback_keys = entry
                .iter()
                .filter(|message| message.id.is_none())
                .map(message_fallback_key)
                .collect::<HashSet<_>>();
            for message in incoming {
                let duplicate = if let Some(id) = message.id {
                    !persisted_ids.insert(id)
                } else {
                    !fallback_keys.insert(message_fallback_key(&message))
                };
                if !duplicate {
                    entry.push(message);
                    inserted_any = true;
                }
            }
            entry.sort_by_key(message_sort_key);
            if matches!(key, ConversationKey::Global) {
                if entry.len() > GLOBAL_HISTORY_LIMIT as usize {
                    let drop_count = entry.len() - GLOBAL_HISTORY_LIMIT as usize;
                    entry.drain(0..drop_count);
                }
            } else if entry.len() > MAX_STORED_MESSAGES_PER_CHANNEL {
                let drop_count = entry.len() - MAX_STORED_MESSAGES_PER_CHANNEL;
                entry.drain(0..drop_count);
            }
        });
        inserted_any
    }

    pub async fn fetch_channel_history(
        &self,
        key: &ConversationKey,
    ) -> Result<ChatHistoryResponse, String> {
        let limit = if matches!(key, ConversationKey::Global) {
            Some(GLOBAL_HISTORY_LIMIT)
        } else {
            None
        };
        get_chat_history(key.clone(), limit)
            .await
            .map_err(|error| error.to_string())
    }

    pub fn draft_message(&self, key: &ConversationKey) -> String {
        self.draft_messages
            .with(|drafts| drafts.get(key).cloned().unwrap_or_default())
    }

    fn draft_message_untracked(&self, key: &ConversationKey) -> String {
        self.draft_messages
            .with_untracked(|drafts| drafts.get(key).cloned().unwrap_or_default())
    }

    pub fn set_draft_message(&self, key: &ConversationKey, message: String) {
        self.send_errors.update(|errors| {
            errors.remove(key);
        });
        self.draft_messages.update(|drafts| {
            if message.is_empty() {
                drafts.remove(key);
            } else {
                drafts.insert(key.clone(), message);
            }
        });
    }

    pub fn clear_draft_message(&self, key: &ConversationKey) {
        self.draft_messages.update(|drafts| {
            drafts.remove(key);
        });
    }

    pub fn chat_send_error(&self, key: &ConversationKey) -> Option<String> {
        self.send_errors.with(|errors| errors.get(key).cloned())
    }

    fn clear_chat_send_error(&self, key: &ConversationKey) {
        self.send_errors.update(|errors| {
            errors.remove(key);
        });
    }

    fn queue_pending_outgoing(
        &self,
        client_id: Uuid,
        key: ConversationKey,
        message: String,
        turn: Option<usize>,
    ) {
        self.pending_outgoing.update(|pending| {
            pending.push(PendingOutgoingChat {
                client_id,
                key,
                message,
                turn,
            });
        });
    }

    fn take_pending_outgoing(
        &self,
        client_id: Option<Uuid>,
        key: Option<&ConversationKey>,
    ) -> Option<PendingOutgoingChat> {
        let mut removed = None;
        self.pending_outgoing.update(|pending| {
            let idx = if let Some(client_id) = client_id {
                pending
                    .iter()
                    .position(|pending| pending.client_id == client_id)
            } else {
                match key {
                    Some(key) => pending.iter().position(|pending| &pending.key == key),
                    None => (!pending.is_empty()).then_some(0),
                }
            };
            if let Some(idx) = idx {
                removed = Some(pending.remove(idx));
            }
        });
        removed
    }

    pub fn handle_failed_chat_send(
        &self,
        key: Option<ConversationKey>,
        client_id: Option<Uuid>,
        reason: String,
    ) {
        let pending = self.take_pending_outgoing(client_id, key.as_ref());
        let had_pending = pending.is_some();
        let Some(error_key) = key.or_else(|| pending.as_ref().map(|pending| pending.key.clone()))
        else {
            return;
        };

        if let Some(pending) = pending {
            if self.draft_message_untracked(&pending.key).is_empty() {
                self.set_draft_message(&pending.key, pending.message);
            }
        }
        self.send_errors.update(|errors| {
            errors.insert(error_key, reason);
        });
        if had_pending && self.messages_hub_data.get_untracked().is_some() {
            self.refresh_messages_hub();
        }
    }

    pub fn send(&self, message: &str, destination: ChatDestination, turn: Option<usize>) -> bool {
        let Some(account) = self.user.get_untracked() else {
            return false;
        };
        let key = ConversationKey::from_destination(&destination);
        let turn = match destination {
            ChatDestination::GamePlayers(_) | ChatDestination::GameSpectators(_) => turn,
            _ => None,
        };
        let client_message_id = Uuid::new_v4();
        let msg = ChatMessage::new(
            account.user.username.clone(),
            account.user.uid,
            message,
            None,
            turn,
        );
        self.clear_chat_send_error(&key);
        let dm_username = dm_username_for_send(&destination);
        let container =
            ChatMessageContainer::new_with_client_id(destination, &msg, Some(client_message_id));
        if !self.api.get_untracked().chat(&container) {
            self.send_errors.update(|errors| {
                errors.insert(key, "Connection is not open. Please try again.".to_string());
            });
            return false;
        }
        self.queue_pending_outgoing(client_message_id, key.clone(), msg.message.clone(), turn);
        self.record_catalog_activity_for_key(&key, dm_username, None);
        true
    }

    pub fn recv(&self, container: ChatMessageContainer) {
        let (key, dm_username) = self.key_for_incoming(&container);
        let from_self = self.current_user_id_untracked() == Some(container.message.user_id);

        if from_self {
            let pending = self.take_pending_outgoing(container.client_id, Some(&key));
            if pending.is_some() || container.client_id.is_none() {
                self.clear_chat_send_error(&key);
            }
        }

        let inserted = self.merge_messages(&key, vec![container.message.clone()]);
        if !inserted {
            return;
        }

        if matches!(container.destination, ChatDestination::Global) {
            let alerts = expect_context::<AlertsContext>();
            alerts
                .last_alert
                .update(|alert| *alert = Some(AlertType::Warn(container.message.message.clone())));
        }

        let is_visible = self.is_channel_visible(&key);
        let tracks_unread = self.tracks_unread(&key);
        let persisted_message_id = container.message.id.filter(|id| *id > 0);
        if !from_self && persisted_message_id.is_none() {
            log::debug!("received persisted chat message without id for {:?}", key);
            self.refresh_messages_hub_silent();
        }
        if !from_self && tracks_unread {
            if let Some(message_id) = persisted_message_id {
                if is_visible {
                    self.defer_visible_unread(&key, message_id);
                } else {
                    self.add_local_unread(&key, message_id);
                }
            }
        } else if !from_self && is_visible && self.tracks_read_receipts(&key) {
            if let Some(message_id) = persisted_message_id {
                self.schedule_read_flush(&key, message_id);
            }
        }
        self.record_catalog_activity_for_key(&key, dm_username, container.message.timestamp);
    }

    fn key_for_incoming(
        &self,
        container: &ChatMessageContainer,
    ) -> (ConversationKey, Option<String>) {
        if let ChatDestination::User((destination_user_id, destination_username)) =
            &container.destination
        {
            let from_self = self.current_user_id_untracked() == Some(container.message.user_id);
            let other_user_id = if from_self {
                *destination_user_id
            } else {
                container.message.user_id
            };
            let username = if from_self {
                destination_username.clone()
            } else {
                container.message.username.clone()
            };
            (ConversationKey::direct(other_user_id), Some(username))
        } else {
            (
                ConversationKey::from_destination(&container.destination),
                None,
            )
        }
    }

    fn record_catalog_activity_for_key(
        &self,
        key: &ConversationKey,
        dm_username: Option<String>,
        timestamp: Option<DateTime<Utc>>,
    ) {
        let timestamp = latest_activity_timestamp(timestamp);
        match key {
            ConversationKey::Direct(other_user_id) => {
                let Some(username) = dm_username else {
                    return;
                };
                let muted = self.muted_tournament_ids.get_untracked();
                let unread_counts = self.read_state.unread_counts_untracked(&muted);
                self.update_messages_hub_or_refresh(move |hub| {
                    if let Some(row) = hub
                        .dms
                        .iter_mut()
                        .find(|row| row.other_user_id == *other_user_id)
                    {
                        row.username = username;
                        row.last_message_at = row.last_message_at.max(timestamp);
                    } else {
                        hub.dms.push(DmConversation {
                            other_user_id: *other_user_id,
                            username,
                            peer_deleted: false,
                            last_message_at: timestamp,
                        });
                    }
                    sort_and_trim_by_activity_keeping_unread(
                        &mut hub.dms,
                        |row| row.last_message_at,
                        |row| ConversationKey::direct(row.other_user_id),
                        &unread_counts,
                    );
                    true
                });
            }
            ConversationKey::Tournament(tournament_id) => {
                let muted = self.muted_tournament_ids.get_untracked();
                let unread_counts = self.read_state.unread_counts_untracked(&muted);
                self.update_messages_hub_or_refresh(|hub| {
                    let Some(row) = hub
                        .tournaments
                        .iter_mut()
                        .find(|row| &row.tournament_id == tournament_id)
                    else {
                        return false;
                    };
                    row.last_message_at = row.last_message_at.max(timestamp);
                    sort_and_trim_by_activity_keeping_unread(
                        &mut hub.tournaments,
                        |row| row.last_message_at,
                        |row| ConversationKey::tournament(&row.tournament_id),
                        &unread_counts,
                    );
                    true
                });
            }
            ConversationKey::Game { game_id, thread } => {
                if *thread == GameThread::Spectators {
                    return;
                }
                let muted = self.muted_tournament_ids.get_untracked();
                let unread_counts = self.read_state.unread_counts_untracked(&muted);
                self.update_messages_hub_or_refresh(|hub| {
                    let Some(row) = hub.games.iter_mut().find(|row| &row.game_id == game_id) else {
                        return false;
                    };
                    row.last_message_at = row.last_message_at.max(timestamp);
                    sort_and_trim_by_activity_keeping_unread(
                        &mut hub.games,
                        |row| row.last_message_at,
                        |row| ConversationKey::game_players(&row.game_id),
                        &unread_counts,
                    );
                    true
                });
            }
            ConversationKey::Global => {}
        }
    }

    fn update_messages_hub_or_refresh(&self, update: impl FnOnce(&mut MessagesHubData) -> bool) {
        let mut updated = false;
        self.messages_hub_data.update(|hub| {
            if let Some(hub) = hub.as_mut() {
                updated = update(hub);
            }
        });
        if !updated {
            self.refresh_messages_hub();
        } else {
            self.refresh_messages_hub_after_current();
        }
    }

    fn tracks_unread(&self, key: &ConversationKey) -> bool {
        match key {
            ConversationKey::Direct(_)
            | ConversationKey::Game {
                thread: GameThread::Players,
                ..
            } => true,
            ConversationKey::Tournament(tournament_id) => !self.is_tournament_muted(tournament_id),
            ConversationKey::Game {
                thread: GameThread::Spectators,
                ..
            }
            | ConversationKey::Global => false,
        }
    }

    fn tracks_read_receipts(&self, key: &ConversationKey) -> bool {
        matches!(
            key,
            ConversationKey::Direct(_)
                | ConversationKey::Game {
                    thread: GameThread::Players,
                    ..
                }
                | ConversationKey::Tournament(_)
        )
    }

    fn clear_unread_state(&self, key: &ConversationKey) {
        self.read_state.clear_channel(key);
    }

    fn max_cached_message_id(&self, key: &ConversationKey) -> i64 {
        self.messages
            .with_untracked(|messages| {
                messages
                    .get(key)
                    .and_then(|messages| messages.iter().filter_map(|message| message.id).max())
            })
            .unwrap_or(0)
    }

    fn read_floor_untracked(&self, key: &ConversationKey) -> i64 {
        self.read_state.read_floor_untracked(key)
    }

    fn add_local_unread(&self, key: &ConversationKey, message_id: i64) {
        self.read_state.add_local_unread(key, message_id);
    }

    pub fn apply_read_receipt_update(&self, key: ConversationKey, last_read_message_id: i64) {
        self.read_state
            .record_confirmed_read(&key, last_read_message_id);
    }

    fn defer_visible_unread(&self, key: &ConversationKey, message_id: i64) {
        self.read_state.defer_visible_unread(key, message_id);
        self.schedule_read_flush(key, message_id);
    }

    fn clear_deferred_visible_unread_through(&self, key: &ConversationKey, read_through_id: i64) {
        self.read_state
            .clear_deferred_visible_through(key, read_through_id);
    }

    fn restore_deferred_visible_unread(&self, key: &ConversationKey) {
        self.read_state.restore_deferred_visible_unread(key);
    }

    fn schedule_read_flush(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= self.read_floor_untracked(key) {
            return;
        }
        let already_scheduled = self.read_state.schedule_read(key, read_through_id);
        if already_scheduled {
            return;
        }
        let chat = *self;
        let key = key.clone();
        timers::schedule_read_flush(move || {
            chat.flush_scheduled_read(&key);
        });
    }

    fn flush_scheduled_read(&self, key: &ConversationKey) {
        let read_through_id = self.read_state.take_scheduled_read(key);
        if read_through_id <= 0 {
            return;
        }
        if self.is_channel_visible(key) {
            self.clear_deferred_visible_unread_through(key, read_through_id);
            self.mark_read_through(key, read_through_id);
        } else {
            self.restore_deferred_visible_unread(key);
        }
    }

    fn mark_read_through(&self, key: &ConversationKey, read_through_id: i64) {
        if read_through_id <= self.read_floor_untracked(key) {
            return;
        }
        self.read_state.clear_local_through(key, read_through_id);
        self.read_state.set_pending_read(key, read_through_id);
        self.read_state.set_in_flight(key, true);

        let chat = *self;
        let request_user_id = self.current_user_id_untracked();
        let key = key.clone();
        spawn_local(async move {
            let did_mark = mark_chat_read(key.clone(), read_through_id).await.is_ok();
            if !chat.is_current_user_untracked(request_user_id) {
                return;
            }
            chat.read_state.set_in_flight(&key, false);
            if did_mark {
                chat.read_state.record_confirmed_read(&key, read_through_id);
                chat.refresh_messages_hub_silent();
            } else {
                chat.read_state
                    .remove_pending_read_at_or_below(&key, read_through_id);
                chat.refresh_messages_hub_silent();
            }
        });
    }

    pub fn open_channel(&self, key: &ConversationKey) {
        let latest_id = self.max_cached_message_id(key);
        if latest_id <= 0 || !self.tracks_read_receipts(key) {
            return;
        }
        self.mark_read_through(key, latest_id);
    }

    pub fn set_channel_visible(&self, key: &ConversationKey) {
        self.visible_channels.update(|visible| {
            *visible.entry(key.clone()).or_default() += 1;
        });
        self.open_channel(key);
    }

    pub fn clear_channel_visible(&self, key: &ConversationKey) {
        self.visible_channels.update(|visible| {
            if let Some(count) = visible.get_mut(key) {
                if *count <= 1 {
                    visible.remove(key);
                } else {
                    *count -= 1;
                }
            }
        });
    }

    fn is_channel_visible(&self, key: &ConversationKey) -> bool {
        self.visible_channels
            .with_untracked(|visible| visible.get(key).copied().unwrap_or(0) > 0)
    }

    pub fn apply_server_unread_states(&self, states: Vec<ConversationUnreadState>) {
        if self.read_state.apply_server_unread_states(states) {
            self.mark_visible_unread_channels();
        }
    }

    fn mark_visible_unread_channels(&self) {
        let visible_keys = self.visible_channels.with_untracked(|visible| {
            visible
                .iter()
                .filter_map(|(key, count)| (*count > 0).then_some(key.clone()))
                .collect::<Vec<_>>()
        });
        for key in visible_keys {
            if !self.tracks_read_receipts(&key) {
                continue;
            }
            let latest_id = self.max_cached_message_id(&key);
            if latest_id > 0 {
                self.schedule_read_flush(&key, latest_id);
            }
        }
    }

    pub fn refresh_messages_hub_silent(&self) {
        self.refresh_messages_hub_inner(false);
    }

    pub fn total_unread_count(&self) -> i64 {
        let muted = self.muted_tournament_ids.get();
        self.read_state.total_unread_count(&muted)
    }

    pub fn total_unread_count_excluding_game(&self, suppressed_game_id: Option<&GameId>) -> i64 {
        let muted = self.muted_tournament_ids.get();
        self.read_state
            .total_unread_count_excluding_game(suppressed_game_id, &muted)
    }

    pub fn latest_unread_message_id_excluding_game(
        &self,
        suppressed_game_id: Option<&GameId>,
    ) -> i64 {
        let muted = self.muted_tournament_ids.get();
        self.read_state
            .latest_unread_message_id_excluding_game(suppressed_game_id, &muted)
    }

    pub fn unread_count_for_game(&self, game_id: &GameId) -> i64 {
        self.unread_count_for_channel(&ConversationKey::game_players(game_id))
    }

    pub fn unread_count_for_tournament(&self, tournament_id: &TournamentId) -> i64 {
        if self.is_tournament_muted(tournament_id) {
            return 0;
        }
        self.unread_count_for_channel(&ConversationKey::tournament(tournament_id))
    }

    pub fn unread_count_for_dm(&self, other_user_id: Uuid) -> i64 {
        self.unread_count_for_channel(&ConversationKey::direct(other_user_id))
    }

    pub fn unread_count_for_channel(&self, key: &ConversationKey) -> i64 {
        let muted = self.muted_tournament_ids.get();
        self.read_state.unread_count_for_channel(key, &muted)
    }

    pub fn unread_count_for_channel_untracked(&self, key: &ConversationKey) -> i64 {
        let muted = self.muted_tournament_ids.get_untracked();
        self.read_state
            .unread_count_for_channel_untracked(key, &muted)
    }
}

fn dm_username_for_send(destination: &ChatDestination) -> Option<String> {
    match destination {
        ChatDestination::User((_, username)) => Some(username.clone()),
        _ => None,
    }
}

mod timers {
    #[cfg(target_arch = "wasm32")]
    pub(super) fn schedule_read_flush(flush: impl FnOnce() + 'static) {
        use leptos::leptos_dom::helpers::set_timeout_with_handle;
        use std::{cell::RefCell, rc::Rc, time::Duration};

        const VISIBLE_CHANNEL_READ_FLUSH_DELAY: Duration = Duration::from_millis(250);

        let callback = Rc::new(RefCell::new(Some(flush)));
        let result = set_timeout_with_handle(
            {
                let callback = Rc::clone(&callback);
                move || {
                    if let Some(flush) = callback.borrow_mut().take() {
                        flush();
                    }
                }
            },
            VISIBLE_CHANNEL_READ_FLUSH_DELAY,
        );

        if result.is_err() {
            if let Some(flush) = callback.borrow_mut().take() {
                flush();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn schedule_read_flush(flush: impl FnOnce() + 'static) {
        flush();
    }
}

pub fn provide_chat() {
    let user = expect_context::<AuthContext>().user;
    let api = expect_context::<ApiRequestsProvider>().0;
    let websocket = expect_context::<WebsocketContext>();
    let chat = Chat::new(user, api);
    provide_context(chat);
    Effect::watch(
        move || {
            (
                chat.user
                    .with(|account| account.as_ref().map(|account| account.user.uid)),
                websocket.ready_state.get(),
            )
        },
        move |(user_id, ready_state), previous, _| {
            let user_id = *user_id;
            let user_changed = previous.is_none_or(|(previous_user, _)| *previous_user != user_id);
            let reconnected = *ready_state == ConnectionReadyState::Open
                && previous.is_some_and(|(_, previous_state)| {
                    *previous_state != ConnectionReadyState::Open
                });
            if user_changed || user_id.is_none() {
                chat.clear_session_state();
            } else if reconnected {
                chat.invalidate_cached_history();
                chat.clear_confirmed_chat_subscriptions();
            }
            if user_id.is_some() && (user_changed || reconnected) {
                chat.refresh_messages_hub();
                chat.refresh_blocked_user_ids();
            }
        },
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::{empty_messages_hub_data, Chat, PendingOutgoingChat};
    use crate::responses::{AccountResponse, UserResponse};
    use chrono::{TimeZone, Utc};
    use leptos::prelude::*;
    use shared_types::{
        ChatDestination,
        ChatMessage,
        ChatMessageContainer,
        ConversationKey,
        ConversationUnreadState,
        GameChannel,
        GameChatCapabilities,
        GameId,
        Takeback,
        TournamentId,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    fn account(user_id: Uuid) -> AccountResponse {
        AccountResponse {
            username: "current".to_string(),
            email: "current@example.test".to_string(),
            id: user_id,
            user: UserResponse {
                username: "current".to_string(),
                uid: user_id,
                patreon: false,
                bot: false,
                admin: false,
                ratings: HashMap::new(),
                takeback: Takeback::default(),
                deleted: false,
                lang: None,
            },
        }
    }

    fn chat_with_user(user_id: Uuid) -> Chat {
        let account = account(user_id);
        Chat::new(
            Signal::derive(move || Some(account.clone())),
            Signal::derive(|| panic!("api is not used in this test")),
        )
    }

    fn message(id: i64, user_id: Uuid, username: &str, body: &str) -> ChatMessage {
        ChatMessage {
            id: Some(id),
            user_id,
            username: username.to_string(),
            timestamp: Some(Utc.timestamp_millis_opt(id * 1000).single().unwrap()),
            message: body.to_string(),
            turn: None,
        }
    }

    #[test]
    fn received_dm_is_keyed_by_sender_not_destination() {
        let owner = Owner::new();
        owner.set();

        let current_user_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let chat = chat_with_user(current_user_id);
        chat.apply_messages_hub_data(empty_messages_hub_data());

        let incoming = ChatMessageContainer::new(
            ChatDestination::User((current_user_id, "current".to_string())),
            &message(1, sender_id, "sender", "hello"),
        );
        chat.recv(incoming);

        let sender_key = ConversationKey::direct(sender_id);
        let self_key = ConversationKey::direct(current_user_id);
        assert_eq!(chat.cached_messages(&sender_key).len(), 1);
        assert!(chat.cached_messages(&self_key).is_empty());
        assert_eq!(chat.unread_count_for_channel_untracked(&sender_key), 1);
        assert_eq!(
            chat.messages_hub_data
                .get_untracked()
                .unwrap()
                .dms
                .first()
                .map(|dm| (dm.other_user_id, dm.username.clone())),
            Some((sender_id, "sender".to_string()))
        );
    }

    #[test]
    fn pending_read_blocks_stale_server_unread_snapshot() {
        let owner = Owner::new();
        owner.set();

        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let chat = chat_with_user(user_id);
        let key = ConversationKey::direct(other_id);
        chat.read_state.set_pending_read(&key, 10);

        chat.apply_server_unread_states(vec![ConversationUnreadState {
            key: key.clone(),
            count: 4,
            latest_message_id: 8,
            latest_unread_message_id: 8,
            last_read_message_id: 0,
        }]);

        assert_eq!(chat.unread_count_for_channel_untracked(&key), 0);
    }

    #[test]
    fn latest_unread_message_id_excluding_game_suppresses_current_game() {
        let owner = Owner::new();
        owner.set();

        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let game_id = GameId("game-one".to_string());
        let chat = chat_with_user(user_id);
        chat.apply_server_unread_states(vec![
            ConversationUnreadState {
                key: ConversationKey::direct(other_id),
                count: 2,
                latest_message_id: 14,
                latest_unread_message_id: 14,
                last_read_message_id: 10,
            },
            ConversationUnreadState {
                key: ConversationKey::game_players(&game_id),
                count: 1,
                latest_message_id: 20,
                latest_unread_message_id: 20,
                last_read_message_id: 0,
            },
        ]);

        assert_eq!(chat.latest_unread_message_id_excluding_game(None), 20);
        assert_eq!(
            chat.latest_unread_message_id_excluding_game(Some(&game_id)),
            14
        );
    }

    #[test]
    fn spectator_messages_do_not_update_messages_hub() {
        let owner = Owner::new();
        owner.set();

        let user_id = Uuid::new_v4();
        let spectator_id = Uuid::new_v4();
        let game_id = GameId("game-one".to_string());
        let chat = chat_with_user(user_id);
        let original_last_message_at = Utc.timestamp_millis_opt(1_000).single().unwrap();
        let mut hub = empty_messages_hub_data();
        hub.games.push(GameChannel {
            game_id: game_id.clone(),
            label: "White vs Black".to_string(),
            access: GameChatCapabilities::new(true, true),
            last_message_at: original_last_message_at,
        });
        chat.apply_messages_hub_data(hub);

        let incoming = ChatMessageContainer::new(
            ChatDestination::GameSpectators(game_id.clone()),
            &message(2, spectator_id, "spectator", "hello spectators"),
        );
        chat.recv(incoming);

        assert_eq!(
            chat.cached_messages(&ConversationKey::game_spectators(&game_id))
                .len(),
            1
        );
        let hub = chat.messages_hub_data.get_untracked().unwrap();
        assert_eq!(hub.games.len(), 1);
        assert_eq!(hub.games[0].last_message_at, original_last_message_at);
    }

    #[test]
    fn failed_send_restores_draft_without_overwriting_current_typing() {
        let owner = Owner::new();
        owner.set();

        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let chat = chat_with_user(user_id);
        let key = ConversationKey::direct(other_id);
        chat.pending_outgoing.update(|pending| {
            pending.push(PendingOutgoingChat {
                client_id: Uuid::new_v4(),
                key: key.clone(),
                message: "old draft".to_string(),
                turn: None,
            });
        });
        chat.set_draft_message(&key, "new typing".to_string());

        chat.handle_failed_chat_send(Some(key.clone()), None, "send failed".to_string());

        assert_eq!(chat.draft_message(&key), "new typing");
        assert_eq!(chat.chat_send_error(&key), Some("send failed".to_string()));
    }

    #[test]
    fn merging_history_dedupes_by_persisted_message_id() {
        let owner = Owner::new();
        owner.set();

        let user_id = Uuid::new_v4();
        let chat = chat_with_user(user_id);
        let tournament_id = TournamentId("test-tournament".to_string());
        let key = ConversationKey::tournament(&tournament_id);
        let first = message(10, user_id, "current", "same id");
        let duplicate = ChatMessage {
            message: "changed local echo".to_string(),
            ..first.clone()
        };

        chat.inject_history(&key, vec![first]);
        chat.inject_history(&key, vec![duplicate]);

        let cached = chat.cached_messages(&key);
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].message, "same id");
    }
}
