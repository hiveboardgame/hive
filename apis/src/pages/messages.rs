//! Messages hub: /messages — DMs, Tournaments, Games, Global. Supports ?dm=:uuid to open a DM from profile.

use crate::components::atoms::{block_button::BlockButton, unblock_button::UnblockButton};
use crate::components::organisms::chat::{messages_with_header_flags, ChatInput, Message};
use crate::components::molecules::time_row::TimeRow;
use crate::functions::blocks_mutes::{mute_tournament_chat, unmute_tournament_chat};
use crate::functions::chat::{get_my_chat_conversations, MyConversations};
use crate::functions::games::get::get_game_from_nanoid;
use crate::providers::{chat::Chat, AuthContext};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::html;
use leptos::leptos_dom::helpers::request_animation_frame;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_use::use_interval_fn;
use leptos_router::hooks::use_query_map;
use shared_types::{
    canonical_dm_channel_id, ChatDestination, ChatMessage, GameId, PrettyString, TimeInfo,
    TournamentId,
    CHANNEL_TYPE_DIRECT, CHANNEL_TYPE_GAME_PLAYERS, CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectedChannel {
    Dm(Uuid, String),
    Tournament(String, String, bool, bool), // nanoid, name, is_participant, muted
    Game(String, String, String, Uuid, Uuid, bool),
    Global,
}

impl SelectedChannel {
    fn channel_type_and_id(&self, me: Uuid) -> (String, String) {
        match self {
            SelectedChannel::Dm(other_id, _) => {
                (CHANNEL_TYPE_DIRECT.to_string(), canonical_dm_channel_id(me, *other_id))
            }
            SelectedChannel::Tournament(nanoid, _, _, _) => {
                (CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(), nanoid.clone())
            }
            SelectedChannel::Game(ct, cid, _, _, _, _) => (ct.clone(), cid.clone()),
            SelectedChannel::Global => (CHANNEL_TYPE_GLOBAL.to_string(), CHANNEL_TYPE_GLOBAL.to_string()),
        }
    }

    fn label(&self) -> String {
        match self {
            SelectedChannel::Dm(_, username) => username.clone(),
            SelectedChannel::Tournament(_, name, _, _) => name.clone(),
            SelectedChannel::Game(_, _, label, _, _, _) => label.clone(),
            SelectedChannel::Global => "Global".to_string(),
        }
    }

    fn to_destination(&self) -> ChatDestination {
        match self {
            SelectedChannel::Dm(other_id, username) => {
                ChatDestination::User((*other_id, username.clone()))
            }
            SelectedChannel::Tournament(nanoid, _, _, _) => {
                ChatDestination::TournamentLobby(TournamentId(nanoid.clone()))
            }
            SelectedChannel::Game(ct, cid, _, white_id, black_id, _) => {
                let game_id = GameId(cid.clone());
                if ct.as_str() == CHANNEL_TYPE_GAME_PLAYERS {
                    ChatDestination::GamePlayers(game_id, *white_id, *black_id)
                } else {
                    ChatDestination::GameSpectators(game_id, *white_id, *black_id)
                }
            }
            SelectedChannel::Global => ChatDestination::Global,
        }
    }
}

#[component]
pub fn Messages() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let current_user_id = move || auth.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid));

    let query_map = use_query_map();
    let dm_param = move || {
        query_map
            .get()
            .get("dm")
            .map(|s| s.to_string())
            .and_then(|s| Uuid::parse_str(&s).ok())
    };
    let dm_username_param = move || {
        query_map.get().get("username").map(|s| s.to_string()).unwrap_or_default()
    };

    let conversations = Resource::new(
        move || chat.conversation_list_version.get(),
        move |_| async move { get_my_chat_conversations().await },
    );
    let block_list_version = RwSignal::new(0);
    let block_list = Resource::new(
        move || block_list_version.get(),
        move |_| async move { crate::functions::blocks_mutes::get_blocked_user_ids().await },
    );
    // Keys of blocked-user messages that the user has expanded (so state survives re-renders).
    let expanded_hidden_messages = RwSignal::new(std::collections::HashSet::<(i64, Uuid, String)>::new());

    let (selected, set_selected) = signal::<Option<SelectedChannel>>(None);
    let (unread_at_open, set_unread_at_open) = signal::<Option<i64>>(None);
    // On mobile: true = show channel list (drawer open), false = show thread full width. Desktop always shows both.
    let mobile_drawer_open = RwSignal::new(true);
    let message_container_ref = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();

    // On mobile: close drawer when a conversation is selected (thread full width); open drawer when none selected.
    Effect::new(move |_| {
        if selected.get().is_some() {
            mobile_drawer_open.set(false);
        } else {
            mobile_drawer_open.set(true);
        }
    });

    // Preselect DM from ?dm= (and optional ?username=) when present; e.g. from profile "Message" link
    Effect::new(move |_| {
        let dm_uid = dm_param();
        if let Some(other_id) = dm_uid {
            let conv = conversations.get().and_then(Result::ok);
            let username_from_query = dm_username_param();
            let username = conv
                .as_ref()
                .and_then(|c| {
                    c.dms.iter().find(|d| d.other_user_id == other_id).map(|d| d.username.clone())
                })
                .or_else(|| {
                    if username_from_query.is_empty() {
                        None
                    } else {
                        Some(username_from_query)
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());
            set_selected.set(Some(SelectedChannel::Dm(other_id, username)));
        }
    });

    // When selected channel changes: capture unread (for scroll-to-first-unread), mark as read, then fetch history.
    // Only run when a channel IS selected — we do not mark-as-read or refresh when none selected, so optimistic
    // unread from a just-received DM/tournament message is not overwritten by server state (fixes feedback #1).
    // Scroll-to-first-unread only for DM, Game, Tournament — not Global.
    Effect::new(move |_| {
        let sel = selected.get();
        let me = match current_user_id() {
            Some(u) => u,
            None => return,
        };
        if let Some(ref ch) = sel {
            set_unread_at_open.set(None);
            let unread = match ch {
                SelectedChannel::Dm(other_id, _) => chat.unread_count_for_dm(*other_id, me),
                SelectedChannel::Tournament(nanoid, _, _, _) => {
                    chat.unread_count_for_tournament(&TournamentId(nanoid.clone()))
                }
                SelectedChannel::Game(_, cid, _, _, _, _) => chat.unread_count_for_game(&GameId(cid.clone())),
                SelectedChannel::Global => chat.unread_count_for_global(),
            };
            if unread > 0 {
                set_unread_at_open.set(Some(unread));
            }

            let (ct, cid) = ch.channel_type_and_id(me);
            let chat = chat.clone();
            let ct2 = ct.clone();
            let cid2 = cid.clone();

            // Mark as read immediately so the badge disappears right away
            match ch {
                SelectedChannel::Dm(other_id, _) => chat.seen_dm(*other_id, me),
                SelectedChannel::Tournament(nanoid, _, _, _) => {
                    chat.seen_tournament_lobby(TournamentId(nanoid.clone()))
                }
                SelectedChannel::Game(_, cid, _, _, _, _) => chat.seen_messages(GameId(cid.clone())),
                SelectedChannel::Global => chat.mark_read(&ct2, &cid2),
            }
            chat.refresh_unread_counts();

            // Fetch history: for finished games fetch both channels; otherwise fetch single channel
            let fetch_finished = matches!(ch, SelectedChannel::Game(_, _, _, _, _, true));
            let cid_for_fetch = cid2.clone();
            spawn_local(async move {
                if fetch_finished {
                    let cid = cid_for_fetch.clone();
                    if let Ok(m) = chat.fetch_channel_history(CHANNEL_TYPE_GAME_PLAYERS, &cid).await {
                        chat.inject_history(CHANNEL_TYPE_GAME_PLAYERS, &cid, m);
                    }
                    if let Ok(m) = chat.fetch_channel_history(CHANNEL_TYPE_GAME_SPECTATORS, &cid).await {
                        chat.inject_history(CHANNEL_TYPE_GAME_SPECTATORS, &cid, m);
                    }
                } else if let Ok(messages) = chat.fetch_channel_history(&ct2, &cid2).await {
                    chat.inject_history(&ct2, &cid2, messages);
                }
            });
        }
    });

    // When new messages arrive for the selected channel, mark as read so badge doesn't appear while viewing
    Effect::new(move |_| {
        let sel = selected.get();
        let me = match current_user_id() {
            Some(u) => u,
            None => return,
        };
        // Create reactive dependency on message count for the selected channel
        let _msg_count = match &sel {
            Some(SelectedChannel::Dm(other_id, _)) => chat
                .users_messages
                .get()
                .get(other_id)
                .map(|v| v.len())
                .unwrap_or(0),
            Some(SelectedChannel::Tournament(nanoid, _, _, _)) => chat
                .tournament_lobby_messages
                .get()
                .get(&TournamentId(nanoid.clone()))
                .map(|v| v.len())
                .unwrap_or(0),
            Some(SelectedChannel::Game(_ct, cid, _, _, _, _finished)) => {
                let gid = GameId(cid.clone());
                let private_len = chat
                    .games_private_messages
                    .get()
                    .get(&gid)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let public_len = chat
                    .games_public_messages
                    .get()
                    .get(&gid)
                    .map(|v| v.len())
                    .unwrap_or(0);
                private_len + public_len
            }
            Some(SelectedChannel::Global) => chat.global_messages.get().len(),
            None => 0,
        };
        if let Some(ref ch) = sel {
            match ch {
                SelectedChannel::Dm(other_id, _) => chat.seen_dm(*other_id, me),
                SelectedChannel::Tournament(nanoid, _, _, _) => {
                    chat.seen_tournament_lobby(TournamentId(nanoid.clone()))
                }
                SelectedChannel::Game(_, cid, _, _, _, _) => chat.seen_messages(GameId(cid.clone())),
                SelectedChannel::Global => {
                    let (ct, cid) = ch.channel_type_and_id(me);
                    chat.mark_read(&ct, &cid);
                }
            }
        }
    });

    // Memo ensures we re-render when message maps change (WebSocket recv, fetch complete).
    // Plain closure called in a block can miss updates due to reactive scope boundaries.
    let messages_list = Signal::derive(move || {
        let sel = selected.get()?;
        let me = current_user_id()?;
        let (ct, _cid) = sel.channel_type_and_id(me);
        let msgs: Vec<ChatMessage> = match &sel {
            SelectedChannel::Dm(other_id, _) => chat.users_messages.get().get(other_id).cloned().unwrap_or_default(),
            SelectedChannel::Tournament(nanoid, _, _, _) => chat
                .tournament_lobby_messages
                .get()
                .get(&TournamentId(nanoid.clone()))
                .cloned()
                .unwrap_or_default(),
            SelectedChannel::Game(_, cid, _, _, _, _) => {
                let gid = GameId(cid.clone());
                let private_msgs = chat.games_private_messages.get().get(&gid).cloned().unwrap_or_default();
                let public_msgs = chat.games_public_messages.get().get(&gid).cloned().unwrap_or_default();
                if ct == CHANNEL_TYPE_GAME_PLAYERS {
                    private_msgs
                } else {
                    public_msgs
                }
            }
            SelectedChannel::Global => chat.global_messages.get(),
        };
        Some(msgs)
    });

    let destination_signal = Signal::derive(move || {
        selected.get().map(|s| s.to_destination())
    });

    // Clear scroll-to-first-unread divider after 10 seconds
    use_interval_fn(
        move || {
            if unread_at_open.get_untracked().is_some() {
                set_unread_at_open.set(None);
            }
        },
        10_000,
    );

    // Scroll: on channel change / initial load → first unread (if any) or bottom. When a NEW message
    // arrives (same channel, count N→N+1) → scroll to bottom so the newest message is visible.
    // Throttle only for "new message" scrolls to avoid jank; never throttle channel change or 0→N load.
    let scroll_last_run = std::cell::RefCell::new(0_f64);
    Effect::watch(
        move || {
            let sel = selected.get();
            let count = messages_list
                .get()
                .as_ref()
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            (sel, count)
        },
        move |(sel, count), prev, _| {
            let (run, is_new_message) = match prev {
                None => (true, false),
                Some((prev_sel, prev_count)) => {
                    let channel_changed = sel != prev_sel;
                    let count_increased = *count > *prev_count;
                    // Only treat as "new message" when count increased on same channel and we already had messages (not initial load 0→N).
                    let is_new_message = count_increased && sel == prev_sel && *prev_count > 0;
                    (channel_changed || count_increased, is_new_message)
                }
            };
            if !run {
                return;
            }
            // Throttle only when scrolling due to a new message (N→N+1); allow channel change and initial load (0→N) every time.
            if is_new_message {
                let now = web_sys::js_sys::Date::now();
                if now - *scroll_last_run.borrow() < 300.0 {
                    return;
                }
                scroll_last_run.replace(now);
            }
            // Clone refs for the rAF closure; read them inside rAF so the DOM (and first-unread divider) are updated before we scroll.
            let container_ref = message_container_ref.clone();
            let first_unread_ref_clone = first_unread_ref.clone();
            request_animation_frame(move || {
                let container = container_ref.get_untracked();
                let target = first_unread_ref_clone.get_untracked();
                if is_new_message {
                    if let Some(c) = container {
                        c.set_scroll_top(c.scroll_height());
                    }
                } else if let Some(t) = target {
                    let _ = t.scroll_into_view_with_bool(true);
                } else if let Some(c) = container {
                    c.set_scroll_top(c.scroll_height());
                }
            });
        },
        true,
    );

    view! {
        <div class="fixed top-12 left-0 right-0 bottom-0 flex flex-col sm:flex-row overflow-hidden bg-gray-100 dark:bg-gray-950 z-0">
            <aside
                class=move || format!(
                    "w-full sm:w-72 flex-shrink-0 flex flex-col min-h-0 overflow-hidden bg-white dark:bg-gray-900 shadow-lg sm:rounded-r-xl border-l border-gray-200 dark:border-gray-700 {} sm:!flex",
                    if mobile_drawer_open.get() { "" } else { "hidden " }
                )
            >
                <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900">
                    <h1 class="text-xl font-bold text-gray-800 dark:text-gray-100 tracking-tight">
                        "Messages"
                    </h1>
                </div>
                <div class="flex-1 min-h-0 overflow-y-auto p-2 pb-6 sm:pb-2">
                    {move || {
                        let conv = conversations.get();
                        match conv {
                            Some(Ok(c)) => {
                                view! {
                                    <ChannelLists
                                        conv=c
                                        current_user_id=current_user_id
                                        selected=selected
                                        set_selected=set_selected
                                        chat=chat
                                    />
                                }
                                    .into_any()
                            }
                            Some(Err(_)) => view! { <p class="p-3 text-sm text-red-600 dark:text-red-400">"Failed to load conversations"</p> }.into_any(),
                            None => view! { <p class="p-3 text-sm text-gray-500 dark:text-gray-400 animate-pulse">"Loading…"</p> }.into_any(),
                        }
                    }}
                </div>
            </aside>
            <main
                class=move || format!(
                    "flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden {} sm:!flex",
                    if mobile_drawer_open.get() { "hidden " } else { "" }
                )
            >
                {move || {
                    if destination_signal.get().is_none() {
                        return view! {
                            <div class="flex-1 flex flex-col items-center justify-center text-gray-500 dark:text-gray-400 p-8 gap-2">
                                <span class="text-4xl opacity-50">"💬"</span>
                                <p class="text-center font-medium">"Select a conversation"</p>
                                <p class="text-sm text-center max-w-xs">"Choose a chat from the sidebar to start messaging."</p>
                            </div>
                        }
                            .into_any();
                    }
                    let mut msgs = messages_list.get().unwrap_or_default();
                    msgs.sort_by_key(|m| m.timestamp.map(|t| t.timestamp()).unwrap_or(0));
                    let empty_thread = msgs.is_empty();
                    let sel = selected.get();
                    let sel_for_messages = sel.clone();
                    let me_uid = current_user_id();
                    view! {
                        <div class="flex flex-col flex-1 min-h-0 overflow-hidden bg-white dark:bg-gray-900 sm:rounded-l-xl border-r border-gray-200 dark:border-gray-700 shadow-inner">
                            <div class="flex items-center gap-2 px-2 py-3 sm:px-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 shrink-0 min-h-[2.75rem]">
                                <button
                                    type="button"
                                    class="sm:hidden flex-shrink-0 flex items-center justify-center gap-1 min-h-[2.25rem] min-w-[2.25rem] -ml-1 rounded-lg text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
                                    aria-label="Back to conversations"
                                    on:click=move |_| mobile_drawer_open.set(true)
                                >
                                    <span class="text-lg" aria-hidden="true">"←"</span>
                                    <span class="text-sm font-medium">"Conversations"</span>
                                </button>
                                <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-100 truncate min-w-0 flex-1">
                                    {sel.as_ref().map(|s| s.label()).unwrap_or_default()}
                                </h2>
                            </div>
                            {move || {
                                let s = sel.clone();
                                match s.as_ref() {
                                    Some(SelectedChannel::Dm(other_id, username)) => {
                                        let other_id_val = *other_id;
                                        let block_list_ref = block_list.clone();
                                        let set_block_version = move || block_list_version.update(|v| *v += 1);
                                        let is_blocked = move || {
                                            block_list_ref.get().and_then(Result::ok).map_or(false, |ids: Vec<Uuid>| ids.contains(&other_id_val))
                                        };
                                        view! {
                                            <div class="px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50/80 dark:bg-gray-800/30 shrink-0 flex flex-wrap items-center gap-2">
                                                <a
                                                    href=format!("/@/{}", username)
                                                    class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                                                >
                                                    "View profile →"
                                                </a>
                                                <Show when=move || is_blocked()>
                                                    <UnblockButton
                                                        blocked_user_id=other_id_val
                                                        on_success=Callback::new(move |()| set_block_version())
                                                    />
                                                </Show>
                                                <Show when=move || !is_blocked()>
                                                    <BlockButton
                                                        blocked_user_id=other_id_val
                                                        on_success=Callback::new(move |()| set_block_version())
                                                    />
                                                </Show>
                                            </div>
                                        }.into_any()
                                    }
                                    Some(SelectedChannel::Game(ct, cid, label, white_id, black_id, finished)) => view! {
                                        <GameChatHeader
                                            channel_type=ct.clone()
                                            channel_id=cid.clone()
                                            label=label.clone()
                                            white_id=*white_id
                                            black_id=*black_id
                                            finished=*finished
                                            set_selected=set_selected
                                        />
                                    }.into_any(),
                                    Some(SelectedChannel::Tournament(nanoid, name, is_participant, muted)) => {
                                        let nanoid_clone = nanoid.clone();
                                        let name_clone = name.clone();
                                        let muted_val = *muted;
                                        let is_participant_val = *is_participant;
                                        let set_selected_mute = set_selected.clone();
                                        let chat_mute = chat.clone();
                                        view! {
                                            <div class="px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50/80 dark:bg-gray-800/30 shrink-0 flex flex-col gap-1">
                                                <div class="flex flex-wrap items-center gap-2">
                                                    <a
                                                        href=format!("/tournament/{}", nanoid)
                                                        class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                                                    >
                                                        "View tournament →"
                                                    </a>
                                                    <button
                                                        type="button"
                                                        class="text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                                                        on:click=move |_| {
                                                            let nanoid = nanoid_clone.clone();
                                                            let name = name_clone.clone();
                                                            let set_sel = set_selected_mute.clone();
                                                            let chat_ctx = chat_mute.clone();
                                                            let currently_muted = muted_val;
                                                            spawn_local(async move {
                                                                let new_muted = if currently_muted {
                                                                    let _ = unmute_tournament_chat(nanoid.clone()).await;
                                                                    false
                                                                } else {
                                                                    let _ = mute_tournament_chat(nanoid.clone()).await;
                                                                    true
                                                                };
                                                                chat_ctx.invalidate_conversation_list();
                                                                set_sel.set(Some(SelectedChannel::Tournament(nanoid, name, is_participant_val, new_muted)));
                                                            });
                                                        }
                                                    >
                                                        {if muted_val { "Unmute tournament chat" } else { "Mute tournament chat" }}
                                                    </button>
                                                </div>
                                                {if !is_participant_val {
                                                    view! { <p class="text-xs text-gray-500 dark:text-gray-400">"Only participants can chat here."</p> }.into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                    _ => view! {}.into_any(),
                                }
                            }}
                            <div node_ref=message_container_ref class="flex-1 overflow-y-auto overflow-x-hidden p-4 min-h-0">
                                {move || {
                                    if empty_thread {
                                        view! {
                                            <div class="flex flex-col items-center justify-center h-full min-h-[12rem] text-gray-500 dark:text-gray-400 gap-2">
                                                <span class="text-3xl opacity-40">"✉️"</span>
                                                <p class="text-sm font-medium">"No messages yet"</p>
                                                <p class="text-xs">"Send a message to start the conversation."</p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        let use_scroll_to_unread = selected.get().is_some();
                                        let n_unread = unread_at_open.get().unwrap_or(0) as usize;
                                        let _ = unread_at_open.get();
                                        let (read_msgs, unread_msgs, show_divider) = if use_scroll_to_unread && n_unread > 0 && n_unread <= msgs.len() {
                                            let split_idx = msgs.len() - n_unread;
                                            let (r, u) = msgs.split_at(split_idx);
                                            (r.to_vec(), u.to_vec(), unread_at_open.get().is_some())
                                        } else {
                                            (msgs.clone(), vec![], false)
                                        };
                                        let read_with_flags = messages_with_header_flags(&read_msgs);
                                        let unread_with_flags = messages_with_header_flags(&unread_msgs);
                                        let blocked_ids: std::collections::HashSet<Uuid> = block_list
                                            .get()
                                            .and_then(Result::ok)
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let blocked_ids2 = blocked_ids.clone();
                                        let is_dm = matches!(sel_for_messages, Some(SelectedChannel::Dm(_, _)));
                                        let expanded_set = expanded_hidden_messages;
                                        view! {
                                            <For each=move || read_with_flags.clone() key=|item| (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.message.clone()) let:item>
                                                {let is_me = me_uid.map(|u| item.0.user_id == u).unwrap_or(false);
                                                let sender_blocked = !is_dm && blocked_ids.contains(&item.0.user_id);
                                                let key = (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.user_id, item.0.message.clone());
                                                let key_cb = key.clone();
                                                let expanded_signal = Signal::derive(move || expanded_set.get().contains(&key));
                                                let on_expand = Callback::new(move |()| {
                                                    expanded_set.update(|s| { s.insert(key_cb.clone()); });
                                                });
                                                view! {
                                                    <Message message=item.0 is_current_user=is_me show_header=item.1 sender_blocked=sender_blocked expanded_signal=expanded_signal on_click_expand=on_expand />
                                                }}
                                            </For>
                                            {if show_divider {
                                                view! {
                                                    <div
                                                        node_ref=first_unread_ref
                                                        class="relative my-4 flex items-center justify-center text-xs text-gray-500 dark:text-gray-400"
                                                    >
                                                        <div class="absolute inset-x-0 border-b border-gray-300 dark:border-gray-600"></div>
                                                        <span class="relative z-10 bg-white dark:bg-gray-900 px-2">"New Messages"</span>
                                                    </div>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
                                            <For each=move || unread_with_flags.clone() key=|item| (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.message.clone()) let:item>
                                                {let is_me = me_uid.map(|u| item.0.user_id == u).unwrap_or(false);
                                                let sender_blocked = !is_dm && blocked_ids2.contains(&item.0.user_id);
                                                let key2 = (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.user_id, item.0.message.clone());
                                                let key_cb2 = key2.clone();
                                                let expanded_signal2 = Signal::derive(move || expanded_set.get().contains(&key2));
                                                let on_expand2 = Callback::new(move |()| {
                                                    expanded_set.update(|s| { s.insert(key_cb2.clone()); });
                                                });
                                                view! {
                                                    <Message message=item.0 is_current_user=is_me show_header=item.1 sender_blocked=sender_blocked expanded_signal=expanded_signal2 on_click_expand=on_expand2 />
                                                }}
                                            </For>
                                        }
                                            .into_any()
                                    }
                                }}
                            </div>
                            <div class="p-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/30 shrink-0">
                                <ChatInput
                                    destination=Signal::derive(move || selected.get().map(|s| s.to_destination()).unwrap_or(ChatDestination::Global))
                                    disabled=move || {
                                        let s = selected.get();
                                        if let Some(ref ch) = s {
                                            if matches!(ch, SelectedChannel::Global) {
                                                return !auth.user.with_untracked(|u| u.as_ref().is_some_and(|a| a.user.admin));
                                            }
                                            if matches!(ch, SelectedChannel::Tournament(_, _, false, _)) {
                                                return true;
                                            }
                                        }
                                        false
                                    }
                                />
                            </div>
                        </div>
                    }
                        .into_any()
                }}
            </main>
        </div>
    }
}

#[component]
fn GameChatToggle(
    channel_type: String,
    channel_id: String,
    label: String,
    white_id: Uuid,
    black_id: Uuid,
    finished: bool,
    set_selected: WriteSignal<Option<SelectedChannel>>,
) -> impl IntoView {
    let viewing_players = channel_type == CHANNEL_TYPE_GAME_PLAYERS;
    let channel_id_players = channel_id.clone();
    let label_players = label.clone();
    let switch_to_players = move |_| {
        set_selected.set(Some(SelectedChannel::Game(
            CHANNEL_TYPE_GAME_PLAYERS.to_string(),
            channel_id_players.clone(),
            label_players.clone(),
            white_id,
            black_id,
            finished,
        )));
    };
    let switch_to_spectators = move |_| {
        set_selected.set(Some(SelectedChannel::Game(
            CHANNEL_TYPE_GAME_SPECTATORS.to_string(),
            channel_id.clone(),
            label.clone(),
            white_id,
            black_id,
            finished,
        )));
    };
    view! {
        <div class="flex rounded-lg border border-gray-300 dark:border-gray-600 p-0.5 bg-gray-100 dark:bg-gray-800">
            <button
                type="button"
                class=move || format!(
                    "flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                    if viewing_players {
                        "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                    } else {
                        "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                    }
                )
                on:click=switch_to_players
            >
                "Players"
            </button>
            <button
                type="button"
                disabled=move || !finished
                class=move || format!(
                    "flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                    if !viewing_players {
                        "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                    } else if finished {
                        "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                    } else {
                        "text-gray-400 dark:text-gray-500 cursor-not-allowed"
                    }
                )
                on:click=switch_to_spectators
            >
                "Spectators"
            </button>
        </div>
    }
}

#[component]
fn GameChatHeader(
    channel_type: String,
    channel_id: String,
    label: String,
    white_id: Uuid,
    black_id: Uuid,
    finished: bool,
    set_selected: WriteSignal<Option<SelectedChannel>>,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let current_user_id = move || auth.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid));
    let game_id = GameId(channel_id.clone());
    let game = Resource::new(
        move || game_id.clone(),
        move |id| async move { get_game_from_nanoid(id).await },
    );
    let is_player = move || {
        current_user_id().is_some_and(|uid| uid == white_id || uid == black_id)
    };
    view! {
        <div class="px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50/80 dark:bg-gray-800/30 shrink-0 flex flex-col gap-2">
            {move || {
                if is_player() {
                    view! {
                        <GameChatToggle
                            channel_type=channel_type.clone()
                            channel_id=channel_id.clone()
                            label=label.clone()
                            white_id=white_id
                            black_id=black_id
                            finished=finished
                            set_selected=set_selected
                        />
                    }
                        .into_any()
                } else {
                    view! {}.into_any()
                }
            }}
            {move || {
                let g = game.get();
                match g {
                    Some(Ok(gr)) => {
                        let time_info = TimeInfo {
                            mode: gr.time_mode,
                            base: gr.time_base,
                            increment: gr.time_increment,
                        };
                        let (state_label, result_str) = if gr.finished {
                            let detail = match (&gr.game_status, &gr.conclusion) {
                                (GameStatus::Finished(GameResult::Winner(color)), c) => {
                                    let winner = match color {
                                        Color::White => gr.white_player.username.clone(),
                                        Color::Black => gr.black_player.username.clone(),
                                    };
                                    format!("{} won {}", winner, c.pretty_string())
                                }
                                (GameStatus::Finished(GameResult::Draw), c) => {
                                    format!("Draw {}", c.pretty_string())
                                }
                                (GameStatus::Adjudicated, _) => gr.tournament_game_result.to_string(),
                                _ => String::new(),
                            };
                            ("Finished:", detail)
                        } else {
                            ("Started", String::new())
                        };
                        let created = gr.created_at.format("%Y-%m-%d %H:%M").to_string();
                        let nanoid = gr.game_id.0.clone();
                        view! {
                            <div class="flex flex-wrap gap-x-3 gap-y-1 items-center text-sm">
                                <span class="font-medium text-gray-700 dark:text-gray-300">{state_label}</span>
                                {if !result_str.is_empty() {
                                    view! { <span class="text-gray-600 dark:text-gray-400">{result_str}</span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                                <a
                                    href=format!("/game/{}", nanoid)
                                    class="font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                                >
                                    "View game →"
                                </a>
                                <span class="text-gray-500 dark:text-gray-400 text-xs font-mono" title="Game ID (match to URL)">{nanoid}</span>
                                <TimeRow time_info extend_tw_classes="text-gray-600 dark:text-gray-400" />
                                <span class="text-gray-500 dark:text-gray-400 text-xs">{created}</span>
                            </div>
                        }
                            .into_any()
                    }
                    Some(Err(_)) => view! { <div class="text-sm text-red-600 dark:text-red-400">"Failed to load game"</div> }.into_any(),
                    None => view! { <div class="text-sm text-gray-500 dark:text-gray-400 animate-pulse">"Loading…"</div> }.into_any(),
                }
            }}
        </div>
    }
}

/// Max height for each channel list section so none dominates; scrollable within.
const SECTION_LIST_MAX_H: &str = "max-h-48 min-h-0 overflow-y-auto";
/// Section header button: collapsible, sticky when sidebar scrolls, good touch target (≥44px).
const SECTION_HEADER_BTN: &str = "sticky top-0 z-10 w-full text-left flex items-center justify-between gap-2 px-2 py-2.5 \
    text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider \
    border-l-2 border-pillbug-teal/50 dark:border-pillbug-teal/40 \
    bg-white dark:bg-gray-900 hover:bg-gray-100 dark:hover:bg-gray-800/50 rounded-r transition-colors min-h-[2.75rem]";

#[component]
fn ChannelLists(
    conv: MyConversations,
    current_user_id: impl Fn() -> Option<Uuid> + 'static,
    selected: ReadSignal<Option<SelectedChannel>>,
    set_selected: WriteSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let me = current_user_id();
    let has_global = conv.has_global;
    let dms_open = RwSignal::new(true);
    let tournaments_open = RwSignal::new(true);
    let games_open = RwSignal::new(true);
    let global_open = RwSignal::new(true);
    let conv_stored = StoredValue::new(conv);
    let empty_hint = "px-2 py-1.5 text-sm text-gray-400 dark:text-gray-500 italic";
    let btn_base = "w-full text-left px-3 py-2 rounded-lg flex justify-between items-center gap-2 \
                    transition-colors duration-150 truncate text-sm min-h-[2.75rem]";
    view! {
        <section class="mb-2 flex flex-col min-h-0">
            <button type="button" class=SECTION_HEADER_BTN on:click=move |_| dms_open.update(|o| *o = !*o)>
                <span>"DMs"</span>
                <span class="text-[0.65rem] opacity-70">{move || if dms_open.get() { "▼" } else { "▶" }}</span>
            </button>
            <Show when=move || dms_open.get() fallback=|| view! {}.into_any()>
                <div class=SECTION_LIST_MAX_H>
                    {move || conv_stored.get_value().dms.is_empty().then(|| view! { <p class=empty_hint>"No DMs yet"</p> })}
                    <For each=move || conv_stored.get_value().dms key=|d| d.other_user_id let:d>
                {let is_selected = move || selected.get().as_ref().is_some_and(|s| matches!(s, SelectedChannel::Dm(id, _) if *id == d.other_user_id));
                let d_other = d.other_user_id;
                view! {
                    <button
                        type="button"
                        class=move || format!(
                            "{} {}",
                            btn_base,
                            if is_selected() {
                                "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium"
                            } else {
                                "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                            }
                        )
                        on:click=move |_| set_selected.set(Some(SelectedChannel::Dm(d.other_user_id, d.username.clone())))
                    >
                        <span class="truncate">{d.username.clone()}</span>
                        {move || {
                            let _ = chat.unread_counts.get();
                            let unread = me.map(|u| chat.unread_count_for_dm(d_other, u)).unwrap_or(0);
                            (unread > 0).then(|| view! { <span class="shrink-0 h-5 min-w-5 flex items-center justify-center px-1.5 text-xs font-medium leading-none text-white bg-ladybug-red dark:bg-red-500 rounded-full">{if unread > 99 { "99+".to_string() } else { unread.to_string() }}</span> })
                        }}
                    </button>
                }}
            </For>
                </div>
            </Show>
        </section>
        <section class="mb-2 flex flex-col min-h-0">
            <button type="button" class=SECTION_HEADER_BTN on:click=move |_| tournaments_open.update(|o| *o = !*o)>
                <span>"Tournaments"</span>
                <span class="text-[0.65rem] opacity-70">{move || if tournaments_open.get() { "▼" } else { "▶" }}</span>
            </button>
            <Show when=move || tournaments_open.get() fallback=|| view! {}.into_any()>
                <div class=SECTION_LIST_MAX_H>
                    {move || conv_stored.get_value().tournaments.is_empty().then(|| view! { <p class=empty_hint>"No tournament chats"</p> })}
                    <For each=move || conv_stored.get_value().tournaments key=|t| t.nanoid.clone() let:t>
                {let nanoid = t.nanoid.clone();
                let name = t.name.clone();
                let nanoid_cmp = nanoid.clone();
                let tid = TournamentId(nanoid.clone());
                let is_selected = move || selected.get().as_ref().is_some_and(|s| matches!(s, SelectedChannel::Tournament(n, _, _, _) if *n == nanoid_cmp));
                view! {
                    <button
                        type="button"
                        class=move || format!(
                            "{} {}",
                            btn_base,
                            if is_selected() {
                                "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium"
                            } else {
                                "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                            }
                        )
                        on:click=move |_| set_selected.set(Some(SelectedChannel::Tournament(nanoid.clone(), name.clone(), t.is_participant, t.muted)))
                    >
                        <span class="truncate flex items-center gap-1">
                            {name.clone()}
                            {if t.muted {
                                view! { <span class="shrink-0 text-[0.65rem] uppercase text-gray-400 dark:text-gray-500" title="Muted">"Muted"</span> }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                        </span>
                        {move || {
                            let _ = chat.unread_counts.get();
                            let unread = chat.unread_count_for_tournament(&tid);
                            (unread > 0).then(|| view! { <span class="shrink-0 h-5 min-w-5 flex items-center justify-center px-1.5 text-xs font-medium leading-none text-white bg-ladybug-red dark:bg-red-500 rounded-full">{if unread > 99 { "99+".to_string() } else { unread.to_string() }}</span> })
                        }}
                    </button>
                }}
            </For>
                </div>
            </Show>
        </section>
        <section class="mb-2 flex flex-col min-h-0">
            <button type="button" class=SECTION_HEADER_BTN on:click=move |_| games_open.update(|o| *o = !*o)>
                <span>"Games"</span>
                <span class="text-[0.65rem] opacity-70">{move || if games_open.get() { "▼" } else { "▶" }}</span>
            </button>
            <Show when=move || games_open.get() fallback=|| view! {}.into_any()>
                <div class=SECTION_LIST_MAX_H>
                    {move || conv_stored.get_value().games.is_empty().then(|| view! { <p class=empty_hint>"No game chats"</p> })}
                    <For each=move || conv_stored.get_value().games key=|g| format!("{}::{}", g.channel_type, g.channel_id) let:g>
                {let channel_type = g.channel_type.clone();
                let channel_id = g.channel_id.clone();
                let label = g.label.clone();
                let display_label = label
                    .replace(" (players)", "")
                    .replace(" (spectators)", "");
                let display_label_with_nanoid = format!("{} ({})", display_label, channel_id);
                let white_id = g.white_id;
                let black_id = g.black_id;
                let cid_cmp = channel_id.clone();
                let gid = GameId(channel_id.clone());
                let is_selected = move || selected.get().as_ref().is_some_and(|s| matches!(s, SelectedChannel::Game(_, cid, _, _, _, _) if *cid == cid_cmp));
                view! {
                    <button
                        type="button"
                        class=move || format!(
                            "{} {}",
                            btn_base,
                            if is_selected() {
                                "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium"
                            } else {
                                "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                            }
                        )
                        on:click=move |_| set_selected.set(Some(SelectedChannel::Game(
                            channel_type.clone(),
                            channel_id.clone(),
                            label.clone(),
                            white_id,
                            black_id,
                            g.finished,
                        )))
                    >
                        <span class="truncate" title=display_label_with_nanoid.clone()>{display_label_with_nanoid.clone()}</span>
                        {move || {
                            let _ = chat.unread_counts.get();
                            let unread = chat.unread_count_for_game(&gid);
                            (unread > 0).then(|| view! { <span class="shrink-0 h-5 min-w-5 flex items-center justify-center px-1.5 text-xs font-medium leading-none text-white bg-ladybug-red dark:bg-red-500 rounded-full">{if unread > 99 { "99+".to_string() } else { unread.to_string() }}</span> })
                        }}
                    </button>
                }}
            </For>
                </div>
            </Show>
        </section>
        {has_global.then(|| {
            view! {
                <section class="mb-2 flex flex-col min-h-0">
                    <button type="button" class=SECTION_HEADER_BTN on:click=move |_| global_open.update(|o| *o = !*o)>
                        <span>"Global"</span>
                        <span class="text-[0.65rem] opacity-70">{move || if global_open.get() { "▼" } else { "▶" }}</span>
                    </button>
                    <Show when=move || global_open.get() fallback=|| view! {}.into_any()>
                        <div class=SECTION_LIST_MAX_H>
                    <button
                        type="button"
                        class=move || format!(
                            "{} {}",
                            btn_base,
                            if selected.get().as_ref().is_some_and(|s| matches!(s, SelectedChannel::Global)) {
                                "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium"
                            } else {
                                "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                            }
                        )
                        on:click=move |_| set_selected.set(Some(SelectedChannel::Global))
                    >
                        "Global"
                    </button>
                        </div>
                    </Show>
                </section>
            }
        })}
    }
}
