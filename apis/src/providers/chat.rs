use crate::functions::chat::{get_chat_unread_counts, mark_chat_read};
use crate::responses::AccountResponse;

use super::{
    api_requests::ApiRequests,
    auth_context::AuthContext,
    AlertType,
    AlertsContext,
    ApiRequestsProvider,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
use shared_types::{
    canonical_dm_channel_id, ChatDestination, ChatMessage, ChatMessageContainer,
    CHANNEL_TYPE_DIRECT, CHANNEL_TYPE_GAME_PLAYERS, CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_TOURNAMENT_LOBBY, GameId, TournamentId,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Key for deduplicating messages when merging (timestamp ms, user_id, body).
fn message_dedup_key(m: &ChatMessage) -> (i64, Uuid, String) {
    let ts = m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0);
    (ts, m.user_id, m.message.clone())
}

/// True if m is effectively a duplicate of something in existing.
/// Uses exact (ts, user_id, message) match, and also (user_id, message) within 5s
/// to catch REST/WebSocket races where timestamps differ slightly.
fn is_duplicate_message(existing: &[ChatMessage], m: &ChatMessage) -> bool {
    let key = message_dedup_key(m);
    let seen_exact: HashSet<_> = existing.iter().map(message_dedup_key).collect();
    if seen_exact.contains(&key) {
        return true;
    }
    let incoming_ts_ms = m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0);
    existing.iter().any(|e| {
        e.user_id == m.user_id
            && e.message == m.message
            && {
                let existing_ts = e.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0);
                (incoming_ts_ms - existing_ts).abs() < 5000
            }
    })
}

/// Filter incoming messages to only those not already in existing.
fn filter_duplicate_messages(
    existing: &[ChatMessage],
    incoming: impl IntoIterator<Item = ChatMessage>,
) -> Vec<ChatMessage> {
    incoming
        .into_iter()
        .filter(|m| !is_duplicate_message(existing, m))
        .collect()
}

/// Merge existing and incoming, deduplicate by (timestamp_ms, user_id, message), sort by timestamp.
/// Used to avoid losing WebSocket messages when REST fetch completes after live delivery.
fn merge_and_dedupe(existing: Vec<ChatMessage>, incoming: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let new_only = filter_duplicate_messages(&existing, incoming);
    let mut merged: Vec<_> = existing.into_iter().chain(new_only).collect();
    merged.sort_by_key(|m| m.timestamp.map(|t| t.timestamp()).unwrap_or(0));
    merged
}

fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
    let parts: Vec<&str> = channel_id.split("::").collect();
    if parts.len() != 2 {
        return None;
    }
    let a: Uuid = parts[0].parse().ok()?;
    let b: Uuid = parts[1].parse().ok()?;
    if a == me {
        Some(b)
    } else if b == me {
        Some(a)
    } else {
        None
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
    /// Server-backed unread counts: (channel_type, channel_id, count). Refreshed via refresh_unread_counts().
    pub unread_counts: RwSignal<Vec<(String, String, i64)>>,
    /// Bump to invalidate conversation list (Messages hub sidebar). Resource key in messages.rs.
    pub conversation_list_version: RwSignal<u32>,
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
            unread_counts: RwSignal::new(Vec::new()),
            conversation_list_version: RwSignal::new(0),
            user,
            api,
        }
    }

    /// Call when a DM is received (or sent) so the Messages hub conversation list refetches.
    pub fn invalidate_conversation_list(&self) {
        self.conversation_list_version.update(|v| *v += 1);
    }

    /// True if there are local "new" messages or server-backed unread count for this game.
    pub fn has_messages(&self, game_id: GameId) -> bool {
        self.games_public_new_messages
            .with_untracked(|m| m.get(&game_id).is_some_and(|v| *v))
            || self.games_private_new_messages
                .with_untracked(|m| m.get(&game_id).is_some_and(|v| *v))
            || self.unread_count_for_game(&game_id) > 0
    }

    /// Mark a channel as read on the server (fire-and-forget). Also refreshes unread counts after.
    /// Optimistically zeros the count locally so badges update immediately.
    pub fn mark_read(&self, channel_type: &str, channel_id: &str) {
        self.optimistically_clear_unread(channel_type, channel_id);
        let channel_type = channel_type.to_string();
        let channel_id = channel_id.to_string();
        let unread_counts = self.unread_counts;
        spawn_local(async move {
            let _ = mark_chat_read(channel_type, channel_id).await;
            if let Ok(counts) = get_chat_unread_counts().await {
                unread_counts.set(counts);
            }
        });
    }

    /// Optimistically set unread count for channel(s) to 0 so badges update immediately.
    fn optimistically_clear_unread(&self, channel_type: &str, channel_id: &str) {
        self.unread_counts.update(|counts| {
            for (_, _, n) in counts
                .iter_mut()
                .filter(|(ct, cid, _)| ct.as_str() == channel_type && cid.as_str() == channel_id)
            {
                *n = 0;
            }
        });
    }

    /// Optimistically increment unread count when a live message arrives so badges update immediately.
    fn optimistically_increment_unread(&self, channel_type: &str, channel_id: &str) {
        self.unread_counts.update(|counts| {
            if let Some((_, _, n)) = counts
                .iter_mut()
                .find(|(ct, cid, _)| ct.as_str() == channel_type && cid.as_str() == channel_id)
            {
                *n += 1;
            } else {
                counts.push((
                    channel_type.to_string(),
                    channel_id.to_string(),
                    1,
                ));
            }
        });
    }

    /// Clear local "new" state for game chat and mark both game_players and game_spectators as read on the server.
    pub fn seen_messages(&self, game_id: GameId) {
        self.games_public_new_messages.update(|m| {
            m.entry(game_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
        self.games_private_new_messages.update(|m| {
            m.entry(game_id.clone()).and_modify(|b| *b = false).or_insert(false);
        });
        let nanoid = game_id.0.clone();
        self.mark_read(CHANNEL_TYPE_GAME_PLAYERS, &nanoid);
        self.mark_read(CHANNEL_TYPE_GAME_SPECTATORS, &nanoid);
    }

    /// Clear local "new" state for tournament lobby chat and mark as read on the server.
    pub fn seen_tournament_lobby(&self, tournament_id: TournamentId) {
        self.tournament_lobby_new_messages.update(|m| {
            m.entry(tournament_id.clone())
                .and_modify(|b| *b = false)
                .or_insert(false);
        });
        self.mark_read(CHANNEL_TYPE_TOURNAMENT_LOBBY, &tournament_id.0);
    }

    /// Clear local "new" state for DM and mark as read on the server. Requires current user id.
    pub fn seen_dm(&self, other_user_id: Uuid, current_user_id: Uuid) {
        self.users_new_messages.update(|m| {
            m.entry(other_user_id).and_modify(|b| *b = false).or_insert(false);
        });
        let channel_id = canonical_dm_channel_id(current_user_id, other_user_id);
        self.mark_read(CHANNEL_TYPE_DIRECT, &channel_id);
    }

    /// Merge server counts with local "new" flags so optimistic unread is not overwritten by stale server state (e.g. 0 before message is persisted).
    fn merge_server_counts_with_optimistic(
        &self,
        server: Vec<(String, String, i64)>,
    ) -> Vec<(String, String, i64)> {
        let mut map: HashMap<(String, String), i64> = server
            .into_iter()
            .map(|(ct, cid, n)| ((ct, cid), n))
            .collect();
        let me = self.user.get_untracked().as_ref().map(|a| a.user.uid);
        self.users_new_messages.with_untracked(|m| {
            for (other_id, &has_new) in m.iter() {
                if has_new {
                    if let Some(current_id) = me {
                        let cid = canonical_dm_channel_id(current_id, *other_id);
                        map.entry((CHANNEL_TYPE_DIRECT.to_string(), cid))
                            .and_modify(|n| *n = (*n).max(1))
                            .or_insert(1);
                    }
                }
            }
        });
        self.tournament_lobby_new_messages.with_untracked(|m| {
            for (tid, &has_new) in m.iter() {
                if has_new {
                    let key = (CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(), tid.0.clone());
                    map.entry(key).and_modify(|n| *n = (*n).max(1)).or_insert(1);
                }
            }
        });
        self.games_private_new_messages.with_untracked(|m| {
            for (gid, &has_new) in m.iter() {
                if has_new {
                    let key = (CHANNEL_TYPE_GAME_PLAYERS.to_string(), gid.0.clone());
                    map.entry(key).and_modify(|n| *n = (*n).max(1)).or_insert(1);
                }
            }
        });
        self.games_public_new_messages.with_untracked(|m| {
            for (gid, &has_new) in m.iter() {
                if has_new {
                    let key = (CHANNEL_TYPE_GAME_SPECTATORS.to_string(), gid.0.clone());
                    map.entry(key).and_modify(|n| *n = (*n).max(1)).or_insert(1);
                }
            }
        });
        map.into_iter().map(|((ct, cid), n)| (ct, cid, n)).collect()
    }

    /// Fetch unread counts from the server and update unread_counts signal.
    /// Merges with local "new" flags so that a just-received DM/tournament message is not overwritten with 0.
    pub fn refresh_unread_counts(&self) {
        let chat = *self;
        spawn_local(async move {
            if let Ok(counts) = get_chat_unread_counts().await {
                let merged = chat.merge_server_counts_with_optimistic(counts);
                chat.unread_counts.set(merged);
            }
        });
    }

    /// Total unread count across all channels (for Messages link badge).
    /// Uses with_untracked to allow safe calls from Effect callbacks and event handlers.
    pub fn total_unread_count(&self) -> i64 {
        self.unread_counts
            .with_untracked(|counts| counts.iter().map(|(_, _, n)| n).sum::<i64>())
    }

    /// Unread count for a game (players + spectators channels). Use for game list badges.
    /// Uses with_untracked to allow safe calls from Effect callbacks (non-reactive context).
    /// If local "new" flag is set for either channel, returns at least 1 so badge is not lost.
    pub fn unread_count_for_game(&self, game_id: &GameId) -> i64 {
        let from_list = self.unread_counts.with_untracked(|counts| {
            counts
                .iter()
                .filter(|(ct, cid, _)| {
                    (ct.as_str() == CHANNEL_TYPE_GAME_PLAYERS
                        || ct.as_str() == CHANNEL_TYPE_GAME_SPECTATORS)
                        && cid == &game_id.0
                })
                .map(|(_, _, n)| *n)
                .sum::<i64>()
        });
        let has_local_new = self.games_private_new_messages.with_untracked(|m| {
            m.get(game_id).copied().unwrap_or(false)
        }) || self
            .games_public_new_messages
            .with_untracked(|m| m.get(game_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    /// Unread count for a tournament lobby. Use for tournament page badge.
    /// Uses with_untracked to allow safe calls from Effect callbacks (non-reactive context).
    /// If local "new" flag is set, returns at least 1 so badge is not lost before server state is updated.
    pub fn unread_count_for_tournament(&self, tournament_id: &TournamentId) -> i64 {
        let from_list = self.unread_counts.with_untracked(|counts| {
            counts
                .iter()
                .filter(|(ct, cid, _)| {
                    ct.as_str() == CHANNEL_TYPE_TOURNAMENT_LOBBY && cid == &tournament_id.0
                })
                .map(|(_, _, n)| *n)
                .next()
                .unwrap_or(0)
        });
        let has_local_new = self.tournament_lobby_new_messages.with_untracked(|m| {
            m.get(tournament_id).copied().unwrap_or(false)
        });
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    /// Unread count for a DM with another user. Use for DM list badge.
    /// Uses with_untracked to allow safe calls from Effect callbacks (non-reactive context).
    /// If local "new" flag is set, returns at least 1 so badge is not lost before server state is updated.
    pub fn unread_count_for_dm(&self, other_user_id: Uuid, current_user_id: Uuid) -> i64 {
        let from_list = self.unread_counts.with_untracked(|counts| {
            let channel_id = canonical_dm_channel_id(current_user_id, other_user_id);
            counts
                .iter()
                .filter(|(ct, cid, _)| ct.as_str() == CHANNEL_TYPE_DIRECT && cid == &channel_id)
                .map(|(_, _, n)| *n)
                .next()
                .unwrap_or(0)
        });
        let has_local_new = self
            .users_new_messages
            .with_untracked(|m| m.get(&other_user_id).copied().unwrap_or(false));
        if has_local_new {
            from_list.max(1)
        } else {
            from_list
        }
    }

    /// Unread count for global chat.
    /// Uses with_untracked to allow safe calls from Effect callbacks (non-reactive context).
    pub fn unread_count_for_global(&self) -> i64 {
        self.unread_counts.with_untracked(|counts| {
            counts
                .iter()
                .find(|(ct, cid, _)| ct.as_str() == CHANNEL_TYPE_GLOBAL && cid.as_str() == CHANNEL_TYPE_GLOBAL)
                .map(|(_, _, n)| *n)
                .unwrap_or(0)
        })
    }

    /// Injects fetched history into the correct in-memory map so the thread view can display it.
    /// Merges with existing messages and deduplicates to avoid losing WebSocket messages when REST
    /// fetch completes after live delivery.
    pub fn inject_history(&self, channel_type: &str, channel_id: &str, messages: Vec<ChatMessage>) {
        let current_user_id = self.user.get_untracked().as_ref().map(|a| a.user.uid);
        match channel_type {
            CHANNEL_TYPE_DIRECT => {
                let Some(me) = current_user_id else { return };
                let other = other_user_from_dm_channel(channel_id, me);
                if let Some(other_id) = other {
                    self.users_messages.update(|m| {
                        let existing = m.get(&other_id).cloned().unwrap_or_default();
                        m.insert(other_id, merge_and_dedupe(existing, messages));
                    });
                }
            }
            CHANNEL_TYPE_TOURNAMENT_LOBBY => {
                let tid = TournamentId(channel_id.to_string());
                self.tournament_lobby_messages.update(|m| {
                    let existing = m.get(&tid).cloned().unwrap_or_default();
                    m.insert(tid, merge_and_dedupe(existing, messages));
                });
            }
            CHANNEL_TYPE_GAME_PLAYERS => {
                let gid = GameId(channel_id.to_string());
                self.games_private_messages.update(|m| {
                    let existing = m.get(&gid).cloned().unwrap_or_default();
                    m.insert(gid, merge_and_dedupe(existing, messages));
                });
            }
            CHANNEL_TYPE_GAME_SPECTATORS => {
                let gid = GameId(channel_id.to_string());
                self.games_public_messages.update(|m| {
                    let existing = m.get(&gid).cloned().unwrap_or_default();
                    m.insert(gid, merge_and_dedupe(existing, messages));
                });
            }
            CHANNEL_TYPE_GLOBAL => {
                self.global_messages.update(|existing| {
                    *existing = merge_and_dedupe(std::mem::take(existing), messages);
                });
            }
            _ => {}
        }
    }

    /// Fetches channel history from GET /api/v1/chat/channel and returns messages (or empty on error).
    /// Does not inject; call inject_history after if needed. Client-only (no-op on server).
    pub fn fetch_channel_history(
        &self,
        channel_type: &str,
        channel_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<ChatMessage>, String>> + '_ {
        let _ct = channel_type.to_string();
        let _cid = channel_id.to_string();
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            return Err("fetch not available on server".to_string());

            #[cfg(target_arch = "wasm32")]
            {
            let url = format!(
                "/api/v1/chat/channel?channel_type={}&channel_id={}&limit=100",
                urlencoding::encode(&_ct),
                urlencoding::encode(&_cid),
            );
            let req = window().fetch_with_str(&url);
            let resp_value = wasm_bindgen_futures::JsFuture::from(req)
                .await
                .map_err(|e| e.as_string().unwrap_or_else(|| "fetch failed".to_string()))?;
            let resp: web_sys::Response = resp_value
                .dyn_into()
                .map_err(|_| "expected Response".to_string())?;
            if !resp.ok() {
                return Err("Failed to load messages".to_string());
            }
            let text_promise = resp.text().map_err(|_| "text() failed".to_string())?;
            let text_js = wasm_bindgen_futures::JsFuture::from(text_promise)
                .await
                .map_err(|e| e.as_string().unwrap_or_else(|| "text await failed".to_string()))?;
            let text_str = text_js
                .as_string()
                .ok_or("expected string body".to_string())?;
            let json: serde_json::Value =
                serde_json::from_str(&text_str).map_err(|e| e.to_string())?;
            let data = json
                .get("data")
                .and_then(|d| d.as_array())
                .ok_or("Invalid response")?;
            let messages: Vec<ChatMessage> = data
                .iter()
                .filter_map(|m| {
                    let obj = m.as_object()?;
                    let username = obj.get("username")?.as_str()?.to_string();
                    let body = obj.get("body")?.as_str()?.to_string();
                    let sender_id = Uuid::parse_str(obj.get("sender_id")?.as_str()?).ok()?;
                    let created_at = obj
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc));
                    let turn = obj.get("turn").and_then(|v| v.as_i64()).map(|t| t as usize);
                    Some(ChatMessage {
                        user_id: sender_id,
                        username,
                        timestamp: created_at,
                        message: body,
                        turn,
                    })
                })
                .collect();
            Ok(messages)
            }
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
            match &last_message.destination {
                ChatDestination::TournamentLobby(id) => {
                    let new_messages: Vec<ChatMessage> = self.tournament_lobby_messages
                        .with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.tournament_lobby_messages.update(|tournament| {
                        tournament
                            .entry(id.clone())
                            .or_default()
                            .extend(new_messages);
                    });
                    // Only treat as "new" when this is a single live message from someone else.
                    let is_live = containers.len() == 1;
                    let from_self = self.user.get_untracked().as_ref().map_or(false, |a| {
                        last_message.message.user_id == a.user.uid
                    });
                    if is_live && !from_self {
                        self.tournament_lobby_new_messages.update(|m| {
                            m.entry(id.clone())
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                        self.optimistically_increment_unread(CHANNEL_TYPE_TOURNAMENT_LOBBY, &id.0);
                        self.invalidate_conversation_list();
                        // Refresh so UI re-renders and shows new messages (merge in refresh preserves optimistic badge).
                        self.refresh_unread_counts();
                    } else if !is_live {
                        self.seen_tournament_lobby(id.clone());
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
                    let new_messages: Vec<ChatMessage> = self.users_messages.with_untracked(|messages| {
                        let existing = messages.get(&thread_other_id).map(Vec::as_slice).unwrap_or(&[]);
                        filter_duplicate_messages(
                            existing,
                            containers.iter().map(|c| c.message.clone()),
                        )
                    });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.users_messages.update(|users| {
                        users.entry(thread_other_id).or_default().extend(new_messages);
                    });
                    let is_live = containers.len() == 1;
                    let from_self = self.user.get_untracked().as_ref().map_or(false, |a| {
                        last_message.message.user_id == a.user.uid
                    });
                    if is_live && !from_self {
                        self.users_new_messages.update(|m| {
                            m.entry(thread_other_id)
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                        if let Some(current_id) = current_user_id {
                            let channel_id = canonical_dm_channel_id(current_id, thread_other_id);
                            self.optimistically_increment_unread(CHANNEL_TYPE_DIRECT, &channel_id);
                        }
                        self.invalidate_conversation_list();
                        // Refresh so UI re-renders and shows new messages (merge in refresh preserves optimistic badge).
                        self.refresh_unread_counts();
                    } else if !is_live {
                        if let Some(current_id) = current_user_id {
                            self.seen_dm(thread_other_id, current_id);
                        }
                    }
                }
                ChatDestination::GamePlayers(id, ..) => {
                    let new_messages: Vec<ChatMessage> = self.games_private_messages
                        .with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.games_private_messages.update(|games| {
                        games.entry(id.clone()).or_default().extend(new_messages);
                    });
                    // Only treat as "new" (red tab) when this is a single live message from someone else.
                    let is_live = containers.len() == 1;
                    let from_self = self.user.get_untracked().as_ref().map_or(false, |a| {
                        last_message.message.user_id == a.user.uid
                    });
                    if is_live && !from_self {
                        self.games_private_new_messages.update(|m| {
                            m.entry(id.clone())
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                        self.optimistically_increment_unread(CHANNEL_TYPE_GAME_PLAYERS, &id.0);
                        self.refresh_unread_counts();
                    } else if !is_live {
                        self.seen_messages(id.clone());
                    }
                }
                ChatDestination::GameSpectators(id, ..) => {
                    let new_messages: Vec<ChatMessage> = self.games_public_messages
                        .with_untracked(|messages| {
                            let existing = messages.get(id).map(Vec::as_slice).unwrap_or(&[]);
                            filter_duplicate_messages(
                                existing,
                                containers.iter().map(|c| c.message.clone()),
                            )
                        });
                    if new_messages.is_empty() {
                        return;
                    }
                    self.games_public_messages.update(|games| {
                        games.entry(id.clone()).or_default().extend(new_messages);
                    });
                    // Only treat as "new" (red tab) when this is a single live message from someone else.
                    let is_live = containers.len() == 1;
                    let from_self = self.user.get_untracked().as_ref().map_or(false, |a| {
                        last_message.message.user_id == a.user.uid
                    });
                    if is_live && !from_self {
                        self.games_public_new_messages.update(|m| {
                            m.entry(id.clone())
                                .and_modify(|value| *value = true)
                                .or_insert(true);
                        });
                        self.optimistically_increment_unread(CHANNEL_TYPE_GAME_SPECTATORS, &id.0);
                        self.refresh_unread_counts();
                    } else if !is_live {
                        self.seen_messages(id.clone());
                    }
                }
                ChatDestination::Global => {
                    let to_add = self.global_messages.with_untracked(|msgs| {
                        filter_duplicate_messages(msgs, containers.iter().map(|c| c.message.clone()))
                    });
                    if !to_add.is_empty() {
                        self.global_messages.update(|m| m.extend(to_add));
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

pub fn provide_chat() {
    let user = expect_context::<AuthContext>().user;
    let api = expect_context::<ApiRequestsProvider>().0;
    provide_context(Chat::new(user, api))
}
