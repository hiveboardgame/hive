//! Messages hub: /messages — DMs, Tournaments, Games, and recent announcements.
//! Supports ?dm=:uuid to open a DM from profile.

use crate::{
    chat::SimpleDestination,
    components::{
        atoms::block_toggle_button::BlockToggleButton,
        molecules::time_row::TimeRow,
        organisms::chat::ChatWindow,
    },
    functions::{
        blocks_mutes::{mute_tournament_chat, unmute_tournament_chat},
        chat::{
            get_messages_hub_data,
            DmConversation,
            GameChannel,
            MyConversations,
            TournamentChannel,
        },
        games::get::get_game_from_nanoid,
    },
    i18n::*,
    providers::{chat::Chat, AuthContext},
    responses::GameResponse,
};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_query_map};
use shared_types::{
    ChannelType,
    GameId,
    PrettyString,
    TimeInfo,
    TournamentId,
};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectedChannel {
    Dm {
        other_id: Uuid,
        username: String,
    },
    Tournament {
        nanoid: String,
        name: String,
        is_participant: bool,
        muted: bool,
    },
    Game {
        channel_type: ChannelType,
        channel_id: String,
        label: String,
        white_id: Uuid,
        black_id: Uuid,
        finished: bool,
    },
    Global,
}

impl SelectedChannel {
    fn label(&self, global_label: &str) -> String {
        match self {
            SelectedChannel::Dm { username, .. } => username.clone(),
            SelectedChannel::Tournament { name, .. } => name.clone(),
            SelectedChannel::Game { label, .. } => label.clone(),
            SelectedChannel::Global => global_label.to_string(),
        }
    }
}

#[component]
pub fn Messages() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
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
        query_map
            .get()
            .get("username")
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let hub_data = Resource::new(
        move || chat.conversation_list_version.get(),
        move |_| async move { get_messages_hub_data().await },
    );
    let blocked_ids = RwSignal::new(HashSet::<Uuid>::new());
    Effect::watch(
        move || hub_data.get(),
        move |result, _, _| {
            if let Some(Ok(data)) = result.clone() {
                blocked_ids.set(data.blocked_user_ids.iter().copied().collect());
                chat.apply_server_unread_counts(data.unread_counts.clone());
            }
        },
        true,
    );

    let selected = RwSignal::new(None::<SelectedChannel>);
    // On mobile: true = show channel list (drawer open), false = show thread full width. Desktop always shows both.
    let mobile_drawer_open = RwSignal::new(true);

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
            let selected_channel = selected.get_untracked();
            let manual_selection_exists = selected_channel.as_ref().is_some_and(
                |current| {
                    !matches!(current, SelectedChannel::Dm { other_id: id, .. } if *id == other_id)
                },
            );
            if manual_selection_exists {
                return;
            }

            let conv = hub_data
                .get()
                .and_then(Result::ok)
                .map(|data| data.conversations);
            let username_from_query = dm_username_param();
            let username = conv
                .as_ref()
                .and_then(|c| {
                    c.dms
                        .iter()
                        .find(|d| d.other_user_id == other_id)
                        .map(|d| d.username.clone())
                })
                .or({
                    if username_from_query.is_empty() {
                        None
                    } else {
                        Some(username_from_query)
                    }
                })
                .unwrap_or_else(|| t_string!(i18n, messages.page.unknown_user).to_string());
            selected.set(Some(SelectedChannel::Dm { other_id, username }));
        }
    });
    let selected_game_summary_key = Memo::new(move |_| {
        selected.get().and_then(|channel| match channel {
            SelectedChannel::Game { channel_id, .. } => Some(GameId(channel_id)),
            _ => None,
        })
    });
    let selected_game_summary = Resource::new(
        move || selected_game_summary_key.get(),
        move |game_id| async move {
            let game_id = game_id?;
            Some(get_game_from_nanoid(game_id).await)
        },
    );
    let selected_game_data = Signal::derive(move || match selected.get() {
        Some(SelectedChannel::Game {
            channel_id,
            white_id,
            black_id,
            finished,
            ..
        }) => {
            Some((GameId(channel_id), white_id, black_id, finished))
        }
        _ => None,
    });
    let selected_game_show_players = Signal::derive(move || {
        matches!(
            selected.get(),
            Some(SelectedChannel::Game { channel_type, .. })
                if channel_type == ChannelType::GamePlayers
        )
    });

    view! {
        <div class="flex overflow-hidden fixed right-0 bottom-0 left-0 top-12 z-0 flex-col bg-gray-100 sm:flex-row dark:bg-gray-950">
            <aside class=move || {
                format!(
                    "w-full sm:w-72 flex-shrink-0 flex flex-col min-h-0 overflow-hidden bg-white dark:bg-gray-900 shadow-lg sm:rounded-r-xl border-l border-gray-200 dark:border-gray-700 {} sm:!flex",
                    if mobile_drawer_open.get() { "" } else { "hidden " },
                )
            }>
                <div class="py-3 px-4 bg-white border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700">
                    <h1 class="text-xl font-bold tracking-tight text-gray-800 dark:text-gray-100">
                        {t!(i18n, messages.page.title)}
                    </h1>
                </div>
                <div class="overflow-y-auto flex-1 p-2 pb-6 min-h-0 sm:pb-2">
                    <ShowLet
                        some=move || hub_data.get()
                        let:hub_result
                        fallback=move || {
                            view! {
                                <p class="p-3 text-sm text-gray-500 animate-pulse dark:text-gray-400">
                                    {t!(i18n, messages.page.loading)}
                                </p>
                            }
                        }
                    >
                        <ShowLet
                            some=move || hub_result.as_ref().ok().cloned()
                            let:hub_page_data
                            fallback=move || {
                                view! {
                                    <p class="p-3 text-sm text-red-600 dark:text-red-400">
                                        {t!(i18n, messages.page.failed_conversations)}
                                    </p>
                                }
                            }
                        >
                            <ChannelLists
                                conv=hub_page_data.conversations
                                current_user_id=current_user_id
                                selected=selected
                                chat=chat
                            />
                        </ShowLet>
                    </ShowLet>
                </div>
            </aside>
            <main class=move || {
                format!(
                    "flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden {} sm:!flex",
                    if mobile_drawer_open.get() { "hidden " } else { "" },
                )
            }>
                <Show
                    when=move || selected.get().is_some()
                    fallback=move || {
                        view! {
                            <div class="flex flex-col flex-1 gap-2 justify-center items-center p-8 text-gray-500 dark:text-gray-400">
                                <span class="text-4xl opacity-50">"💬"</span>
                                <p class="font-medium text-center">
                                    {t!(i18n, messages.page.select_conversation)}
                                </p>
                                <p class="max-w-xs text-sm text-center">
                                    {t!(i18n, messages.page.choose_conversation)}
                                </p>
                            </div>
                        }
                    }
                >
                    <div class="flex overflow-hidden flex-col flex-1 min-h-0 bg-white border-r border-gray-200 shadow-inner sm:rounded-l-xl dark:bg-gray-900 dark:border-gray-700">
                        <div class="flex gap-2 items-center py-3 px-2 bg-gray-50 border-b border-gray-200 sm:px-4 dark:border-gray-700 shrink-0 min-h-[2.75rem] dark:bg-gray-800/50">
                            <button
                                type="button"
                                class="flex flex-shrink-0 gap-1 justify-center items-center -ml-1 text-gray-600 rounded-lg transition-colors sm:hidden dark:text-gray-400 hover:text-gray-900 hover:bg-gray-200 min-h-[2.25rem] min-w-[2.25rem] dark:hover:bg-gray-700 dark:hover:text-gray-100"
                                aria-label=t_string!(i18n, messages.page.back_to_conversations)
                                on:click=move |_| mobile_drawer_open.set(true)
                            >
                                <span class="text-lg" aria-hidden="true">
                                    "←"
                                </span>
                                <span class="text-sm font-medium">
                                    {t!(i18n, messages.page.conversations)}
                                </span>
                            </button>
                            <h2 class="flex-1 min-w-0 text-lg font-semibold text-gray-800 dark:text-gray-100 truncate">
                                {move || {
                                    selected
                                        .get()
                                        .as_ref()
                                        .map(|s| {
                                            s.label(
                                                &t_string!(
                                                    i18n,
                                                    messages.sections.recent_announcements
                                                ),
                                            )
                                        })
                                        .unwrap_or_default()
                                }}
                            </h2>
                        </div>
                        <SelectedChannelActions
                            selected=selected
                            blocked_ids=blocked_ids
                            chat=chat
                            game_summary=Signal::derive(move || {
                                selected_game_summary.get().flatten()
                            })
                        />
                        <div class="overflow-hidden flex-1 min-h-0">
                            <ShowLet some=move || selected.get() let:selected_channel>
                                {move || match selected_channel.clone() {
                                    SelectedChannel::Dm { other_id, username } => {
                                        view! {
                                            <ChatWindow
                                                destination=SimpleDestination::User
                                                correspondant_id=other_id
                                                correspondant_username=username
                                            />
                                        }
                                            .into_any()
                                    }
                                    SelectedChannel::Tournament {
                                        nanoid,
                                        is_participant,
                                        ..
                                    } => {
                                        let input_disabled = Signal::derive(move || {
                                            !is_participant
                                        });
                                        view! {
                                            <ChatWindow
                                                destination=SimpleDestination::Tournament(
                                                    TournamentId(nanoid),
                                                )
                                                input_disabled
                                            />
                                        }
                                            .into_any()
                                    }
                                    SelectedChannel::Game { .. } => {
                                        view! {
                                            <ChatWindow
                                                destination=SimpleDestination::Game
                                                game_data=selected_game_data
                                                game_channel_override=selected_game_show_players
                                            />
                                        }
                                            .into_any()
                                    }
                                    SelectedChannel::Global => {
                                        view! {
                                            <ChatWindow destination=SimpleDestination::Global />
                                        }
                                            .into_any()
                                    }
                                }}
                            </ShowLet>
                        </div>
                    </div>
                </Show>
            </main>
        </div>
    }
}

#[component]
fn ChannelHeaderBar(children: Children) -> impl IntoView {
    view! {
        <div class="py-2 px-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50/80 shrink-0 dark:bg-gray-800/30">
            {children()}
        </div>
    }
}

#[component]
fn SelectedChannelActions(
    selected: RwSignal<Option<SelectedChannel>>,
    blocked_ids: RwSignal<HashSet<Uuid>>,
    chat: Chat,
    game_summary: Signal<Option<Result<GameResponse, ServerFnError>>>,
) -> impl IntoView {
    let selected_dm = Memo::new(move |_| match selected.get() {
        Some(SelectedChannel::Dm { other_id, username }) => Some((other_id, username)),
        _ => None,
    });
    let selected_game = Memo::new(move |_| match selected.get() {
        Some(SelectedChannel::Game {
            channel_type,
            channel_id,
            label,
            white_id,
            black_id,
            finished,
        }) => Some((
            channel_type,
            channel_id,
            label,
            white_id,
            black_id,
            finished,
        )),
        _ => None,
    });
    let selected_tournament = Memo::new(move |_| match selected.get() {
        Some(SelectedChannel::Tournament {
            nanoid,
            name,
            is_participant,
            ..
        }) => {
            Some((nanoid, name, is_participant))
        }
        _ => None,
    });

    view! {
        <ShowLet some=move || selected_dm.get() let:dm>
            <DmChannelActions other_id=dm.0 username=dm.1 blocked_ids />
        </ShowLet>
        <ShowLet some=move || selected_game.get() let:game>
            <GameChatHeader
                channel_type=game.0
                channel_id=game.1
                label=game.2
                white_id=game.3
                black_id=game.4
                finished=game.5
                selected=selected
                game_summary
            />
        </ShowLet>
        <ShowLet some=move || selected_tournament.get() let:tournament>
            <TournamentChannelActions
                selected=selected
                nanoid=tournament.0
                name=tournament.1
                is_participant=tournament.2
                chat
            />
        </ShowLet>
    }
}

#[component]
fn DmChannelActions(
    other_id: Uuid,
    username: String,
    blocked_ids: RwSignal<HashSet<Uuid>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let is_blocked = Signal::derive(move || blocked_ids.with(|ids| ids.contains(&other_id)));
    let on_block_toggle_success = Callback::new(move |is_now_blocked| {
        blocked_ids.update(|ids| {
            if is_now_blocked {
                ids.insert(other_id);
            } else {
                ids.remove(&other_id);
            }
        });
    });

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-wrap gap-2 items-center">
                <A
                    href=format!("/@/{}", username)
                    attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                >
                    {t!(i18n, messages.page.view_profile)}
                </A>
                <BlockToggleButton
                    blocked_user_id=other_id
                    is_blocked
                    on_success=on_block_toggle_success
                />
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn TournamentChannelActions(
    selected: RwSignal<Option<SelectedChannel>>,
    nanoid: String,
    name: String,
    is_participant: bool,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let nanoid_for_href = nanoid.clone();
    let nanoid_for_muted = nanoid.clone();
    let nanoid_for_action = nanoid.clone();
    let name_for_action = name.clone();
    let is_muted = Signal::derive(move || {
        selected.with(|current| {
            matches!(
                current,
                Some(SelectedChannel::Tournament {
                    nanoid: current_nanoid,
                    muted: true,
                    ..
                }) if *current_nanoid == nanoid_for_muted
            )
        })
    });
    let selected_for_action = selected;
    let chat_for_action = chat;
    let is_participant_for_action = is_participant;
    let toggle_mute = Action::new(move |currently_muted: &bool| {
        let currently_muted = *currently_muted;
        let nanoid = nanoid_for_action.clone();
        let name = name_for_action.clone();
        async move {
            let new_muted = if currently_muted {
                if unmute_tournament_chat(nanoid.clone()).await.is_err() {
                    return;
                }
                false
            } else {
                if mute_tournament_chat(nanoid.clone()).await.is_err() {
                    return;
                }
                true
            };
            let should_update_selection = selected_for_action.with_untracked(|current| {
                matches!(
                    current,
                    Some(SelectedChannel::Tournament { nanoid: current_nanoid, .. })
                        if *current_nanoid == nanoid
                )
            });
            if should_update_selection {
                selected_for_action.set(Some(SelectedChannel::Tournament {
                    nanoid,
                    name,
                    is_participant: is_participant_for_action,
                    muted: new_muted,
                }));
            }
            chat_for_action.invalidate_conversation_list();
            chat_for_action.refresh_unread_counts();
        }
    });

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-1">
                <div class="flex flex-wrap gap-2 items-center">
                    <A
                        href=format!("/tournament/{}", nanoid_for_href)
                        attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                    >
                        {t!(i18n, messages.page.view_tournament)}
                    </A>
                    <button
                        type="button"
                        disabled=move || toggle_mute.pending().get()
                        class="text-sm font-medium text-gray-600 transition-colors dark:text-gray-400 disabled:text-gray-400 dark:hover:text-pillbug-teal/90 dark:disabled:text-gray-500 hover:text-pillbug-teal"
                        on:click=move |_| {
                            toggle_mute.dispatch(is_muted.get_untracked());
                        }
                    >
                        {move || {
                            if is_muted.get() {
                                t_string!(i18n, messages.page.unmute_tournament_chat)
                            } else {
                                t_string!(i18n, messages.page.mute_tournament_chat)
                            }
                        }}
                    </button>
                </div>
                <Show when=move || !is_participant>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                        {t!(i18n, messages.chat.tournament_read_restricted)}
                    </p>
                </Show>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn GameChatToggle(
    channel_type: ChannelType,
    channel_id: String,
    label: String,
    white_id: Uuid,
    black_id: Uuid,
    finished: bool,
    selected: RwSignal<Option<SelectedChannel>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let viewing_players = channel_type == ChannelType::GamePlayers;
    let channel_id_players = channel_id.clone();
    let label_players = label.clone();
    let switch_to_players = move |_| {
        selected.set(Some(SelectedChannel::Game {
            channel_type: ChannelType::GamePlayers,
            channel_id: channel_id_players.clone(),
            label: label_players.clone(),
            white_id,
            black_id,
            finished,
        }));
    };
    let switch_to_spectators = move |_| {
        selected.set(Some(SelectedChannel::Game {
            channel_type: ChannelType::GameSpectators,
            channel_id: channel_id.clone(),
            label: label.clone(),
            white_id,
            black_id,
            finished,
        }));
    };
    view! {
        <div class="flex p-0.5 bg-gray-100 rounded-lg border border-gray-300 dark:bg-gray-800 dark:border-gray-600">
            <button
                type="button"
                class=move || {
                    format!(
                        "flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                        if viewing_players {
                            "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                        } else {
                            "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                        },
                    )
                }
                on:click=switch_to_players
            >
                {t!(i18n, messages.chat.players)}
            </button>
            <button
                type="button"
                disabled=move || !finished
                class=move || {
                    format!(
                        "flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                        if !viewing_players {
                            "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                        } else if finished {
                            "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                        } else {
                            "text-gray-400 dark:text-gray-500 cursor-not-allowed"
                        },
                    )
                }
                on:click=switch_to_spectators
            >
                {t!(i18n, messages.chat.spectators)}
            </button>
        </div>
    }
}

#[component]
fn GameChatHeader(
    channel_type: ChannelType,
    channel_id: String,
    label: String,
    white_id: Uuid,
    black_id: Uuid,
    finished: bool,
    selected: RwSignal<Option<SelectedChannel>>,
    game_summary: Signal<Option<Result<GameResponse, ServerFnError>>>,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let current_user_id = move || auth.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid));
    let is_player = move || current_user_id().is_some_and(|uid| uid == white_id || uid == black_id);
    let channel_type_for_label = channel_type.clone();
    let static_chat_label = Signal::derive(move || {
        if channel_type_for_label == ChannelType::GamePlayers {
            t_string!(i18n, messages.chat.players_chat)
        } else {
            t_string!(i18n, messages.chat.spectator_chat)
        }
    });
    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-2">
                <Show
                    when=move || is_player() && finished
                    fallback=move || {
                        view! {
                            <div class="flex flex-col gap-1">
                                <span class="inline-flex items-center py-1 px-2.5 text-xs font-medium text-gray-700 bg-white rounded-full border border-gray-300 dark:text-gray-200 dark:bg-gray-800 dark:border-gray-600 w-fit">
                                    {move || static_chat_label.get()}
                                </span>
                                <Show when=move || is_player() && !finished>
                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                        {t!(i18n, messages.chat.spectator_unlock)}
                                    </p>
                                </Show>
                            </div>
                        }
                    }
                >
                    <GameChatToggle
                        channel_type=channel_type.clone()
                        channel_id=channel_id.clone()
                        label=label.clone()
                        white_id=white_id
                        black_id=black_id
                        finished=finished
                        selected=selected
                    />
                </Show>
                <ShowLet
                    some=move || game_summary.get()
                    let:game_result
                    fallback=move || {
                        view! {
                            <div class="text-sm text-gray-500 animate-pulse dark:text-gray-400">
                                {t!(i18n, messages.page.loading)}
                            </div>
                        }
                    }
                >
                    <ShowLet
                        some=move || game_result.as_ref().ok().cloned()
                        let:game_data
                        fallback=move || {
                            view! {
                                <div class="text-sm text-red-600 dark:text-red-400">
                                    {t!(i18n, messages.page.failed_game)}
                                </div>
                            }
                        }
                    >
                        <GameChatSummary game=game_data />
                    </ShowLet>
                </ShowLet>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn GameChatSummary(game: GameResponse) -> impl IntoView {
    let i18n = use_i18n();
    let time_info = TimeInfo {
        mode: game.time_mode,
        base: game.time_base,
        increment: game.time_increment,
    };
    let game_finished = game.finished;
    let game_status = game.game_status.clone();
    let conclusion = game.conclusion.clone();
    let white_username = game.white_player.username.clone();
    let black_username = game.black_player.username.clone();
    let tournament_game_result = game.tournament_game_result.to_string();
    let state_label = Signal::derive(move || {
        if game_finished {
            t_string!(i18n, messages.page.state_finished)
        } else {
            t_string!(i18n, messages.page.state_started)
        }
    });
    let result_text = Signal::derive(move || {
        if !game_finished {
            return None;
        }
        let detail = match (&game_status, &conclusion) {
            (GameStatus::Finished(GameResult::Winner(color)), conclusion) => {
                let winner = if *color == Color::White {
                    white_username.clone()
                } else {
                    black_username.clone()
                };
                t_string!(
                    i18n,
                    messages.page.result_won,
                    winner = winner,
                    conclusion = conclusion.pretty_string()
                )
            }
            (GameStatus::Finished(GameResult::Draw), conclusion) => {
                t_string!(
                    i18n,
                    messages.page.result_draw,
                    conclusion = conclusion.pretty_string()
                )
            }
            (GameStatus::Adjudicated, _) => tournament_game_result.clone(),
            _ => String::new(),
        };
        (!detail.is_empty()).then_some(detail)
    });
    let created = game.created_at.format("%Y-%m-%d %H:%M").to_string();
    let nanoid = game.game_id.0.clone();

    view! {
        <div class="flex flex-wrap gap-y-1 gap-x-3 items-center text-sm">
            <span class="font-medium text-gray-700 dark:text-gray-300">
                {move || state_label.get()}
            </span>
            <ShowLet some=move || result_text.get() let:text>
                <span class="text-gray-600 dark:text-gray-400">{text}</span>
            </ShowLet>
            <A
                href=format!("/game/{}", nanoid.clone())
                attr:class="font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
            >
                {t!(i18n, messages.page.view_game)}
            </A>
            <span
                class="font-mono text-xs text-gray-500 dark:text-gray-400"
                title=move || t_string!(i18n, messages.page.game_id_tooltip)
            >
                {nanoid}
            </span>
            <TimeRow time_info extend_tw_classes="text-gray-600 dark:text-gray-400" />
            <span class="text-xs text-gray-500 dark:text-gray-400">{created}</span>
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
const EMPTY_HINT_CLASS: &str = "px-2 py-1.5 text-sm text-gray-400 dark:text-gray-500 italic";
const CHANNEL_BTN_BASE: &str =
    "w-full text-left px-3 py-2 rounded-lg flex justify-between items-center gap-2 \
    transition-colors duration-150 truncate text-sm min-h-[2.75rem]";
const CHANNEL_BTN_SELECTED: &str =
    "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium";
const CHANNEL_BTN_IDLE: &str =
    "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300";

fn channel_button_class(is_selected: bool) -> String {
    format!(
        "{} {}",
        CHANNEL_BTN_BASE,
        if is_selected {
            CHANNEL_BTN_SELECTED
        } else {
            CHANNEL_BTN_IDLE
        }
    )
}

#[component]
fn ChannelLists(
    conv: MyConversations,
    current_user_id: impl Fn() -> Option<Uuid> + 'static,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let me = current_user_id();
    let MyConversations {
        dms,
        tournaments,
        games,
        has_global: _has_global,
    } = conv;
    view! {
        <DmChannelsSection dms=dms me=me selected=selected chat=chat />
        <TournamentChannelsSection tournaments=tournaments selected=selected chat=chat />
        <GameChannelsSection games=games selected=selected chat=chat />
        <GlobalChannelSection selected=selected />
    }
}

#[component]
fn SectionHeaderButton(title: Signal<String>, open: RwSignal<bool>) -> impl IntoView {
    view! {
        <button type="button" class=SECTION_HEADER_BTN on:click=move |_| open.update(|o| *o = !*o)>
            <span>{move || title.get()}</span>
            <span class="opacity-70 text-[0.65rem]">
                {move || if open.get() { "▼" } else { "▶" }}
            </span>
        </button>
    }
}

#[component]
fn DmChannelsSection(
    dms: Vec<DmConversation>,
    me: Option<Uuid>,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let dms = std::sync::Arc::new(dms);
    let is_empty = dms.is_empty();
    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.dms)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty
                        .then(|| {
                            view! {
                                <p class=EMPTY_HINT_CLASS>{t!(i18n, messages.sections.no_dms)}</p>
                            }
                        })}
                    <For
                        each={
                            let dms = dms.clone();
                            move || (*dms).clone()
                        }
                        key=|dm| dm.other_user_id
                        children=move |dm| {
                            view! { <DmChannelItem dm=dm me=me selected=selected chat=chat /> }
                        }
                    />
                </div>
            </Show>
        </section>
    }
}

#[component]
fn DmChannelItem(
    dm: DmConversation,
    me: Option<Uuid>,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let DmConversation {
        other_user_id,
        username,
        ..
    } = dm;
    let username_for_selection = username.clone();
    let is_selected = move || {
        selected
            .get()
            .as_ref()
            .is_some_and(|s| {
                matches!(s, SelectedChannel::Dm { other_id: id, .. } if *id == other_user_id)
            })
    };
    let unread = Signal::derive(move || {
        me.map(|uid| chat.unread_count_for_dm(other_user_id, uid))
            .unwrap_or(0)
    });
    view! {
        <button
            type="button"
            class=move || channel_button_class(is_selected())
            on:click=move |_| {
                selected.set(Some(SelectedChannel::Dm {
                    other_id: other_user_id,
                    username: username_for_selection.clone(),
                }));
            }
        >
            <span class="truncate">{username}</span>
            <ChannelUnreadBadge unread=unread />
        </button>
    }
}

#[component]
fn TournamentChannelsSection(
    tournaments: Vec<TournamentChannel>,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let tournaments = std::sync::Arc::new(tournaments);
    let is_empty = tournaments.is_empty();
    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.tournaments)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty
                        .then(|| {
                            view! {
                                <p class=EMPTY_HINT_CLASS>
                                    {t!(i18n, messages.sections.no_tournament_chats)}
                                </p>
                            }
                        })}
                    <For
                        each={
                            let tournaments = tournaments.clone();
                            move || (*tournaments).clone()
                        }
                        key=|tournament| tournament.nanoid.clone()
                        children=move |tournament| {
                            view! {
                                <TournamentChannelItem
                                    tournament=tournament
                                    selected=selected
                                    chat=chat
                                />
                            }
                        }
                    />
                </div>
            </Show>
        </section>
    }
}

#[component]
fn TournamentChannelItem(
    tournament: TournamentChannel,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let TournamentChannel {
        nanoid,
        name,
        is_participant,
        muted,
        ..
    } = tournament;
    let nanoid_for_selection = nanoid.clone();
    let name_for_selection = name.clone();
    let nanoid_for_match = nanoid.clone();
    let nanoid_for_current_mute = nanoid.clone();
    let tournament_id = TournamentId(nanoid.clone());
    let is_selected = move || {
        selected.get().as_ref().is_some_and(
            |s| {
                matches!(
                    s,
                    SelectedChannel::Tournament {
                        nanoid: channel_nanoid,
                        ..
                    } if *channel_nanoid == nanoid_for_match
                )
            },
        )
    };
    let unread = Signal::derive(move || chat.unread_count_for_tournament(&tournament_id));
    view! {
        <button
            type="button"
            class=move || channel_button_class(is_selected())
            on:click=move |_| {
                let muted_for_selection = selected
                    .with_untracked(|current| {
                        match current {
                            Some(SelectedChannel::Tournament {
                                nanoid: current_nanoid,
                                muted: current_muted,
                                ..
                            }) if *current_nanoid == nanoid_for_current_mute => *current_muted,
                            _ => muted,
                        }
                    });
                selected.set(Some(SelectedChannel::Tournament {
                    nanoid: nanoid_for_selection.clone(),
                    name: name_for_selection.clone(),
                    is_participant,
                    muted: muted_for_selection,
                }));
            }
        >
            <span class="flex gap-1 items-center truncate">
                {name}
                {muted
                    .then(|| {
                        view! {
                            <span
                                class="text-gray-400 uppercase dark:text-gray-500 shrink-0 text-[0.65rem]"
                                title=move || t_string!(i18n, messages.sections.muted)
                            >
                                {t!(i18n, messages.sections.muted)}
                            </span>
                        }
                    })}
            </span>
            <ChannelUnreadBadge unread=unread />
        </button>
    }
}

#[component]
fn GameChannelsSection(
    games: Vec<GameChannel>,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let games = std::sync::Arc::new(games);
    let is_empty = games.is_empty();
    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.games)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty
                        .then(|| {
                            view! {
                                <p class=EMPTY_HINT_CLASS>
                                    {t!(i18n, messages.sections.no_game_chats)}
                                </p>
                            }
                        })}
                    <For
                        each={
                            let games = games.clone();
                            move || (*games).clone()
                        }
                        key=|game| format!("{}::{}", game.channel_type, game.channel_id)
                        children=move |game| {
                            view! { <GameChannelItem game=game selected=selected chat=chat /> }
                        }
                    />
                </div>
            </Show>
        </section>
    }
}

#[component]
fn GameChannelItem(
    game: GameChannel,
    selected: RwSignal<Option<SelectedChannel>>,
    chat: Chat,
) -> impl IntoView {
    let GameChannel {
        channel_type,
        channel_id,
        label,
        white_id,
        black_id,
        finished,
        ..
    } = game;
    let display_label = label
        .rsplit_once(" (")
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| label.clone());
    let display_label_with_nanoid = format!("{} ({})", display_label, channel_id);
    let channel_id_for_match = channel_id.clone();
    let channel_id_for_selection = channel_id.clone();
    let label_for_selection = label.clone();
    let game_id = GameId(channel_id.clone());
    let parsed_channel_type = channel_type.parse::<ChannelType>().ok();
    let is_selected = move || {
        selected.get().as_ref().is_some_and(
            |s| {
                matches!(
                    s,
                    SelectedChannel::Game { channel_id: cid, .. } if *cid == channel_id_for_match
                )
            },
        )
    };
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id));
    view! {
        <button
            type="button"
            class=move || channel_button_class(is_selected())
            on:click=move |_| {
                let Some(channel_type) = parsed_channel_type else {
                    return;
                };
                selected.set(Some(SelectedChannel::Game {
                    channel_type,
                    channel_id: channel_id_for_selection.clone(),
                    label: label_for_selection.clone(),
                    white_id,
                    black_id,
                    finished,
                }));
            }
        >
            <span class="truncate" title=display_label_with_nanoid.clone()>
                {display_label_with_nanoid.clone()}
            </span>
            <ChannelUnreadBadge unread=unread />
        </button>
    }
}

#[component]
fn GlobalChannelSection(selected: RwSignal<Option<SelectedChannel>>) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let is_selected = move || {
        selected
            .get()
            .as_ref()
            .is_some_and(|s| matches!(s, SelectedChannel::Global))
    };
    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || {
                    t_string!(i18n, messages.sections.recent_announcements)
                }).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    <button
                        type="button"
                        class=move || channel_button_class(is_selected())
                        on:click=move |_| selected.set(Some(SelectedChannel::Global))
                    >
                        {t!(i18n, messages.sections.recent_announcements)}
                    </button>
                </div>
            </Show>
        </section>
    }
}

#[component]
fn ChannelUnreadBadge(unread: Signal<i64>) -> impl IntoView {
    view! {
        <Show when=move || unread.get() != 0>
            <span class="flex justify-center items-center px-1.5 h-5 text-xs font-medium leading-none text-white rounded-full dark:bg-red-500 shrink-0 min-w-5 bg-ladybug-red">
                {move || {
                    let count = unread.get();
                    if count > 99 { "99+".to_string() } else { count.to_string() }
                }}
            </span>
        </Show>
    }
}
