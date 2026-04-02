use crate::{
    components::update_from_event::update_from_input,
    functions::blocks_mutes::get_blocked_user_ids,
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use chrono::{Duration, Local};
use leptos::{html, prelude::*};
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use leptos::leptos_dom::helpers::request_animation_frame;
use leptos_use::use_interval_fn;
use shared_types::{
    ChatDestination, ChatMessage, GameId, SimpleDestination,
    CHANNEL_TYPE_GAME_PLAYERS, CHANNEL_TYPE_GAME_SPECTATORS, CHANNEL_TYPE_GLOBAL,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use std::collections::HashSet;
use uuid::Uuid;

/// Max time gap for grouping consecutive same-user messages. Messages further apart show the header again.
const MESSAGE_GROUP_MAX_GAP: Duration = Duration::minutes(2);

/// For each message, true if the "user · timestamp · turn" header should be shown (first of a consecutive same-user block).
/// Same-user messages are only grouped if they are within MESSAGE_GROUP_MAX_GAP (2 min) of each other.
pub(crate) fn messages_with_header_flags(messages: &[ChatMessage]) -> Vec<(ChatMessage, bool)> {
    messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let same_user = i > 0 && messages[i - 1].user_id == m.user_id;
            let gap_too_large = match (i.checked_sub(1).and_then(|j| messages.get(j)), m) {
                (Some(prev), curr) => match (prev.timestamp, curr.timestamp) {
                    (Some(prev_ts), Some(curr_ts)) => {
                        (curr_ts - prev_ts).abs() > MESSAGE_GROUP_MAX_GAP
                    }
                    _ => true,
                },
                _ => true,
            };
            let show_header = i == 0 || !same_user || gap_too_large;
            (m.clone(), show_header)
        })
        .collect()
}

#[component]
pub fn Message(
    message: ChatMessage,
    #[prop(optional)] is_current_user: bool,
    /// When false, only the message bubble is shown (for consecutive messages from the same user).
    #[prop(optional, default = true)] show_header: bool,
    /// When true, show "Message hidden. Click to expand." (Discord-style for blocked users in shared channels).
    #[prop(optional)] sender_blocked: bool,
    /// When sender_blocked, parent-owned expanded state so it survives re-renders. If None, component uses internal state.
    #[prop(optional)] expanded_signal: Option<Signal<bool>>,
    /// Callback when user clicks "Click to expand". Parent should add this message to its expanded set.
    #[prop(optional)] on_click_expand: Option<Callback<(), ()>>,
) -> impl IntoView {
    let expanded_inner = RwSignal::new(false);
    let is_expanded = move || {
        match (expanded_signal.as_ref(), on_click_expand.as_ref()) {
            (Some(s), _) => s.get(),
            _ => expanded_inner.get(),
        }
    };
    let on_expand = move |_| {
        if let Some(ref cb) = on_click_expand {
            cb.run(());
        } else {
            expanded_inner.set(true);
        }
    };
    let outer_margin = move || {
        if show_header {
            "mb-3"
        } else {
            "mb-1"
        }
    };
    view! {
        <div class=move || {
            let margin = outer_margin();
            if is_current_user {
                format!("flex flex-col items-end {} w-full", margin)
            } else {
                format!("flex flex-col items-start {} w-full", margin)
            }
        }>
            {move || {
                let show_hidden = sender_blocked && !is_expanded();
                let m = message.clone();
                if show_hidden {
                    view! {
                        <button
                            type="button"
                            class="mb-1 px-3 py-2 rounded-lg max-w-[85%] sm:max-w-[75%] text-left text-sm text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 border border-gray-200 dark:border-gray-600 transition-colors"
                            on:click=on_expand
                        >
                            "Message hidden. Click to expand."
                        </button>
                    }.into_any()
                } else {
                    let user_local_time = m
                        .timestamp
                        .map(|t| t.with_timezone(&Local).format("%d/%m %H:%M").to_string())
                        .unwrap_or_default();
                    let turn = m.turn.map(|turn| format!(" · turn {turn}"));
                    view! {
                        {if show_header {
                            view! {
                                <div class="flex flex-wrap items-baseline gap-x-2 gap-y-0.5 mb-1 px-1 max-w-[85%] sm:max-w-[75%]">
                                    <span class="font-semibold text-sm text-gray-800 dark:text-gray-200 truncate">
                                        {m.username}
                                    </span>
                                    <span class="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                                        {user_local_time}
                                        {turn}
                                    </span>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                        <div class=move || {
                            if is_current_user {
                                "px-3 py-2 rounded-2xl rounded-br-md max-w-[85%] sm:max-w-[75%] \
                                 bg-pillbug-teal/90 dark:bg-pillbug-teal/80 text-white text-sm break-words \
                                 shadow-sm"
                            } else {
                                "px-3 py-2 rounded-2xl rounded-bl-md max-w-[85%] sm:max-w-[75%] \
                                 bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 text-sm break-words \
                                 shadow-sm"
                            }
                        }>
                            {m.message}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
pub fn ChatInput(
    destination: Signal<ChatDestination>,
    disabled: impl Fn() -> bool + 'static + Send + Sync,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let game_state = use_context::<GameStateSignal>();
    let turn = move || game_state.map(|gs| gs.signal.with(|state| state.state.turn));
    let is_disabled = Signal::derive(move || disabled());
    let is_disabled_send = is_disabled.clone();
    let is_disabled_placeholder = is_disabled.clone();
    let is_disabled_keydown = is_disabled.clone();
    let send = move || {
        if is_disabled_send.get() {
            return;
        }
        let message = chat.typed_message.get();
        if !message.is_empty() {
            chat.send(&message, destination(), turn());
            chat.typed_message.set(String::new());
        };
    };
    let placeholder = move || {
        if is_disabled_placeholder.get() {
            "Admin only"
        } else {
            match destination() {
                ChatDestination::GamePlayers(_, _, _) => "Chat with opponent",
                ChatDestination::GameSpectators(_, _, _) => "Chat with spectators",
                _ => "Chat",
            }
        }
    };
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    view! {
        <input
            node_ref=my_input
            type="text"
            disabled=move || is_disabled.get()
            class="box-border w-full px-4 py-3 rounded-xl border border-gray-300 dark:border-gray-600 \
                   bg-white dark:bg-gray-800 text-black dark:text-white placeholder-gray-500 dark:placeholder-gray-400 \
                   focus:outline-none focus:ring-2 focus:ring-pillbug-teal/50 focus:border-pillbug-teal \
                   transition-shadow shrink-0 shadow-inner disabled:opacity-50 disabled:cursor-not-allowed"
            prop:value=chat.typed_message
            prop:placeholder=placeholder
            on:input=update_from_input(chat.typed_message)
            on:keydown=move |evt| {
                if evt.key() == "Enter" && !is_disabled_keydown.get() {
                    evt.prevent_default();
                    send();
                }
            }

            maxlength="1000"
        />
    }
}

/// When provided for SimpleDestination::Game, overrides which channel to show: true = Players, false = Spectators.
/// Used on the game page so players can toggle. When None, destination is derived from uid_is_player.
#[component]
pub fn ChatWindow(
    destination: SimpleDestination,
    #[prop(optional)] correspondant_id: Option<Uuid>,
    #[prop(optional)] correspondant_username: String,
    /// When Some and true, input is disabled (e.g. tournament read-only for non-participants).
    #[prop(optional)]
    input_disabled: Option<Signal<bool>>,
    /// When Some, for Game destination use this (true = Players, false = Spectators) instead of uid_is_player.
    #[prop(optional)]
    game_channel_override: Option<Signal<bool>>,
) -> impl IntoView {
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let game_state = expect_context::<GameStateSignal>();
    let uid = auth_context
        .user
        .with_untracked(|a| a.as_ref().map(|user| user.user.uid));
    let white_id_slice = create_read_slice(game_state.signal, |gs| gs.white_id);
    let black_id_slice = create_read_slice(game_state.signal, |gs| gs.black_id);
    let white_id = move || white_id_slice();
    let black_id = move || black_id_slice();

    let correspondant_id = Signal::derive(move || correspondant_id.map_or(Uuid::new_v4(), |id| id));
    let correspondant_username = Signal::derive(move || correspondant_username.clone());
    let destination_for_fetch = destination.clone();
    let destination_for_loading = destination.clone();
    let div = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let (unread_at_open, set_unread_at_open) = signal::<Option<i64>>(None);
    let block_list = Resource::new(|| (), |_| async move { get_blocked_user_ids().await });
    let expanded_hidden_messages =
        RwSignal::new(HashSet::<(i64, Uuid, String)>::new());

    let game_state_ready = move || white_id().is_some() && black_id().is_some();
    let actual_destination = Signal::derive(move || match destination.clone() {
        SimpleDestination::Game => {
            match (white_id(), black_id()) {
                (Some(w), Some(b)) => {
                    let use_players = game_channel_override
                        .as_ref()
                        .map(|s| s.get())
                        .unwrap_or_else(|| game_state.signal.with(|gs| gs.uid_is_player(uid)));
                    if use_players {
                        ChatDestination::GamePlayers(game_id(), w, b)
                    } else {
                        ChatDestination::GameSpectators(game_id(), w, b)
                    }
                }
                _ => ChatDestination::Global,
            }
        }
        SimpleDestination::User => {
            ChatDestination::User((correspondant_id(), correspondant_username()))
        }
        SimpleDestination::Global => ChatDestination::Global,
        SimpleDestination::Tournament(tournament_id) => {
            ChatDestination::TournamentLobby(tournament_id)
        }
    });

    // Fetch chat history on mount when ChatWindow shows tournament or game.
    // For finished games, fetch both channels and merge.
    Effect::watch(
        move || {
            match &destination_for_fetch {
                SimpleDestination::Tournament(tid) => {
                    vec![(CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(), tid.0.clone())]
                }
                SimpleDestination::Game => {
                    let gid = game_id();
                    if gid.0.is_empty() {
                        vec![]
                    } else if game_state.signal.with(|gs| {
                        gs.game_response
                            .as_ref()
                            .map_or(false, |gr| gr.finished)
                    }) {
                        vec![
                            (CHANNEL_TYPE_GAME_PLAYERS.to_string(), gid.0.clone()),
                            (CHANNEL_TYPE_GAME_SPECTATORS.to_string(), gid.0.clone()),
                        ]
                    } else {
                        let ct = if game_state.signal.with(|gs| gs.uid_is_player(uid)) {
                            CHANNEL_TYPE_GAME_PLAYERS.to_string()
                        } else {
                            CHANNEL_TYPE_GAME_SPECTATORS.to_string()
                        };
                        vec![(ct, gid.0)]
                    }
                }
                _ => vec![],
            }
        },
        move |channels, _, _| {
            for (ct, cid) in channels {
                let chat = chat.clone();
                let ct = ct.clone();
                let cid = cid.clone();
                spawn_local(async move {
                    if let Ok(messages) = chat.fetch_channel_history(&ct, &cid).await {
                        chat.inject_history(&ct, &cid, messages);
                    }
                });
            }
        },
        true,
    );

    // Mark channel as read on server when viewing tournament lobby, DM, or game chat.
    // Capture unread count before marking so we can show divider and scroll to first unread.
    Effect::watch(
        move || {
            let dest = actual_destination();
            let msg_count = match &dest {
                ChatDestination::TournamentLobby(tid) => (chat.tournament_lobby_messages)()
                    .get(tid)
                    .map(|v| v.len())
                    .unwrap_or(0),
                ChatDestination::User((id, _)) => (chat.users_messages)()
                    .get(id)
                    .map(|v| v.len())
                    .unwrap_or(0),
                ChatDestination::GamePlayers(gid, ..) | ChatDestination::GameSpectators(gid, ..) => {
                    (chat.games_private_messages)()
                        .get(gid)
                        .map(|v| v.len())
                        .unwrap_or(0)
                        + (chat.games_public_messages)()
                            .get(gid)
                            .map(|v| v.len())
                            .unwrap_or(0)
                }
                _ => 0,
            };
            (dest, msg_count)
        },
        move |(dest, _), _, _| {
            set_unread_at_open.set(None); // Reset when dest changes; set below if this channel has unread
            let unread = match &dest {
                ChatDestination::TournamentLobby(tid) => chat.unread_count_for_tournament(tid),
                ChatDestination::User((other_id, _)) => auth_context
                    .user
                    .with_untracked(|a| a.as_ref().map_or(0, |a| chat.unread_count_for_dm(*other_id, a.user.uid))),
                ChatDestination::GamePlayers(gid, ..) | ChatDestination::GameSpectators(gid, ..) => {
                    chat.unread_count_for_game(gid)
                }
                ChatDestination::Global => chat.unread_count_for_global(),
            };
            if unread > 0 {
                set_unread_at_open.set(Some(unread));
            }
            match &dest {
                ChatDestination::TournamentLobby(tournament_id) => {
                    chat.seen_tournament_lobby(tournament_id.clone());
                }
                ChatDestination::User((other_id, _)) => {
                    if let Some(current_id) = auth_context.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid)) {
                        chat.seen_dm(*other_id, current_id);
                    }
                }
                ChatDestination::GamePlayers(game_id, ..) | ChatDestination::GameSpectators(game_id, ..) => {
                    chat.seen_messages(game_id.clone());
                }
                ChatDestination::Global => {
                    chat.mark_read(CHANNEL_TYPE_GLOBAL, CHANNEL_TYPE_GLOBAL);
                }
            }
        },
        true,
    );

    // Clear divider after 10 seconds
    use_interval_fn(
        move || {
            if unread_at_open.get_untracked().is_some() {
                set_unread_at_open.set(None);
            }
        },
        10_000,
    );

    // Scroll: on destination change / initial load → first unread (if any) or bottom. When a NEW message
    // arrives (same destination, count N→N+1) → scroll to bottom so the newest message is visible.
    let first_unread_ref = first_unread_ref.clone();
    Effect::watch(
        move || {
            let dest = actual_destination();
            let count = match &dest {
                ChatDestination::TournamentLobby(tid) => (chat.tournament_lobby_messages)().get(tid).map(|v| v.len()).unwrap_or(0),
                ChatDestination::GamePlayers(gid, ..) | ChatDestination::GameSpectators(gid, ..) => {
                    (chat.games_private_messages)().get(gid).map(|v| v.len()).unwrap_or(0)
                        + (chat.games_public_messages)().get(gid).map(|v| v.len()).unwrap_or(0)
                }
                _ => 0,
            };
            (dest, count)
        },
        move |(dest, count), prev, _| {
            let (run, is_new_message) = match prev {
                None => (true, false),
                Some((prev_dest, prev_count)) => {
                    let dest_changed = dest != prev_dest;
                    let count_increased = count > prev_count;
                    let is_new_message = count_increased && dest == prev_dest && *prev_count > 0;
                    (dest_changed || count_increased, is_new_message)
                }
            };
            if !run {
                return;
            }
            let container = div.get_untracked();
            let target = first_unread_ref.get_untracked();
            request_animation_frame(move || {
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

    let messages = move || {
        let mut v = match actual_destination() {
            ChatDestination::TournamentLobby(tournament) => (chat.tournament_lobby_messages)()
                .get(&tournament)
                .cloned()
                .unwrap_or_default(),
            ChatDestination::GamePlayers(game_id, ..) => {
                let private_msgs = (chat.games_private_messages)()
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_default();
                let public_msgs = (chat.games_public_messages)()
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_default();
                let finished = game_state.signal.with(|gs| {
                    gs.game_response
                        .as_ref()
                        .map_or(false, |gr| gr.finished)
                });
                let single_channel = game_channel_override.is_some();
                if finished && !single_channel {
                    let mut merged = private_msgs;
                    merged.extend(public_msgs);
                    merged
                } else {
                    private_msgs
                }
            }
            ChatDestination::GameSpectators(game_id, ..) => {
                let private_msgs = (chat.games_private_messages)()
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_default();
                let public_msgs = (chat.games_public_messages)()
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_default();
                let finished = game_state.signal.with(|gs| {
                    gs.game_response
                        .as_ref()
                        .map_or(false, |gr| gr.finished)
                });
                let single_channel = game_channel_override.is_some();
                if finished && !single_channel {
                    let mut merged = private_msgs;
                    merged.extend(public_msgs);
                    merged
                } else {
                    public_msgs
                }
            }
            ChatDestination::User((correspondant_id, _username)) => (chat.users_messages)()
                .get(&correspondant_id)
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        v.sort_by_key(|m| m.timestamp.map(|t| t.timestamp()).unwrap_or(0));
        v
    };
    let show_loading = move || {
        matches!(destination_for_loading.clone(), SimpleDestination::Game) && !game_state_ready()
    };
    let me_uid = move || auth_context.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid));
    view! {
        <div
            id="ignoreChat"
            class="flex flex-col flex-grow justify-between w-full min-w-full max-w-full h-full min-h-0 overflow-hidden"
        >
            <div node_ref=div class="overflow-y-auto flex-grow w-full min-w-full h-0 min-h-0 p-4">
                {move || {
                    if show_loading() {
                        view! {
                            <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400 text-sm">
                                "Loading chat…"
                            </div>
                        }
                            .into_any()
                    } else {
                        let msgs = messages();
                        let empty_thread = msgs.is_empty();
                        if empty_thread {
                            view! {
                                <div class="flex flex-col items-center justify-center h-full min-h-[8rem] text-gray-500 dark:text-gray-400 gap-2">
                                    <span class="text-3xl opacity-40">"✉️"</span>
                                    <p class="text-sm font-medium">"No messages yet"</p>
                                    <p class="text-xs">"Send a message to start the conversation."</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            let n_unread = unread_at_open.get().unwrap_or(0) as usize;
                            let _ = unread_at_open.get();
                            let (read_msgs, unread_msgs) = if n_unread > 0 && n_unread <= msgs.len() {
                                let split_idx = msgs.len() - n_unread;
                                let (r, u) = msgs.split_at(split_idx);
                                (r.to_vec(), u.to_vec())
                            } else {
                                (msgs.clone(), vec![])
                            };
                            let read_with_flags = messages_with_header_flags(&read_msgs);
                            let unread_with_flags = messages_with_header_flags(&unread_msgs);
                            let me = me_uid();
                            let blocked_ids: HashSet<Uuid> = block_list
                                .get()
                                .and_then(Result::ok)
                                .unwrap_or_default()
                                .into_iter()
                                .collect();
                            let blocked_ids2 = blocked_ids.clone();
                            let is_shared_channel = matches!(
                                actual_destination(),
                                ChatDestination::GamePlayers(_, _, _)
                                    | ChatDestination::GameSpectators(_, _, _)
                                    | ChatDestination::TournamentLobby(_)
                                    | ChatDestination::Global
                            );
                            let expanded_set = expanded_hidden_messages;
                            view! {
                                <For each=move || read_with_flags.clone() key=|item| (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.message.clone()) let:item>
                                    {let is_me = me.is_some_and(|u| item.0.user_id == u);
                                    let sender_blocked = is_shared_channel && blocked_ids.contains(&item.0.user_id);
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
                                {move || {
                                    if n_unread > 0 && unread_at_open.get().is_some() {
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
                                    }
                                }}
                                <For each=move || unread_with_flags.clone() key=|item| (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.message.clone()) let:item>
                                    {let is_me = me.is_some_and(|u| item.0.user_id == u);
                                    let sender_blocked2 = is_shared_channel && blocked_ids2.contains(&item.0.user_id);
                                    let key2 = (item.0.timestamp.map(|t| t.timestamp()).unwrap_or(0), item.0.user_id, item.0.message.clone());
                                    let key_cb2 = key2.clone();
                                    let expanded_signal2 = Signal::derive(move || expanded_set.get().contains(&key2));
                                    let on_expand2 = Callback::new(move |()| {
                                        expanded_set.update(|s| { s.insert(key_cb2.clone()); });
                                    });
                                    view! {
                                        <Message message=item.0 is_current_user=is_me show_header=item.1 sender_blocked=sender_blocked2 expanded_signal=expanded_signal2 on_click_expand=on_expand2 />
                                    }}
                                </For>
                            }
                                .into_any()
                        }
                    }
                }}
            </div>
            <div class="shrink-0 border-t border-gray-200 dark:border-gray-700">
                <ChatInput
                    destination=actual_destination
                    disabled=move || {
                        let extra = input_disabled.as_ref().map(|s| s.get()).unwrap_or(false);
                        if extra {
                            true
                        } else {
                            match actual_destination() {
                                ChatDestination::Global => !auth_context
                                    .user
                                    .with_untracked(|u| u.as_ref().is_some_and(|a| a.user.admin)),
                                _ => false,
                            }
                        }
                    }
                />
            </div>
        </div>
    }
}
