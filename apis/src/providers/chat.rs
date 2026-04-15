use crate::{
    chat::ChannelKey,
    functions::blocks_mutes::get_blocked_user_ids,
    functions::chat::{get_chat_history, get_chat_unread_counts, mark_chat_read},
    responses::AccountResponse,
};

use super::{
    api_requests::ApiRequests,
    auth_context::AuthContext,
    AlertType,
    AlertsContext,
    ApiRequestsProvider,
};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{
    other_user_from_dm_channel,
    ChannelType,
    ChatDestination,
    ChatMessage,
    ChatMessageContainer,
    GameId,
    TournamentId,
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const RECENT_ANNOUNCEMENTS_LIMIT: usize = 3;
const HISTORY_TIMESTAMP_SKEW_MILLIS: i64 = 500;
const MAX_STORED_MESSAGES_PER_CHANNEL: usize = 200;
const MAX_STORED_CHANNELS_PER_SECTION: usize = 128;

fn last_message_timestamp(messages: &[ChatMessage]) -> i64 {
    messages
        .last()
        .and_then(|message| message.timestamp.map(|timestamp| timestamp.timestamp_millis()))
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
    /// Stable shared block list for chat-adjacent surfaces.
    pub blocked_user_ids: RwSignal<HashSet<Uuid>>,
    /// Server-backed unread counts: (channel_type, channel_id, count). Refreshed via refresh_unread_counts().
    pub unread_counts: RwSignal<Vec<(String, String, i64)>>,
    /// Channels currently marked read optimistically; stale refreshes should not reintroduce unread.
    pending_read_channels: RwSignal<HashSet<ChannelKey>>,
    /// Channels currently visible in the UI. Used to suppress unread bumps for live messages in open threads.
    visible_channels: RwSignal<HashMap<ChannelKey, usize>>,
    /// Visible channels with a debounced read flush already scheduled.
    pending_visible_channel_reads: RwSignal<HashSet<ChannelKey>>,
    /// Live unread that arrived while a channel was visible and is waiting for the debounced
    /// read flush to either confirm read or restore unread if the channel closes first.
    deferred_visible_unread_counts: RwSignal<HashMap<ChannelKey, i64>>,
    /// Bump to invalidate conversation list (Messages hub sidebar). Resource key in messages.rs.
    pub conversation_list_version: RwSignal<u32>,
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
            blocked_user_ids: RwSignal::new(HashSet::new()),
            unread_counts: RwSignal::new(Vec::new()),
            pending_read_channels: RwSignal::new(HashSet::new()),
            visible_channels: RwSignal::new(HashMap::new()),
            pending_visible_channel_reads: RwSignal::new(HashSet::new()),
            deferred_visible_unread_counts: RwSignal::new(HashMap::new()),
            conversation_list_version: RwSignal::new(0),
            block_list_version: RwSignal::new(0),
            user,
            api,
        }
    }

    /// Bump the Messages hub conversation resource key so the sidebar refetches.
    pub fn invalidate_conversation_list(&self) {
        self.conversation_list_version.update(|v| *v += 1);
    }

    pub fn invalidate_block_list(&self) {
        self.block_list_version.update(|v| *v += 1);
    }

    fn remove_channel_keys(&self, keys: impl IntoIterator<Item = ChannelKey>) {
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
        let Some(current_user_id) = self.user.get_untracked().as_ref().map(|a| a.user.uid) else {
            return;
        };
        self.remove_channel_keys(
            removed_user_ids
                .into_iter()
                .map(|other_user_id| ChannelKey::direct(current_user_id, other_user_id)),
        );
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
                .map(|tournament_id| ChannelKey::tournament(&tournament_id)),
        );
    }

    fn prune_game_threads(&self, channel_type: ChannelType) {
        let mut removed_game_ids = Vec::new();
        match channel_type {
            ChannelType::GamePlayers => {
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
            ChannelType::GameSpectators => {
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
            _ => return,
        }

        if removed_game_ids.is_empty() {
            return;
        }

        self.remove_channel_keys(removed_game_ids.into_iter().map(|game_id| match channel_type {
            ChannelType::GamePlayers => ChannelKey::game_players(&game_id),
            ChannelType::GameSpectators => ChannelKey::game_spectators(&game_id),
            _ => unreachable!(),
        }));
    }

    async fn fetch_and_store_unread_counts(self) {
        if let Ok(counts) = get_chat_unread_counts().await { self.apply_server_unread_counts(counts) }
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
    pub fn apply_server_unread_counts(&self, counts: Vec<(String, String, i64)>) {
        let merged = self.merge_server_counts_with_optimistic(counts);
        self.unread_counts.set(merged);
    }

    /// Mark a channel as read on the server (fire-and-forget).
    /// Optimistically zeros the count locally so badges update immediately.
    pub fn mark_read(&self, key: &ChannelKey) {
        self.optimistically_clear_unread(key);
        let mark_key = key.clone();
        self.pending_read_channels.update(|pending| {
            pending.insert(mark_key.clone());
        });
        let chat = *self;
        spawn_local(async move {
            let did_mark =
                mark_chat_read(mark_key.channel_type.to_string(), mark_key.channel_id.clone())
                    .await
                    .is_ok();
            if !did_mark {
                chat.pending_read_channels.update(|pending| {
                    pending.remove(&mark_key);
                });
                chat.fetch_and_store_unread_counts().await;
            }
        });
    }

    pub fn set_channel_visible(&self, key: &ChannelKey) {
        self.visible_channels.update(|visible| {
            *visible.entry(key.clone()).or_insert(0) += 1;
        });
    }

    pub fn clear_channel_visible(&self, key: &ChannelKey) {
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

    fn is_channel_visible(&self, key: &ChannelKey) -> bool {
        self.visible_channels
            .with_untracked(|visible| visible.get(key).copied().unwrap_or(0) > 0)
    }

    fn flush_visible_channel_read(&self, key: &ChannelKey) {
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

    fn clear_visible_channel_read_flush(&self, key: &ChannelKey) {
        self.pending_visible_channel_reads.update(|pending| {
            pending.remove(key);
        });
        self.clear_deferred_visible_unread(key);
    }

    fn schedule_visible_channel_read_flush(&self, key: &ChannelKey) {
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

    fn defer_visible_channel_unread(&self, key: &ChannelKey) {
        self.deferred_visible_unread_counts.update(|pending| {
            pending
                .entry(key.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        });
        self.schedule_visible_channel_read_flush(key);
    }

    fn clear_deferred_visible_unread(&self, key: &ChannelKey) {
        self.deferred_visible_unread_counts.update(|pending| {
            pending.remove(key);
        });
    }

    fn take_deferred_visible_unread(&self, key: &ChannelKey) -> i64 {
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

    fn restore_deferred_visible_unread(&self, key: &ChannelKey) {
        let deferred = self.take_deferred_visible_unread(key);
        if deferred == 0 {
            return;
        }
        match key.channel_type {
            ChannelType::Direct => {
                let Some(other_user_id) = self.other_user_id_from_direct_key(key) else {
                    return;
                };
                self.users_new_messages.update(|messages| {
                    messages
                        .entry(other_user_id)
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ChannelType::TournamentLobby => {
                self.tournament_lobby_new_messages.update(|messages| {
                    messages
                        .entry(TournamentId(key.channel_id.clone()))
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ChannelType::GamePlayers => {
                self.games_private_new_messages.update(|messages| {
                    messages
                        .entry(GameId(key.channel_id.clone()))
                        .and_modify(|value| *value = true)
                        .or_insert(true);
                });
                self.optimistically_increment_unread_by(key, deferred);
            }
            ChannelType::Global => {}
            ChannelType::GameSpectators => {}
        }
    }

    /// Optimistically set unread count for channel(s) to 0 so badges update immediately.
    fn optimistically_clear_unread(&self, key: &ChannelKey) {
        self.unread_counts.update(|counts| {
            for (_, _, n) in counts
                .iter_mut()
                .filter(|(ct, cid, _)| ct == key.channel_type.as_str() && cid == &key.channel_id)
            {
                *n = 0;
            }
        });
    }

    /// Optimistically increment unread count when a live message arrives so badges update immediately.
    fn optimistically_increment_unread(&self, key: &ChannelKey) {
        self.optimistically_increment_unread_by(key, 1);
    }

    fn optimistically_increment_unread_by(&self, key: &ChannelKey, delta: i64) {
        if delta <= 0 {
            return;
        }
        self.pending_read_channels.update(|pending| {
            pending.remove(key);
        });
        self.unread_counts.update(|counts| {
            if let Some((_, _, n)) = counts
                .iter_mut()
                .find(|(ct, cid, _)| ct == key.channel_type.as_str() && cid == &key.channel_id)
            {
                *n += delta;
            } else {
                counts.push((key.channel_type.to_string(), key.channel_id.clone(), delta));
            }
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
        let players_key = ChannelKey::game_players(game_id);
        let spectators_key = ChannelKey::game_spectators(game_id);

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
        self.unread_counts.update(|counts| {
            counts.retain(|(channel_type, channel_id, _)| {
                channel_id != &game_id.0
                    || !matches!(
                        channel_type.as_str(),
                        CHANNEL_TYPE_GAME_PLAYERS | CHANNEL_TYPE_GAME_SPECTATORS
                    )
            });
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

    fn other_user_id_from_direct_key(&self, key: &ChannelKey) -> Option<Uuid> {
        let current_user_id = self.user.get_untracked().as_ref().map(|a| a.user.uid)?;
        other_user_from_dm_channel(&key.channel_id, current_user_id)
    }

    fn clear_local_new_for_channel(&self, key: &ChannelKey) {
        match key.channel_type {
            ChannelType::Direct => {
                if let Some(other_user_id) = self.other_user_id_from_direct_key(key) {
                    self.clear_dm_new_messages(other_user_id);
                }
            }
            ChannelType::TournamentLobby => {
                self.clear_tournament_lobby_new_messages(&TournamentId(key.channel_id.clone()));
            }
            ChannelType::GamePlayers | ChannelType::GameSpectators => {
                self.seen_messages(GameId(key.channel_id.clone()));
            }
            ChannelType::Global => {}
        }
    }

    pub fn open_channel(&self, key: &ChannelKey) {
        self.clear_local_new_for_channel(key);
        if key.channel_type != ChannelType::Global {
            self.mark_read(key);
        }
    }

    fn refresh_conversation_list_for_live_thread_activity(&self, is_live: bool) {
        // Keep catalog invalidation narrow on purpose: the Messages hub favors snappy local
        // updates over broad metadata refreshes, even if some selected-row details lag behind.
        if is_live {
            self.invalidate_conversation_list();
        }
    }

    /// Merge server counts with local "new" flags so optimistic unread is not overwritten by stale server state (e.g. 0 before message is persisted).
    fn merge_server_counts_with_optimistic(
        &self,
        server: Vec<(String, String, i64)>,
    ) -> Vec<(String, String, i64)> {
        let mut map: HashMap<ChannelKey, i64> = server
            .into_iter()
            .filter_map(|(channel_type, channel_id, count)| {
                ChannelKey::from_raw(&channel_type, channel_id).map(|key| (key, count))
            })
            .collect();
        let server_map = map.clone();
        let me = self.user.get_untracked().as_ref().map(|a| a.user.uid);
        self.users_new_messages.with_untracked(|m| {
            for (other_id, &has_new) in m.iter() {
                if has_new {
                    if let Some(current_id) = me {
                        map.entry(ChannelKey::direct(current_id, *other_id))
                            .and_modify(|n| *n = (*n).max(1))
                            .or_insert(1);
                    }
                }
            }
        });
        self.tournament_lobby_new_messages.with_untracked(|m| {
            for (tid, &has_new) in m.iter() {
                if has_new {
                    map.entry(ChannelKey::tournament(tid))
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
            }
        });
        self.games_private_new_messages.with_untracked(|m| {
            for (gid, &has_new) in m.iter() {
                if has_new {
                    map.entry(ChannelKey::game_players(gid))
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
            }
        });
        let pending_keys: Vec<ChannelKey> = self
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
        map.into_iter()
            .map(|(key, count)| (key.channel_type.to_string(), key.channel_id, count))
            .collect()
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
            .with(|counts| counts.iter().map(|(_, _, n)| n).sum::<i64>())
    }

    /// Unread count for a game (players channel only). Use for game list badges.
    /// Spectator messages intentionally do not contribute to badges/notifications.
    /// If local "new" flag is set, returns at least 1 so badge is not lost.
    pub fn unread_count_for_game(&self, game_id: &GameId) -> i64 {
        let from_list = self.unread_count_for_channel(&ChannelKey::game_players(game_id));
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
        let from_list = self.unread_count_for_channel(&ChannelKey::tournament(tournament_id));
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
    pub fn unread_count_for_dm(&self, other_user_id: Uuid, current_user_id: Uuid) -> i64 {
        let from_list =
            self.unread_count_for_channel(&ChannelKey::direct(current_user_id, other_user_id));
        let has_local_new = self
            .users_new_messages
            .with(|m| m.get(&other_user_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    pub fn unread_count_for_channel(&self, key: &ChannelKey) -> i64 {
        self.unread_counts.with(|counts| {
            counts
                .iter()
                .find(|(channel_type, channel_id, _)| {
                    channel_type == key.channel_type.as_str() && channel_id == &key.channel_id
                })
                .map(|(_, _, n)| *n)
                .unwrap_or(0)
        })
    }

    /// Replaces the cached history for a channel with a fresh server snapshot.
    /// Used when the server-side view of a thread changes and stale local messages
    /// should not be merged back in.
    pub fn replace_history(&self, key: &ChannelKey, mut messages: Vec<ChatMessage>) {
        let current_user_id = self.user.get_untracked().as_ref().map(|a| a.user.uid);
        messages.sort_by_key(|m| m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0));
        trim_stored_messages(&mut messages);

        match key.channel_type {
            ChannelType::Direct => {
                let Some(me) = current_user_id else { return };
                let Some(other_id) = other_user_from_dm_channel(&key.channel_id, me) else {
                    return;
                };
                self.users_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(&other_id);
                    } else {
                        stored.insert(other_id, messages);
                    }
                });
                self.prune_dm_threads();
            }
            ChannelType::TournamentLobby => {
                let tournament_id = TournamentId(key.channel_id.clone());
                self.tournament_lobby_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(&tournament_id);
                    } else {
                        stored.insert(tournament_id, messages);
                    }
                });
                self.prune_tournament_threads();
            }
            ChannelType::GamePlayers => {
                let game_id = GameId(key.channel_id.clone());
                self.games_private_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(&game_id);
                    } else {
                        stored.insert(game_id, messages);
                    }
                });
                self.prune_game_threads(ChannelType::GamePlayers);
            }
            ChannelType::GameSpectators => {
                let game_id = GameId(key.channel_id.clone());
                self.games_public_messages.update(|stored| {
                    if messages.is_empty() {
                        stored.remove(&game_id);
                    } else {
                        stored.insert(game_id, messages);
                    }
                });
                self.prune_game_threads(ChannelType::GameSpectators);
            }
            ChannelType::Global => {
                retain_recent_announcements(&mut messages);
                self.global_messages.set(messages);
            }
        }
    }

    /// Injects fetched history into the correct in-memory map so the thread view can display it.
    /// Merges with existing messages and deduplicates to avoid losing WebSocket messages when REST
    /// fetch completes after live delivery.
    pub fn inject_history(&self, key: &ChannelKey, messages: Vec<ChatMessage>) {
        let current_user_id = self.user.get_untracked().as_ref().map(|a| a.user.uid);
        match key.channel_type {
            ChannelType::Direct => {
                let Some(me) = current_user_id else { return };
                let other = other_user_from_dm_channel(&key.channel_id, me);
                if let Some(other_id) = other {
                    self.users_messages.update(|m| {
                        let entry = m.entry(other_id).or_default();
                        let existing = std::mem::take(entry);
                        let mut merged = merge_and_dedupe(existing, messages);
                        trim_stored_messages(&mut merged);
                        *entry = merged;
                    });
                    self.prune_dm_threads();
                }
            }
            ChannelType::TournamentLobby => {
                let tid = TournamentId(key.channel_id.clone());
                self.tournament_lobby_messages.update(|m| {
                    let entry = m.entry(tid).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
                self.prune_tournament_threads();
            }
            ChannelType::GamePlayers => {
                let gid = GameId(key.channel_id.clone());
                self.games_private_messages.update(|m| {
                    let entry = m.entry(gid).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
                self.prune_game_threads(ChannelType::GamePlayers);
            }
            ChannelType::GameSpectators => {
                let gid = GameId(key.channel_id.clone());
                self.games_public_messages.update(|m| {
                    let entry = m.entry(gid).or_default();
                    let existing = std::mem::take(entry);
                    let mut merged = merge_and_dedupe(existing, messages);
                    trim_stored_messages(&mut merged);
                    *entry = merged;
                });
                self.prune_game_threads(ChannelType::GameSpectators);
            }
            ChannelType::Global => {
                self.global_messages.update(|existing| {
                    *existing = merge_and_dedupe(std::mem::take(existing), messages);
                    retain_recent_announcements(existing);
                });
            }
        }
    }

    /// Loads channel history through the server-fn path and returns messages without injecting them.
    pub fn fetch_channel_history(
        &self,
        key: &ChannelKey,
    ) -> impl std::future::Future<Output = Result<Vec<ChatMessage>, String>> + '_ {
        let key = key.clone();
        async move {
            let history_limit = if key.channel_type == ChannelType::Global {
                RECENT_ANNOUNCEMENTS_LIMIT as i64
            } else {
                100
            };
            get_chat_history(
                key.channel_type.to_string(),
                key.channel_id.clone(),
                Some(history_limit),
            )
            .await
            .map_err(|error| error.to_string())
        }
    }

    pub fn send(&self, message: &str, destination: ChatDestination, turn: Option<usize>) {
        if matches!(
            &destination,
            ChatDestination::User((_, _)) | ChatDestination::TournamentLobby(_)
        ) {
            self.invalidate_conversation_list();
        }
        self.user.with_untracked(|a| {
            if let Some(account) = a {
                let id = account.user.uid;
                let name = account.user.username.clone();
                let turn = match destination {
                    ChatDestination::GamePlayers(_, _, _)
                    | ChatDestination::GameSpectators(_, _, _) => turn,
                    _ => None,
                };
                let msg = ChatMessage::new(name, id, message, None, turn);
                let container = ChatMessageContainer::new(destination, &msg);
                self.api.get_untracked().chat(&container);
            }
        });
    }

    pub fn recv(&mut self, containers: &[ChatMessageContainer]) {
        if let Some(last_message) = containers.last() {
            let is_live = containers.len() == 1;
            let from_self = self
                .user
                .get_untracked()
                .as_ref()
                .is_some_and(|a| last_message.message.user_id == a.user.uid);
            match &last_message.destination {
                ChatDestination::TournamentLobby(id) => {
                    let channel_key = ChannelKey::tournament(id);
                    let new_messages: Vec<ChatMessage> =
                        self.tournament_lobby_messages.with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_live_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.tournament_lobby_messages.update(|tournament| {
                        let entry = tournament.entry(id.clone()).or_default();
                        entry.extend(new_messages);
                        trim_stored_messages(entry);
                    });
                    self.prune_tournament_threads();
                    self.refresh_conversation_list_for_live_thread_activity(is_live);
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
                    let channel_key = current_user_id
                        .map(|current_id| ChannelKey::direct(current_id, thread_other_id));
                    let new_messages: Vec<ChatMessage> =
                        self.users_messages.with_untracked(|messages| {
                            let existing = messages
                                .get(&thread_other_id)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]);
                            filter_duplicate_live_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.users_messages.update(|users| {
                        let entry = users.entry(thread_other_id).or_default();
                        entry.extend(new_messages);
                        trim_stored_messages(entry);
                    });
                    self.prune_dm_threads();
                    self.refresh_conversation_list_for_live_thread_activity(is_live);
                    if is_live && !from_self {
                        if let Some(channel_key) = channel_key {
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
                }
                ChatDestination::GamePlayers(id, ..) => {
                    let channel_key = ChannelKey::game_players(id);
                    let new_messages: Vec<ChatMessage> =
                        self.games_private_messages.with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_live_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.games_private_messages.update(|games| {
                        let entry = games.entry(id.clone()).or_default();
                        entry.extend(new_messages);
                        trim_stored_messages(entry);
                    });
                    self.prune_game_threads(ChannelType::GamePlayers);
                    self.refresh_conversation_list_for_live_thread_activity(is_live);
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
                ChatDestination::GameSpectators(id, ..) => {
                    let channel_key = ChannelKey::game_spectators(id);
                    let new_messages: Vec<ChatMessage> =
                        self.games_public_messages.with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_live_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.games_public_messages.update(|games| {
                        let entry = games.entry(id.clone()).or_default();
                        entry.extend(new_messages);
                        trim_stored_messages(entry);
                    });
                    self.prune_game_threads(ChannelType::GameSpectators);
                    self.refresh_conversation_list_for_live_thread_activity(is_live);
                    if is_live && !from_self && self.is_channel_visible(&channel_key) {
                        self.seen_messages(id.clone());
                        self.schedule_visible_channel_read_flush(&channel_key);
                        // Spectator chat intentionally does not contribute to unread badges/notifications.
                    }
                }
                ChatDestination::Global => {
                    let to_add = self.global_messages.with_untracked(|msgs| {
                        filter_duplicate_live_messages(
                            msgs,
                            containers.iter().map(|c| c.message.clone()),
                        )
                    });
                    if !to_add.is_empty() {
                        self.global_messages.update(|m| {
                            m.extend(to_add);
                            retain_recent_announcements(m);
                        });
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
                chat.user.with(|account| account.as_ref().map(|account| account.user.uid)),
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
            }

            if user_id.is_some() {
                chat.refresh_blocked_user_ids();
            }
        },
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::{
        Chat,
        MAX_STORED_CHANNELS_PER_SECTION,
        filter_duplicate_history_messages,
        filter_duplicate_live_messages,
        merge_and_dedupe,
    };
    use chrono::{TimeZone, Utc};
    use leptos::prelude::*;
    use shared_types::{ChatMessage, TournamentId, CHANNEL_TYPE_TOURNAMENT_LOBBY};
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
    fn history_dedupe_only_consumes_one_match_per_existing_message() {
        let user_id = Uuid::new_v4();
        let existing = vec![message(user_id, "gg", None)];
        let incoming_first = message(user_id, "gg", Some(1_000));
        let incoming_second = message(user_id, "gg", Some(2_000));
        let result = filter_duplicate_history_messages(
            &existing,
            vec![incoming_first, incoming_second.clone()],
        );
        assert_eq!(result, vec![incoming_second]);
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
    fn history_dedupe_prefers_exact_timestamp_match_when_order_is_inverted() {
        let user_id = Uuid::new_v4();
        let existing = vec![message(user_id, "gg", Some(1_000))];
        let newer_message = message(user_id, "gg", Some(4_000));
        let exact_duplicate = message(user_id, "gg", Some(1_000));

        let result = filter_duplicate_history_messages(
            &existing,
            vec![newer_message.clone(), exact_duplicate],
        );

        assert_eq!(result, vec![newer_message]);
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
    fn merge_preserves_millisecond_order_within_same_second() {
        let user_id = Uuid::new_v4();
        let existing = vec![message(user_id, "live", Some(1_950))];
        let incoming = vec![message(user_id, "history", Some(1_120))];

        let result = merge_and_dedupe(existing, incoming);
        let bodies: Vec<_> = result.into_iter().map(|m| m.message).collect();

        assert_eq!(bodies, vec!["history".to_string(), "live".to_string()]);
    }

    #[test]
    fn pruning_cached_threads_keeps_server_unread_counts() {
        let owner = Owner::new();
        owner.set();

        let chat = Chat::new(
            Signal::derive(|| None),
            Signal::derive(|| panic!("api is not used in this test")),
        );
        let user_id = Uuid::new_v4();
        let evicted_tournament = TournamentId("old-thread".to_string());

        chat.unread_counts.set(vec![(
            CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(),
            evicted_tournament.0.clone(),
            4,
        )]);
        chat.tournament_lobby_messages.update(|messages| {
            messages.insert(
                evicted_tournament.clone(),
                vec![message(user_id, "old", Some(1))],
            );
            for idx in 0..MAX_STORED_CHANNELS_PER_SECTION {
                messages.insert(
                    TournamentId(format!("recent-{idx}")),
                    vec![message(user_id, "recent", Some(1_000 + idx as i64))],
                );
            }
        });

        chat.prune_tournament_threads();

        assert!(
            chat.tournament_lobby_messages
                .with_untracked(|messages| !messages.contains_key(&evicted_tournament))
        );
        assert_eq!(chat.unread_count_for_tournament(&evicted_tournament), 4);
        assert_eq!(chat.total_unread_count(), 4);
    }
}
