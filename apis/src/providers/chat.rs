use crate::{
    chat::ConversationKey,
    functions::{
        blocks_mutes::get_blocked_user_ids,
        chat::{get_chat_history, get_chat_unread_counts, get_messages_hub_data, mark_chat_read},
    },
    responses::AccountResponse,
};
use chrono::{DateTime, Utc};

use super::{
    api_requests::ApiRequests,
    auth_context::AuthContext,
    AlertType,
    AlertsContext,
    ApiRequestsProvider,
};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{
    ChatDestination,
    ChatHistoryResponse,
    ChatMessage,
    ChatMessageContainer,
    DmConversation,
    GameChannel,
    GameId,
    GameThread,
    MessagesHubData,
    TournamentChannel,
    TournamentId,
    UnreadCount,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const RECENT_ANNOUNCEMENTS_LIMIT: usize = 3;
const HISTORY_TIMESTAMP_SKEW_MILLIS: i64 = 500;
const MAX_STORED_MESSAGES_PER_CHANNEL: usize = 200;
const MAX_STORED_CHANNELS_PER_SECTION: usize = 128;
const MESSAGES_HUB_SECTION_LIMIT: usize = 25;

fn empty_messages_hub_data() -> MessagesHubData {
    MessagesHubData {
        dms: Vec::new(),
        tournaments: Vec::new(),
        games: Vec::new(),
    }
}

fn latest_activity_timestamp(timestamp: Option<DateTime<Utc>>) -> DateTime<Utc> {
    timestamp.unwrap_or_else(Utc::now)
}

fn sort_and_trim_dm_catalog(items: &mut Vec<DmConversation>) {
    items.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    items.truncate(MESSAGES_HUB_SECTION_LIMIT);
}

fn sort_and_trim_tournament_catalog(items: &mut Vec<TournamentChannel>) {
    items.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    items.truncate(MESSAGES_HUB_SECTION_LIMIT);
}

fn sort_and_trim_game_catalog(items: &mut Vec<GameChannel>) {
    items.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    items.truncate(MESSAGES_HUB_SECTION_LIMIT);
}

fn upsert_dm_catalog_row(
    hub: &mut MessagesHubData,
    other_user_id: Uuid,
    username: String,
    timestamp: DateTime<Utc>,
) {
    if let Some(dm) = hub
        .dms
        .iter_mut()
        .find(|dm| dm.other_user_id == other_user_id)
    {
        dm.username = username;
        if timestamp > dm.last_message_at {
            dm.last_message_at = timestamp;
        }
    } else {
        hub.dms.push(DmConversation {
            other_user_id,
            username,
            last_message_at: timestamp,
        });
    }

    sort_and_trim_dm_catalog(&mut hub.dms);
}

fn update_tournament_catalog_row_if_present(
    hub: &mut MessagesHubData,
    tournament_id: &TournamentId,
    timestamp: DateTime<Utc>,
) -> bool {
    let Some(channel) = hub
        .tournaments
        .iter_mut()
        .find(|channel| channel.nanoid == tournament_id.0)
    else {
        return false;
    };

    if timestamp > channel.last_message_at {
        channel.last_message_at = timestamp;
    }
    sort_and_trim_tournament_catalog(&mut hub.tournaments);
    true
}

fn update_game_catalog_row_if_present(
    hub: &mut MessagesHubData,
    game_id: &GameId,
    thread: GameThread,
    timestamp: DateTime<Utc>,
) -> bool {
    let Some(channel) = hub
        .games
        .iter_mut()
        .find(|channel| channel.game_id == *game_id && channel.thread == thread)
    else {
        return false;
    };

    if timestamp > channel.last_message_at {
        channel.last_message_at = timestamp;
    }
    sort_and_trim_game_catalog(&mut hub.games);
    true
}

fn last_message_timestamp(messages: &[ChatMessage]) -> i64 {
    messages
        .last()
        .and_then(|message| {
            message
                .timestamp
                .map(|timestamp| timestamp.timestamp_millis())
        })
        .unwrap_or(0)
}

fn trim_stored_messages(messages: &mut Vec<ChatMessage>) {
    if messages.len() > MAX_STORED_MESSAGES_PER_CHANNEL {
        let trim_count = messages.len() - MAX_STORED_MESSAGES_PER_CHANNEL;
        messages.drain(0..trim_count);
    }
}

fn evict_oldest_channels<K>(
    messages: &mut HashMap<K, Vec<ChatMessage>>,
    max_channels: usize,
) -> Vec<K>
where
    K: Clone + Eq + std::hash::Hash,
{
    let mut removed_keys = Vec::new();
    while messages.len() > max_channels {
        let Some(oldest_key) = messages
            .iter()
            .min_by_key(|(_, stored_messages)| last_message_timestamp(stored_messages))
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        messages.remove(&oldest_key);
        removed_keys.push(oldest_key);
    }
    removed_keys
}

fn live_message_match_score(a: &ChatMessage, b: &ChatMessage) -> Option<i64> {
    (a == b).then_some(0)
}

fn history_message_match_score(a: &ChatMessage, b: &ChatMessage) -> Option<i64> {
    if a.user_id != b.user_id || a.turn != b.turn || a.message != b.message {
        return None;
    }
    match (a.timestamp.as_ref(), b.timestamp.as_ref()) {
        // Live websocket timestamps are taken slightly before DB insert time.
        // Use a tight skew window so history merges duplicate copies while
        // still preserving intentional repeated messages.
        (Some(left), Some(right)) => {
            let delta = left.timestamp_millis().abs_diff(right.timestamp_millis()) as i64;
            (delta <= HISTORY_TIMESTAMP_SKEW_MILLIS).then_some(delta)
        }
        // Keep pre-existing fallback behavior for missing timestamps.
        _ => Some(HISTORY_TIMESTAMP_SKEW_MILLIS + 1),
    }
}

/// Filter incoming messages to only those not already in existing.
/// Matches one incoming to at most one existing entry so repeated legitimate
/// messages are not dropped when only one prior message matches.
/// Candidate pairs are matched by the best score first to avoid order-sensitive
/// mismatches when tolerant matching is used.
fn filter_duplicate_messages_by(
    existing: &[ChatMessage],
    incoming: impl IntoIterator<Item = ChatMessage>,
    match_score: fn(&ChatMessage, &ChatMessage) -> Option<i64>,
) -> Vec<ChatMessage> {
    let incoming: Vec<_> = incoming.into_iter().collect();
    let mut candidate_pairs = Vec::new();
    for (existing_idx, existing_message) in existing.iter().enumerate() {
        for (incoming_idx, incoming_message) in incoming.iter().enumerate() {
            if let Some(score) = match_score(existing_message, incoming_message) {
                candidate_pairs.push((score, existing_idx, incoming_idx));
            }
        }
    }
    candidate_pairs
        .sort_by_key(|(score, existing_idx, incoming_idx)| (*score, *existing_idx, *incoming_idx));

    let mut matched_existing = vec![false; existing.len()];
    let mut matched_incoming = vec![false; incoming.len()];
    for (_, existing_idx, incoming_idx) in candidate_pairs {
        if matched_existing[existing_idx] || matched_incoming[incoming_idx] {
            continue;
        }
        matched_existing[existing_idx] = true;
        matched_incoming[incoming_idx] = true;
    }

    incoming
        .into_iter()
        .enumerate()
        .filter_map(|(idx, message)| (!matched_incoming[idx]).then_some(message))
        .collect()
}

/// Filter duplicate messages for live WebSocket delivery. Uses strict equality
/// so users can intentionally send the same text repeatedly.
fn filter_duplicate_live_messages(
    existing: &[ChatMessage],
    incoming: impl IntoIterator<Item = ChatMessage>,
) -> Vec<ChatMessage> {
    filter_duplicate_messages_by(existing, incoming, live_message_match_score)
}

/// Filter duplicate messages when merging fetched history into local state.
/// Uses a small timestamp skew window to reconcile websocket-vs-persisted
/// duplicates while preserving repeated messages separated in time.
fn filter_duplicate_history_messages(
    existing: &[ChatMessage],
    incoming: impl IntoIterator<Item = ChatMessage>,
) -> Vec<ChatMessage> {
    filter_duplicate_messages_by(existing, incoming, history_message_match_score)
}

/// Merge existing and incoming, deduplicate, sort by timestamp.
/// Used to avoid losing WebSocket messages when REST fetch completes after live delivery.
fn merge_and_dedupe(existing: Vec<ChatMessage>, incoming: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let new_only = filter_duplicate_history_messages(&existing, incoming);
    let mut merged: Vec<_> = existing.into_iter().chain(new_only).collect();
    merged.sort_by_key(|m| m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0));
    merged
}

fn retain_recent_announcements(messages: &mut Vec<ChatMessage>) {
    messages.sort_by_key(|m| m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0));
    if messages.len() > RECENT_ANNOUNCEMENTS_LIMIT {
        let trim_count = messages.len() - RECENT_ANNOUNCEMENTS_LIMIT;
        messages.drain(0..trim_count);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingOutgoingChat {
    key: ConversationKey,
    message: String,
    turn: Option<usize>,
}

#[derive(Copy, Clone, Debug)]
pub struct Chat {
    pub users_messages: RwSignal<HashMap<Uuid, Vec<ChatMessage>>>, // Uuid -> Messages
    pub users_new_messages: RwSignal<HashMap<Uuid, bool>>,
    pub games_private_messages: RwSignal<HashMap<GameId, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_private_new_messages: RwSignal<HashMap<GameId, bool>>,
    pub games_public_messages: RwSignal<HashMap<GameId, Vec<ChatMessage>>>, // game_id -> Messages
    pub games_public_new_messages: RwSignal<HashMap<GameId, bool>>,
    pub tournament_lobby_messages: RwSignal<HashMap<TournamentId, Vec<ChatMessage>>>, // tournament_id -> Messages
    pub tournament_lobby_new_messages: RwSignal<HashMap<TournamentId, bool>>,
    pub global_messages: RwSignal<Vec<ChatMessage>>,
    pub typed_message: RwSignal<String>,
    pending_outgoing_messages: RwSignal<Vec<PendingOutgoingChat>>,
    chat_send_errors: RwSignal<HashMap<ConversationKey, String>>,
    loaded_history_channels: RwSignal<HashSet<ConversationKey>>,
    /// Stable shared block list for chat-adjacent surfaces.
    pub blocked_user_ids: RwSignal<HashSet<Uuid>>,
    /// Server-backed unread counts keyed by canonical conversation identity.
    pub unread_counts: RwSignal<HashMap<ConversationKey, i64>>,
    /// Channels currently marked read optimistically; stale refreshes should not reintroduce unread.
    pending_read_channels: RwSignal<HashSet<ConversationKey>>,
    /// Channels currently visible in the UI. Used to suppress unread bumps for live messages in open threads.
    visible_channels: RwSignal<HashMap<ConversationKey, usize>>,
    /// Visible channels with a debounced read flush already scheduled.
    pending_visible_channel_reads: RwSignal<HashSet<ConversationKey>>,
    /// Live unread that arrived while a channel was visible and is waiting for the debounced
    /// read flush to either confirm read or restore unread if the channel closes first.
    deferred_visible_unread_counts: RwSignal<HashMap<ConversationKey, i64>>,
    /// Provider-owned Messages hub catalog for sidebar rendering.
    pub messages_hub_data: RwSignal<Option<MessagesHubData>>,
    pub messages_hub_loading: RwSignal<bool>,
    pub messages_hub_error: RwSignal<Option<String>>,
    /// Bump to invalidate any cached block list snapshots used by chat UIs.
    pub block_list_version: RwSignal<u32>,
    user: Signal<Option<AccountResponse>>,
    api: Signal<ApiRequests>,
}

impl Chat {
    pub fn new(user: Signal<Option<AccountResponse>>, api: Signal<ApiRequests>) -> Self {
        Self {
            users_messages: RwSignal::new(HashMap::new()),
            users_new_messages: RwSignal::new(HashMap::new()),
            games_private_messages: RwSignal::new(HashMap::new()),
            games_private_new_messages: RwSignal::new(HashMap::new()),
            games_public_messages: RwSignal::new(HashMap::new()),
            games_public_new_messages: RwSignal::new(HashMap::new()),
            tournament_lobby_messages: RwSignal::new(HashMap::new()),
            tournament_lobby_new_messages: RwSignal::new(HashMap::new()),
            global_messages: RwSignal::new(Vec::new()),
            typed_message: RwSignal::new(String::new()),
            pending_outgoing_messages: RwSignal::new(Vec::new()),
            chat_send_errors: RwSignal::new(HashMap::new()),
            loaded_history_channels: RwSignal::new(HashSet::new()),
            blocked_user_ids: RwSignal::new(HashSet::new()),
            unread_counts: RwSignal::new(HashMap::new()),
            pending_read_channels: RwSignal::new(HashSet::new()),
            visible_channels: RwSignal::new(HashMap::new()),
            pending_visible_channel_reads: RwSignal::new(HashSet::new()),
            deferred_visible_unread_counts: RwSignal::new(HashMap::new()),
            messages_hub_data: RwSignal::new(None),
            messages_hub_loading: RwSignal::new(false),
            messages_hub_error: RwSignal::new(None),
            block_list_version: RwSignal::new(0),
            user,
            api,
        }
    }

    fn apply_messages_hub_data(&self, data: MessagesHubData) {
        self.messages_hub_error.set(None);
        self.messages_hub_loading.set(false);
        self.messages_hub_data.set(Some(data));
    }

    async fn fetch_and_store_messages_hub(self) {
        if self.user.get_untracked().is_none() {
            self.messages_hub_loading.set(false);
            self.messages_hub_error.set(None);
            self.messages_hub_data.set(Some(empty_messages_hub_data()));
            return;
        }

        match get_messages_hub_data().await {
            Ok(data) => self.apply_messages_hub_data(data),
            Err(error) => {
                self.messages_hub_loading.set(false);
                self.messages_hub_error.set(Some(error.to_string()));
            }
        }
    }

    pub fn refresh_messages_hub(&self) {
        if self.user.get_untracked().is_none() {
            self.messages_hub_loading.set(false);
            self.messages_hub_error.set(None);
            self.messages_hub_data.set(Some(empty_messages_hub_data()));
            return;
        }

        self.messages_hub_loading.set(true);
        self.messages_hub_error.set(None);
        let chat = *self;
        spawn_local(async move {
            chat.fetch_and_store_messages_hub().await;
        });
    }

    pub fn invalidate_block_list(&self) {
        self.block_list_version.update(|v| *v += 1);
    }

    pub fn set_tournament_muted(&self, tournament_nanoid: &str, muted: bool) {
        self.messages_hub_data.update(|hub| {
            let Some(hub) = hub.as_mut() else {
                return;
            };
            if let Some(channel) = hub
                .tournaments
                .iter_mut()
                .find(|channel| channel.nanoid == tournament_nanoid)
            {
                channel.muted = muted;
            }
        });
    }

    fn record_dm_catalog_activity(
        &self,
        other_user_id: Uuid,
        username: String,
        timestamp: Option<DateTime<Utc>>,
    ) {
        let timestamp = latest_activity_timestamp(timestamp);
        let mut updated = false;
        self.messages_hub_data.update(|hub| {
            let Some(hub) = hub.as_mut() else {
                return;
            };
            upsert_dm_catalog_row(hub, other_user_id, username, timestamp);
            updated = true;
        });

        if !updated {
            self.refresh_messages_hub();
        }
    }

    fn record_tournament_catalog_activity(
        &self,
        tournament_id: &TournamentId,
        timestamp: Option<DateTime<Utc>>,
    ) {
        let timestamp = latest_activity_timestamp(timestamp);
        let mut found = false;
        self.messages_hub_data.update(|hub| {
            let Some(hub) = hub.as_mut() else {
                return;
            };
            found = update_tournament_catalog_row_if_present(hub, tournament_id, timestamp);
        });

        if !found {
            self.refresh_messages_hub();
        }
    }

    fn record_game_catalog_activity(
        &self,
        game_id: &GameId,
        thread: GameThread,
        timestamp: Option<DateTime<Utc>>,
    ) {
        let timestamp = latest_activity_timestamp(timestamp);
        let mut found = false;
        self.messages_hub_data.update(|hub| {
            let Some(hub) = hub.as_mut() else {
                return;
            };
            found = update_game_catalog_row_if_present(hub, game_id, thread, timestamp);
        });

        if !found {
            self.refresh_messages_hub();
        }
    }

    fn record_catalog_activity_for_key(
        &self,
        key: &ConversationKey,
        dm_username: Option<String>,
        timestamp: Option<DateTime<Utc>>,
    ) {
        match key {
            ConversationKey::Direct(other_user_id) => {
                let Some(username) = dm_username else {
                    return;
                };
                self.record_dm_catalog_activity(*other_user_id, username, timestamp);
            }
            ConversationKey::Tournament(tournament_id) => {
                self.record_tournament_catalog_activity(tournament_id, timestamp);
            }
            ConversationKey::Game { game_id, thread } => {
                self.record_game_catalog_activity(game_id, *thread, timestamp);
            }
            ConversationKey::Global => {}
        }
    }

    fn with_messages_for_key_untracked<R>(
        &self,
        key: &ConversationKey,
        f: impl FnOnce(&[ChatMessage]) -> R,
    ) -> R {
        match key {
            ConversationKey::Direct(other_user_id) => self.users_messages.with_untracked(
                |messages| f(messages.get(other_user_id).map(Vec::as_slice).unwrap_or(&[])),
            ),
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_messages.with_untracked(|messages| {
                    f(messages.get(tournament_id).map(Vec::as_slice).unwrap_or(&[]))
                })
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => self.games_private_messages.with_untracked(|messages| {
                f(messages.get(game_id).map(Vec::as_slice).unwrap_or(&[]))
            }),
            ConversationKey::Game {
                game_id,
                thread: GameThread::Spectators,
            } => self.games_public_messages.with_untracked(|messages| {
                f(messages.get(game_id).map(Vec::as_slice).unwrap_or(&[]))
            }),
            ConversationKey::Global => self.global_messages.with_untracked(|messages| f(messages)),
        }
    }

    fn with_messages_for_key<R>(
        &self,
        key: &ConversationKey,
        f: impl FnOnce(&[ChatMessage]) -> R,
    ) -> R {
        match key {
            ConversationKey::Direct(other_user_id) => {
                self.users_messages.with(|messages| {
                    f(messages.get(other_user_id).map(Vec::as_slice).unwrap_or(&[]))
                })
            }
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_messages.with(|messages| {
                    f(messages.get(tournament_id).map(Vec::as_slice).unwrap_or(&[]))
                })
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => self.games_private_messages.with(|messages| {
                f(messages.get(game_id).map(Vec::as_slice).unwrap_or(&[]))
            }),
            ConversationKey::Game {
                game_id,
                thread: GameThread::Spectators,
            } => self.games_public_messages.with(|messages| {
                f(messages.get(game_id).map(Vec::as_slice).unwrap_or(&[]))
            }),
            ConversationKey::Global => self.global_messages.with(|messages| f(messages)),
        }
    }

    fn mark_history_loaded(&self, key: &ConversationKey) {
        self.loaded_history_channels.update(|loaded| {
            loaded.insert(key.clone());
        });
    }

    fn replace_messages_for_key(&self, key: &ConversationKey, mut messages: Vec<ChatMessage>) {
        self.mark_history_loaded(key);
        match key {
            ConversationKey::Direct(other_id) => {
                self.users_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(other_id);
                    } else {
                        stored.insert(*other_id, messages);
                    }
                });
            }
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(tournament_id);
                    } else {
                        stored.insert(tournament_id.clone(), messages);
                    }
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => {
                self.games_private_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(game_id);
                    } else {
                        stored.insert(game_id.clone(), messages);
                    }
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Spectators,
            } => {
                self.games_public_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(game_id);
                    } else {
                        stored.insert(game_id.clone(), messages);
                    }
                });
            }
            ConversationKey::Global => {
                retain_recent_announcements(&mut messages);
                self.global_messages.set(messages);
            }
        }
    }

    fn inject_messages_for_key(&self, key: &ConversationKey, messages: Vec<ChatMessage>) {
        self.mark_history_loaded(key);
        match key {
            ConversationKey::Direct(other_id) => {
                self.users_messages.update(|stored| {
                    let entry = stored.entry(*other_id).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
            }
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_messages.update(|stored| {
                    let entry = stored.entry(tournament_id.clone()).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => {
                self.games_private_messages.update(|stored| {
                    let entry = stored.entry(game_id.clone()).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Spectators,
            } => {
                self.games_public_messages.update(|stored| {
                    let entry = stored.entry(game_id.clone()).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
            }
            ConversationKey::Global => {
                self.global_messages.update(|existing| {
                    *existing = merge_and_dedupe(std::mem::take(existing), messages);
                    retain_recent_announcements(existing);
                });
            }
        }
    }

    fn append_live_messages_for_key(&self, key: &ConversationKey, messages: Vec<ChatMessage>) {
        self.mark_history_loaded(key);
        match key {
            ConversationKey::Direct(other_id) => {
                self.users_messages.update(|stored| {
                    let entry = stored.entry(*other_id).or_default();
                    entry.extend(messages);
                    trim_stored_messages(entry);
                });
            }
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_messages.update(|stored| {
                    let entry = stored.entry(tournament_id.clone()).or_default();
                    entry.extend(messages);
                    trim_stored_messages(entry);
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => {
                self.games_private_messages.update(|stored| {
                    let entry = stored.entry(game_id.clone()).or_default();
                    entry.extend(messages);
                    trim_stored_messages(entry);
                });
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Spectators,
            } => {
                self.games_public_messages.update(|stored| {
                    let entry = stored.entry(game_id.clone()).or_default();
                    entry.extend(messages);
                    trim_stored_messages(entry);
                });
            }
            ConversationKey::Global => {
                self.global_messages.update(|existing| {
                    existing.extend(messages);
                    retain_recent_announcements(existing);
                });
            }
        }
    }

    fn prune_threads_for_key(&self, key: &ConversationKey) {
        match key {
            ConversationKey::Direct(_) => self.prune_dm_threads(),
            ConversationKey::Tournament(_) => self.prune_tournament_threads(),
            ConversationKey::Game { thread, .. } => self.prune_game_threads(*thread),
            ConversationKey::Global => {}
        }
    }

    fn filter_duplicate_live_messages_for_key(
        &self,
        key: &ConversationKey,
        incoming: impl IntoIterator<Item = ChatMessage>,
    ) -> Vec<ChatMessage> {
        self.with_messages_for_key_untracked(key, |existing| {
            filter_duplicate_live_messages(existing, incoming)
        })
    }

    fn remove_channel_keys(&self, keys: impl IntoIterator<Item = ConversationKey>) {
        let keys: HashSet<_> = keys.into_iter().collect();
        if keys.is_empty() {
            return;
        }

        // Preserve server-backed unread counts when only the cached message body is pruned.
        // The Messages hub and header badge still read unread state for channels whose thread
        // contents are no longer resident locally.
        self.pending_read_channels.update(|pending| {
            pending.retain(|key| !keys.contains(key));
        });
        self.loaded_history_channels.update(|loaded| {
            loaded.retain(|key| !keys.contains(key));
        });
        self.visible_channels.update(|visible| {
            visible.retain(|key, _| !keys.contains(key));
        });
        self.pending_visible_channel_reads.update(|pending| {
            pending.retain(|key| !keys.contains(key));
        });
        self.deferred_visible_unread_counts.update(|counts| {
            counts.retain(|key, _| !keys.contains(key));
        });
    }

    fn prune_dm_threads(&self) {
        let mut removed_user_ids = Vec::new();
        self.users_messages.update(|messages| {
            removed_user_ids = evict_oldest_channels(messages, MAX_STORED_CHANNELS_PER_SECTION);
        });
        if removed_user_ids.is_empty() {
            return;
        }
        self.users_new_messages.update(|messages| {
            for user_id in &removed_user_ids {
                messages.remove(user_id);
            }
        });
        self.remove_channel_keys(removed_user_ids.into_iter().map(ConversationKey::direct));
    }

    fn prune_tournament_threads(&self) {
        let mut removed_tournament_ids = Vec::new();
        self.tournament_lobby_messages.update(|messages| {
            removed_tournament_ids =
                evict_oldest_channels(messages, MAX_STORED_CHANNELS_PER_SECTION);
        });
        if removed_tournament_ids.is_empty() {
            return;
        }
        self.tournament_lobby_new_messages.update(|messages| {
            for tournament_id in &removed_tournament_ids {
                messages.remove(tournament_id);
            }
        });
        self.remove_channel_keys(
            removed_tournament_ids
                .into_iter()
                .map(|tournament_id| ConversationKey::tournament(&tournament_id)),
        );
    }

    fn prune_game_threads(&self, thread: GameThread) {
        let mut removed_game_ids = Vec::new();
        match thread {
            GameThread::Players => {
                self.games_private_messages.update(|messages| {
                    removed_game_ids =
                        evict_oldest_channels(messages, MAX_STORED_CHANNELS_PER_SECTION);
                });
                self.games_private_new_messages.update(|messages| {
                    for game_id in &removed_game_ids {
                        messages.remove(game_id);
                    }
                });
            }
            GameThread::Spectators => {
                self.games_public_messages.update(|messages| {
                    removed_game_ids =
                        evict_oldest_channels(messages, MAX_STORED_CHANNELS_PER_SECTION);
                });
                self.games_public_new_messages.update(|messages| {
                    for game_id in &removed_game_ids {
                        messages.remove(game_id);
                    }
                });
            }
        }

        if removed_game_ids.is_empty() {
            return;
        }

        self.remove_channel_keys(removed_game_ids.into_iter().map(|game_id| match thread {
            GameThread::Players => ConversationKey::game_players(&game_id),
            GameThread::Spectators => ConversationKey::game_spectators(&game_id),
        }));
    }

    async fn fetch_and_store_unread_counts(self) {
        if let Ok(counts) = get_chat_unread_counts().await {
            self.apply_server_unread_counts(counts)
        }
    }

    async fn fetch_and_store_blocked_user_ids(self) {
        if self.user.get_untracked().is_none() {
            self.blocked_user_ids.set(HashSet::new());
            return;
        }

        if let Ok(blocked_user_ids) = get_blocked_user_ids().await {
            self.blocked_user_ids
                .set(blocked_user_ids.into_iter().collect());
        }
    }

    /// Apply a fresh server snapshot of unread counts while preserving optimistic local state.
    pub fn apply_server_unread_counts(&self, counts: Vec<UnreadCount>) {
        let merged = self.merge_server_counts_with_optimistic(counts);
        self.unread_counts.set(merged);
    }

    /// Mark a channel as read on the server (fire-and-forget).
    /// Optimistically zeros the count locally so badges update immediately.
    pub fn mark_read(&self, key: &ConversationKey) {
        self.optimistically_clear_unread(key);
        let mark_key = key.clone();
        self.pending_read_channels.update(|pending| {
            pending.insert(mark_key.clone());
        });
        let chat = *self;
        spawn_local(async move {
            let did_mark = mark_chat_read(mark_key.clone()).await.is_ok();
            if !did_mark {
                chat.pending_read_channels.update(|pending| {
                    pending.remove(&mark_key);
                });
                chat.fetch_and_store_unread_counts().await;
            }
        });
    }

    pub fn set_channel_visible(&self, key: &ConversationKey) {
        self.visible_channels.update(|visible| {
            *visible.entry(key.clone()).or_insert(0) += 1;
        });
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

    fn flush_visible_channel_read(&self, key: &ConversationKey) {
        self.pending_visible_channel_reads.update(|pending| {
            pending.remove(key);
        });
        if self.is_channel_visible(key) {
            self.clear_deferred_visible_unread(key);
            self.mark_read(key);
        } else {
            self.restore_deferred_visible_unread(key);
        }
    }

    fn clear_visible_channel_read_flush(&self, key: &ConversationKey) {
        self.pending_visible_channel_reads.update(|pending| {
            pending.remove(key);
        });
        self.clear_deferred_visible_unread(key);
    }

    fn schedule_visible_channel_read_flush(&self, key: &ConversationKey) {
        if self
            .pending_visible_channel_reads
            .with_untracked(|pending| pending.contains(key))
        {
            return;
        }
        self.pending_visible_channel_reads.update(|pending| {
            pending.insert(key.clone());
        });
        let scheduled_chat = *self;
        let scheduled_key = key.clone();
        let immediate_chat = *self;
        let immediate_key = key.clone();
        timers::schedule_visible_channel_read_flush(
            move || {
                scheduled_chat.flush_visible_channel_read(&scheduled_key);
            },
            move || {
                immediate_chat.clear_visible_channel_read_flush(&immediate_key);
            },
        );
    }

    fn defer_visible_channel_unread(&self, key: &ConversationKey) {
        self.deferred_visible_unread_counts.update(|pending| {
            pending
                .entry(key.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        });
        self.schedule_visible_channel_read_flush(key);
    }

    fn clear_deferred_visible_unread(&self, key: &ConversationKey) {
        self.deferred_visible_unread_counts.update(|pending| {
            pending.remove(key);
        });
    }

    fn take_deferred_visible_unread(&self, key: &ConversationKey) -> i64 {
        let deferred = self
            .deferred_visible_unread_counts
            .with_untracked(|pending| pending.get(key).copied())
            .unwrap_or(0)
            .max(0);
        self.deferred_visible_unread_counts.update(|pending| {
            pending.remove(key);
        });
        deferred
    }

    fn restore_deferred_visible_unread(&self, key: &ConversationKey) {
        let deferred = self.take_deferred_visible_unread(key);
        if deferred == 0 {
            return;
        }
        match key {
            ConversationKey::Direct(other_user_id) => {
                self.users_new_messages.update(|messages| {
                    messages
                        .entry(*other_user_id)
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ConversationKey::Tournament(tournament_id) => {
                self.tournament_lobby_new_messages.update(|messages| {
                    messages
                        .entry(tournament_id.clone())
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ConversationKey::Game {
                game_id,
                thread: GameThread::Players,
            } => {
                self.games_private_new_messages.update(|messages| {
                    messages
                        .entry(game_id.clone())
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ConversationKey::Game {
                thread: GameThread::Spectators,
                ..
            }
            | ConversationKey::Global => {}
        }
    }

    /// Optimistically set unread count for channel(s) to 0 so badges update immediately.
    fn optimistically_clear_unread(&self, key: &ConversationKey) {
        self.unread_counts.update(|counts| {
            if let Some(count) = counts.get_mut(key) {
                *count = 0;
            }
        });
    }

    /// Optimistically increment unread count when a live message arrives so badges update immediately.
    fn optimistically_increment_unread(&self, key: &ConversationKey) {
        self.optimistically_increment_unread_by(key, 1);
    }

    fn optimistically_increment_unread_by(&self, key: &ConversationKey, delta: i64) {
        if delta <= 0 {
            return;
        }
        self.pending_read_channels.update(|pending| {
            pending.remove(key);
        });
        self.unread_counts.update(|counts| {
            counts
                .entry(key.clone())
                .and_modify(|count| *count += delta)
                .or_insert(delta);
        });
    }

    /// Clear local "new" state for game chat.
    /// This is safe to call from passive UI flows and does not write server read receipts.
    pub fn seen_messages(&self, game_id: GameId) {
        self.games_public_new_messages.update(|m| {
            m.entry(game_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
        self.games_private_new_messages.update(|m| {
            m.entry(game_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
    }

    pub fn clear_game_thread(&self, game_id: &GameId) {
        let players_key = ConversationKey::game_players(game_id);
        let spectators_key = ConversationKey::game_spectators(game_id);

        self.games_private_messages.update(|games| {
            games.remove(game_id);
        });
        self.games_public_messages.update(|games| {
            games.remove(game_id);
        });
        self.games_private_new_messages.update(|games| {
            games.remove(game_id);
        });
        self.games_public_new_messages.update(|games| {
            games.remove(game_id);
        });
        self.loaded_history_channels.update(|loaded| {
            loaded.remove(&players_key);
            loaded.remove(&spectators_key);
        });
        self.unread_counts.update(|counts| {
            counts.remove(&players_key);
            counts.remove(&spectators_key);
        });
        self.pending_read_channels.update(|pending| {
            pending.remove(&players_key);
            pending.remove(&spectators_key);
        });
        self.visible_channels.update(|visible| {
            visible.remove(&players_key);
            visible.remove(&spectators_key);
        });
        self.pending_visible_channel_reads.update(|pending| {
            pending.remove(&players_key);
            pending.remove(&spectators_key);
        });
        self.deferred_visible_unread_counts.update(|counts| {
            counts.remove(&players_key);
            counts.remove(&spectators_key);
        });
    }

    fn clear_tournament_lobby_new_messages(&self, tournament_id: &TournamentId) {
        self.tournament_lobby_new_messages.update(|m| {
            m.entry(tournament_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
    }

    fn clear_dm_new_messages(&self, other_user_id: Uuid) {
        self.users_new_messages.update(|m| {
            m.entry(other_user_id)
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
    }

    fn clear_local_new_for_channel(&self, key: &ConversationKey) {
        match key {
            ConversationKey::Direct(other_user_id) => self.clear_dm_new_messages(*other_user_id),
            ConversationKey::Tournament(tournament_id) => {
                self.clear_tournament_lobby_new_messages(tournament_id);
            }
            ConversationKey::Game { game_id, .. } => self.seen_messages(game_id.clone()),
            ConversationKey::Global => {}
        }
    }

    fn queue_pending_outgoing_message(
        &self,
        key: ConversationKey,
        message: String,
        turn: Option<usize>,
    ) {
        self.pending_outgoing_messages.update(|pending| {
            pending.push(PendingOutgoingChat { key, message, turn });
        });
    }

    fn take_pending_outgoing_message(
        &self,
        key: Option<&ConversationKey>,
    ) -> Option<PendingOutgoingChat> {
        let mut removed = None;
        self.pending_outgoing_messages.update(|pending| {
            let index = key
                .and_then(|key| pending.iter().position(|candidate| candidate.key == *key))
                .or_else(|| (!pending.is_empty()).then_some(0));
            if let Some(index) = index {
                removed = Some(pending.remove(index));
            }
        });
        removed
    }

    pub fn clear_chat_send_error(&self, key: &ConversationKey) {
        self.chat_send_errors.update(|errors| {
            errors.remove(key);
        });
    }

    pub fn chat_send_error(&self, key: &ConversationKey) -> Option<String> {
        self.chat_send_errors
            .with(|errors| errors.get(key).cloned())
    }

    pub fn cached_messages(&self, key: &ConversationKey) -> Vec<ChatMessage> {
        let mut messages = self.with_messages_for_key(key, |messages| messages.to_vec());
        messages.sort_by_key(|message| {
            message
                .timestamp
                .map(|timestamp| timestamp.timestamp_millis())
                .unwrap_or(0)
        });
        messages
    }

    pub fn has_cached_history(&self, key: &ConversationKey) -> bool {
        self.loaded_history_channels
            .with_untracked(|loaded| loaded.contains(key))
            || self.with_messages_for_key_untracked(key, |messages| !messages.is_empty())
    }

    fn acknowledge_outgoing_message(&self, container: &ChatMessageContainer) {
        let Some(current_user_id) = self.user.get_untracked().as_ref().map(|a| a.user.uid) else {
            return;
        };
        if container.message.user_id != current_user_id {
            return;
        }

        let key = ConversationKey::from_destination(&container.destination);
        self.clear_chat_send_error(&key);
        self.pending_outgoing_messages.update(|pending| {
            if let Some(index) = pending.iter().position(|candidate| {
                candidate.key == key
                    && candidate.message == container.message.message
                    && candidate.turn == container.message.turn
            }) {
                pending.remove(index);
            }
        });
    }

    pub fn handle_failed_chat_send(&self, key: Option<ConversationKey>, reason: String) {
        let failed = self.take_pending_outgoing_message(key.as_ref());
        let error_key = key.or_else(|| failed.as_ref().map(|pending| pending.key.clone()));

        let Some(failed) = failed else {
            if let Some(error_key) = error_key {
                self.chat_send_errors.update(|errors| {
                    errors.insert(error_key, reason);
                });
            }
            return;
        };
        if self.typed_message.get_untracked().is_empty() && self.is_channel_visible(&failed.key) {
            self.typed_message.set(failed.message.clone());
        }
        self.chat_send_errors.update(|errors| {
            errors.insert(failed.key, reason);
        });
    }

    pub fn open_channel(&self, key: &ConversationKey) {
        self.clear_local_new_for_channel(key);
        if !matches!(key, ConversationKey::Global)
            && self.unread_count_for_channel_untracked(key) > 0
        {
            self.mark_read(key);
        }
    }

    /// Merge server counts with local "new" flags so optimistic unread is not overwritten by stale server state (e.g. 0 before message is persisted).
    fn merge_server_counts_with_optimistic(
        &self,
        server: Vec<UnreadCount>,
    ) -> HashMap<ConversationKey, i64> {
        let mut map: HashMap<ConversationKey, i64> = server
            .into_iter()
            .map(|unread| (unread.key, unread.count))
            .collect();
        let server_map = map.clone();
        self.users_new_messages.with_untracked(|m| {
            for (other_id, &has_new) in m.iter() {
                if has_new {
                    map.entry(ConversationKey::direct(*other_id))
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
            }
        });
        self.tournament_lobby_new_messages.with_untracked(|m| {
            for (tid, &has_new) in m.iter() {
                if has_new {
                    map.entry(ConversationKey::tournament(tid))
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
            }
        });
        self.games_private_new_messages.with_untracked(|m| {
            for (gid, &has_new) in m.iter() {
                if has_new {
                    map.entry(ConversationKey::game_players(gid))
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
            }
        });
        let pending_keys: Vec<ConversationKey> = self
            .pending_read_channels
            .with_untracked(|pending| pending.iter().cloned().collect());
        if !pending_keys.is_empty() {
            let mut resolved = Vec::new();
            for key in pending_keys {
                let server_count = server_map.get(&key).copied().unwrap_or(0);
                if server_count == 0 {
                    resolved.push(key);
                } else {
                    map.insert(key, 0);
                }
            }
            if !resolved.is_empty() {
                self.pending_read_channels.update(|pending| {
                    for key in resolved {
                        pending.remove(&key);
                    }
                });
            }
        }
        map
    }

    /// Fetch unread counts from the server and update unread_counts signal.
    /// Merges with local "new" flags so that a just-received DM/tournament message is not overwritten with 0.
    pub fn refresh_unread_counts(&self) {
        let chat = *self;
        spawn_local(async move {
            chat.fetch_and_store_unread_counts().await;
        });
    }

    pub fn refresh_blocked_user_ids(&self) {
        let chat = *self;
        spawn_local(async move {
            chat.fetch_and_store_blocked_user_ids().await;
        });
    }

    pub fn set_blocked_user(&self, blocked_user_id: Uuid, is_blocked: bool) {
        self.blocked_user_ids.update(|blocked_user_ids| {
            if is_blocked {
                blocked_user_ids.insert(blocked_user_id);
            } else {
                blocked_user_ids.remove(&blocked_user_id);
            }
        });
    }

    /// Total unread count across channels that participate in unread tracking.
    pub fn total_unread_count(&self) -> i64 {
        self.unread_counts
            .with(|counts| counts.values().sum::<i64>())
    }

    /// Unread count for a game (players channel only). Use for game list badges.
    /// Spectator messages intentionally do not contribute to badges/notifications.
    /// If local "new" flag is set, returns at least 1 so badge is not lost.
    pub fn unread_count_for_game(&self, game_id: &GameId) -> i64 {
        let from_list = self.unread_count_for_channel(&ConversationKey::game_players(game_id));
        let has_local_new = self
            .games_private_new_messages
            .with(|m| m.get(game_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    /// Unread count for a tournament lobby. Use for tournament page badge.
    /// If local "new" flag is set, returns at least 1 so badge is not lost before server state is updated.
    pub fn unread_count_for_tournament(&self, tournament_id: &TournamentId) -> i64 {
        let from_list = self.unread_count_for_channel(&ConversationKey::tournament(tournament_id));
        let has_local_new = self
            .tournament_lobby_new_messages
            .with(|m| m.get(tournament_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    /// Unread count for a DM with another user. Use for DM list badge.
    /// If local "new" flag is set, returns at least 1 so badge is not lost before server state is updated.
    pub fn unread_count_for_dm(&self, other_user_id: Uuid) -> i64 {
        let from_list = self.unread_count_for_channel(&ConversationKey::direct(other_user_id));
        let has_local_new = self
            .users_new_messages
            .with(|m| m.get(&other_user_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    pub fn unread_count_for_channel_untracked(&self, key: &ConversationKey) -> i64 {
        self.unread_counts
            .with_untracked(|counts| counts.get(key).copied().unwrap_or(0))
    }

    pub fn unread_count_for_channel(&self, key: &ConversationKey) -> i64 {
        self.unread_counts
            .with(|counts| counts.get(key).copied().unwrap_or(0))
    }

    /// Replaces the cached history for a channel with a fresh server snapshot.
    /// Used when the server-side view of a thread changes and stale local messages
    /// should not be merged back in.
    pub fn replace_history(&self, key: &ConversationKey, mut messages: Vec<ChatMessage>) {
        messages.sort_by_key(|m| m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0));
        trim_stored_messages(&mut messages);
        self.replace_messages_for_key(key, messages);
        self.prune_threads_for_key(key);
    }

    /// Injects fetched history into the correct in-memory map so the thread view can display it.
    /// Merges with existing messages and deduplicates to avoid losing WebSocket messages when REST
    /// fetch completes after live delivery.
    pub fn inject_history(&self, key: &ConversationKey, messages: Vec<ChatMessage>) {
        self.inject_messages_for_key(key, messages);
        self.prune_threads_for_key(key);
    }

    /// Loads channel history through the server-fn path and returns messages without injecting them.
    pub fn fetch_channel_history(
        &self,
        key: &ConversationKey,
    ) -> impl std::future::Future<Output = Result<ChatHistoryResponse, String>> + '_ {
        let key = key.clone();
        async move {
            let history_limit = if matches!(key, ConversationKey::Global) {
                RECENT_ANNOUNCEMENTS_LIMIT as i64
            } else {
                100
            };
            get_chat_history(key, Some(history_limit))
                .await
                .map_err(|error| error.to_string())
        }
    }

    pub fn send(&self, message: &str, destination: ChatDestination, turn: Option<usize>) {
        let channel_key = ConversationKey::from_destination(&destination);
        let dm_username = match &destination {
            ChatDestination::User((_, username)) => Some(username.clone()),
            _ => None,
        };
        self.record_catalog_activity_for_key(&channel_key, dm_username, Some(Utc::now()));
        self.user.with_untracked(|a| {
            if let Some(account) = a {
                let id = account.user.uid;
                let name = account.user.username.clone();
                self.clear_chat_send_error(&channel_key);
                let turn = match &destination {
                    ChatDestination::GamePlayers(_) | ChatDestination::GameSpectators(_) => turn,
                    _ => None,
                };
                let msg = ChatMessage::new(name, id, message, None, turn);
                self.queue_pending_outgoing_message(channel_key, msg.message.clone(), msg.turn);
                let container = ChatMessageContainer::new(destination, &msg);
                self.api.get_untracked().chat(&container);
            }
        });
    }

    pub fn recv(&mut self, containers: &[ChatMessageContainer]) {
        for container in containers {
            self.acknowledge_outgoing_message(container);
        }
        if let Some(last_message) = containers.last() {
            let is_live = containers.len() == 1;
            let from_self = self
                .user
                .get_untracked()
                .as_ref()
                .is_some_and(|a| last_message.message.user_id == a.user.uid);
            match &last_message.destination {
                ChatDestination::TournamentLobby(id) => {
                    let channel_key = ConversationKey::tournament(id);
                    let new_messages = self.filter_duplicate_live_messages_for_key(
                        &channel_key,
                        containers.iter().map(|c| c.message.clone()),
                    );
                    if new_messages.is_empty() {
                        return;
                    }
                    self.append_live_messages_for_key(&channel_key, new_messages);
                    self.prune_threads_for_key(&channel_key);
                    self.record_catalog_activity_for_key(
                        &channel_key,
                        None,
                        last_message.message.timestamp,
                    );
                    if is_live && !from_self {
                        if self.is_channel_visible(&channel_key) {
                            self.clear_tournament_lobby_new_messages(id);
                            self.defer_visible_channel_unread(&channel_key);
                        } else {
                            self.tournament_lobby_new_messages.update(|m| {
                                m.entry(id.clone())
                                    .and_modify(|value| *value = true)
                                    .or_insert(true);
                            });
                            self.optimistically_increment_unread(&channel_key);
                        }
                    }
                }

                ChatDestination::User((dest_id, _name)) => {
                    // Container destination is from sender's perspective. For recipient, the
                    // "other" in the thread is the sender (message.user_id), not dest_id.
                    let current_user_id = self.user.get_untracked().as_ref().map(|a| a.user.uid);
                    let thread_other_id = match current_user_id {
                        Some(me) if last_message.message.user_id == me => *dest_id, // I sent: other is recipient
                        _ => last_message.message.user_id, // I received: other is sender
                    };
                    let thread_username = match current_user_id {
                        Some(me) if last_message.message.user_id == me => _name.clone(),
                        _ => last_message.message.username.clone(),
                    };
                    let channel_key = ConversationKey::direct(thread_other_id);
                    let new_messages = self.filter_duplicate_live_messages_for_key(
                        &channel_key,
                        containers.iter().map(|c| c.message.clone()),
                    );
                    if new_messages.is_empty() {
                        return;
                    }
                    self.append_live_messages_for_key(&channel_key, new_messages);
                    self.prune_threads_for_key(&channel_key);
                    self.record_catalog_activity_for_key(
                        &channel_key,
                        Some(thread_username),
                        last_message.message.timestamp,
                    );
                    if is_live && !from_self {
                        if self.is_channel_visible(&channel_key) {
                            self.clear_dm_new_messages(thread_other_id);
                            self.defer_visible_channel_unread(&channel_key);
                        } else {
                            self.users_new_messages.update(|m| {
                                m.entry(thread_other_id)
                                    .and_modify(|value| *value = true)
                                    .or_insert(true);
                            });
                            self.optimistically_increment_unread(&channel_key);
                        }
                    }
                }
                ChatDestination::GamePlayers(id) => {
                    let channel_key = ConversationKey::game_players(id);
                    let new_messages = self.filter_duplicate_live_messages_for_key(
                        &channel_key,
                        containers.iter().map(|c| c.message.clone()),
                    );
                    if new_messages.is_empty() {
                        return;
                    }
                    self.append_live_messages_for_key(&channel_key, new_messages);
                    self.prune_threads_for_key(&channel_key);
                    self.record_catalog_activity_for_key(
                        &channel_key,
                        None,
                        last_message.message.timestamp,
                    );
                    if is_live && !from_self {
                        if self.is_channel_visible(&channel_key) {
                            self.seen_messages(id.clone());
                            self.defer_visible_channel_unread(&channel_key);
                        } else {
                            self.games_private_new_messages.update(|m| {
                                m.entry(id.clone())
                                    .and_modify(|value| *value = true)
                                    .or_insert(true);
                            });
                            self.optimistically_increment_unread(&channel_key);
                        }
                    }
                }
                ChatDestination::GameSpectators(id) => {
                    let channel_key = ConversationKey::game_spectators(id);
                    let new_messages = self.filter_duplicate_live_messages_for_key(
                        &channel_key,
                        containers.iter().map(|c| c.message.clone()),
                    );
                    if new_messages.is_empty() {
                        return;
                    }
                    self.append_live_messages_for_key(&channel_key, new_messages);
                    self.prune_threads_for_key(&channel_key);
                    self.record_catalog_activity_for_key(
                        &channel_key,
                        None,
                        last_message.message.timestamp,
                    );
                    if is_live && !from_self && self.is_channel_visible(&channel_key) {
                        self.seen_messages(id.clone());
                        self.schedule_visible_channel_read_flush(&channel_key);
                        // Spectator chat intentionally does not contribute to unread badges/notifications.
                    }
                }
                ChatDestination::Global => {
                    let channel_key = ConversationKey::Global;
                    let to_add = self.filter_duplicate_live_messages_for_key(
                        &channel_key,
                        containers.iter().map(|c| c.message.clone()),
                    );
                    if !to_add.is_empty() {
                        self.append_live_messages_for_key(&channel_key, to_add);
                    }
                    let alerts = expect_context::<AlertsContext>();
                    alerts.last_alert.update(|v| {
                        *v = Some(AlertType::Warn(last_message.message.message.to_string()))
                    });
                }
            }
        }
    }
}

mod timers {
    #[cfg(target_arch = "wasm32")]
    pub(super) fn schedule_visible_channel_read_flush(
        scheduled: impl FnOnce() + 'static,
        immediate: impl FnOnce() + 'static,
    ) {
        use leptos::leptos_dom::helpers::set_timeout_with_handle;
        use std::{cell::RefCell, rc::Rc, time::Duration};

        const VISIBLE_CHANNEL_READ_FLUSH_DELAY: Duration = Duration::from_millis(250);

        let scheduled_callback = Rc::new(RefCell::new(Some(scheduled)));
        let schedule_result = set_timeout_with_handle(
            {
                let scheduled_callback = Rc::clone(&scheduled_callback);
                move || {
                    if let Some(scheduled) = scheduled_callback.borrow_mut().take() {
                        scheduled();
                    }
                }
            },
            VISIBLE_CHANNEL_READ_FLUSH_DELAY,
        );

        if schedule_result.is_err() {
            if let Some(scheduled) = scheduled_callback.borrow_mut().take() {
                scheduled();
            } else {
                immediate();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn schedule_visible_channel_read_flush(
        _scheduled: impl FnOnce() + 'static,
        immediate: impl FnOnce() + 'static,
    ) {
        immediate();
    }
}

pub fn provide_chat() {
    let user = expect_context::<AuthContext>().user;
    let api = expect_context::<ApiRequestsProvider>().0;
    let chat = Chat::new(user, api);
    provide_context(chat);
    Effect::watch(
        move || {
            (
                chat.user
                    .with(|account| account.as_ref().map(|account| account.user.uid)),
                chat.block_list_version.get(),
            )
        },
        move |(user_id, _), previous, _| {
            let user_id = *user_id;
            let user_changed = match previous {
                Some(previous) => previous.0 != user_id,
                None => true,
            };

            if user_changed || user_id.is_none() {
                chat.blocked_user_ids.set(HashSet::new());
                chat.pending_outgoing_messages.set(Vec::new());
                chat.chat_send_errors.set(HashMap::new());
                chat.loaded_history_channels.set(HashSet::new());
                chat.messages_hub_data.set(None);
                chat.messages_hub_error.set(None);
                chat.messages_hub_loading.set(false);
                if user_id.is_none() {
                    chat.unread_counts.set(HashMap::new());
                }
            }

            if user_id.is_some() {
                chat.refresh_blocked_user_ids();
                if user_changed || chat.messages_hub_data.get_untracked().is_none() {
                    chat.refresh_messages_hub();
                }
            }
        },
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::{filter_duplicate_history_messages, filter_duplicate_live_messages, Chat};
    use chrono::{TimeZone, Utc};
    use leptos::prelude::*;
    use shared_types::{ChatMessage, ConversationKey, TournamentId};
    use uuid::Uuid;

    fn message(user_id: Uuid, body: &str, timestamp_millis: Option<i64>) -> ChatMessage {
        ChatMessage {
            user_id,
            username: "tester".to_string(),
            timestamp: timestamp_millis
                .map(|millis| Utc.timestamp_millis_opt(millis).single().unwrap()),
            message: body.to_string(),
            turn: Some(5),
        }
    }

    #[test]
    fn live_dedupe_keeps_repeated_message_with_different_timestamp() {
        let user_id = Uuid::new_v4();
        let existing = vec![message(user_id, "gg", Some(1_000))];
        let incoming = vec![message(user_id, "gg", Some(1_500))];
        let result = filter_duplicate_live_messages(&existing, incoming);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn live_dedupe_removes_exact_duplicate() {
        let user_id = Uuid::new_v4();
        let existing_message = message(user_id, "gg", Some(1_000));
        let existing = vec![existing_message.clone()];
        let incoming = vec![existing_message];
        let result = filter_duplicate_live_messages(&existing, incoming);
        assert!(result.is_empty());
    }

    #[test]
    fn history_dedupe_removes_duplicate_with_small_timestamp_skew() {
        let user_id = Uuid::new_v4();
        let existing = vec![message(user_id, "gg", Some(1_000))];
        let incoming = vec![message(user_id, "gg", Some(1_220))];
        let result = filter_duplicate_history_messages(&existing, incoming);
        assert!(result.is_empty());
    }

    #[test]
    fn history_dedupe_keeps_distinct_repeated_messages_during_reconnect_merge() {
        let user_id = Uuid::new_v4();
        let existing = vec![
            message(user_id, "gg", Some(1_000)),
            message(user_id, "gg", Some(3_000)),
        ];
        let incoming_unique = message(user_id, "gg", Some(2_000));
        let incoming_duplicate = message(user_id, "gg", Some(3_000));

        let result = filter_duplicate_history_messages(
            &existing,
            vec![incoming_unique.clone(), incoming_duplicate],
        );

        assert_eq!(result, vec![incoming_unique]);
    }

    #[test]
    fn failed_send_restores_draft_when_failed_channel_is_still_visible() {
        let owner = Owner::new();
        owner.set();

        let chat = Chat::new(
            Signal::derive(|| None),
            Signal::derive(|| panic!("api is not used in this test")),
        );
        let failed_key = ConversationKey::tournament(&TournamentId("visible-thread".to_string()));

        chat.queue_pending_outgoing_message(failed_key.clone(), "retry me".to_string(), None);
        chat.set_channel_visible(&failed_key);

        chat.handle_failed_chat_send(Some(failed_key.clone()), "send failed".to_string());

        assert_eq!(chat.typed_message.get_untracked(), "retry me");
        assert_eq!(
            chat.chat_send_error(&failed_key),
            Some("send failed".to_string())
        );
    }

    #[test]
    fn failed_send_does_not_restore_draft_into_different_visible_thread() {
        let owner = Owner::new();
        owner.set();

        let chat = Chat::new(
            Signal::derive(|| None),
            Signal::derive(|| panic!("api is not used in this test")),
        );
        let failed_key = ConversationKey::tournament(&TournamentId("failed-thread".to_string()));
        let active_key = ConversationKey::tournament(&TournamentId("active-thread".to_string()));

        chat.queue_pending_outgoing_message(failed_key.clone(), "retry me".to_string(), None);
        chat.set_channel_visible(&active_key);

        chat.handle_failed_chat_send(Some(failed_key.clone()), "send failed".to_string());

        assert!(chat.typed_message.get_untracked().is_empty());
        assert_eq!(
            chat.chat_send_error(&failed_key),
            Some("send failed".to_string())
        );
    }

}
