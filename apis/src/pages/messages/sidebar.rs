use crate::{
    components::atoms::unread_badge::UnreadBadge,
    hooks::tap_feedback::use_tap_feedback,
    i18n::*,
    providers::chat::Chat,
};
use leptos::{either::Either, prelude::*};
use leptos_icons::Icon;
use leptos_router::components::A;
use shared_types::{
    ConversationKey,
    DmConversation,
    GameChannel,
    GameId,
    GameThread,
    TournamentChannel,
    TournamentId,
};
use uuid::Uuid;

use super::{
    catalog::MessagesCatalog,
    message_dm_href,
    message_game_href,
    message_path_is,
    message_path_matches_game,
    message_tournament_href,
    MESSAGE_GLOBAL_PATH,
};

const SECTION_LIST_MAX_H: &str = "overflow-y-auto min-h-0 max-h-48";
const SECTION_HEADER_BUTTON_CLASS: &str = "sticky top-0 z-10 flex justify-between items-center w-full text-xs text-left text-gray-500 uppercase rounded-none dark:text-gray-400 ui-disclosure-summary min-h-10";
const EMPTY_HINT_CLASS: &str = "py-1.5 px-2 text-sm italic text-gray-500 dark:text-gray-400";
const CHANNEL_BUTTON_BASE_CLASS: &str =
    "ui-messages-channel-link no-link-style flex min-h-10 w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm transition-colors duration-200";
const CHANNEL_BUTTON_SELECTED_CLASS: &str = "ui-segmented-active font-bold";
const CHANNEL_BUTTON_IDLE_CLASS: &str =
    "text-gray-800 hover:bg-blue-light/70 dark:text-gray-100 dark:hover:bg-pillbug-teal/15";

fn dm_channel_key(channel: &DmConversation) -> (Uuid, String, bool) {
    (
        channel.other_user_id,
        channel.username.clone(),
        channel.peer_deleted,
    )
}

fn tournament_channel_key(channel: &TournamentChannel) -> (TournamentId, String) {
    (channel.tournament_id.clone(), channel.name.clone())
}

fn game_channel_key(channel: &GameChannel) -> (GameId, String, bool) {
    (
        channel.game_id.clone(),
        channel.label.clone(),
        channel.finished,
    )
}

#[component]
pub(super) fn MessagesSidebar(current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let mark_channel_press = use_tap_feedback(".ui-messages-channel-link");
    let catalog = MessagesCatalog::new(chat);
    let dms_title = Signal::derive(move || t_string!(i18n, messages.sections.dms).to_string());
    let dms_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_dms).to_string());
    let tournaments_title =
        Signal::derive(move || t_string!(i18n, messages.sections.tournaments).to_string());
    let tournaments_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_tournament_chats).to_string());
    let games_title = Signal::derive(move || t_string!(i18n, messages.sections.games).to_string());
    let games_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_game_chats).to_string());
    let snapshot = catalog.snapshot();
    let dms = Signal::derive(move || {
        snapshot
            .get()
            .map_or_else(Vec::new, |snapshot| snapshot.dms.clone())
    });
    let tournaments = Signal::derive(move || {
        snapshot
            .get()
            .map_or_else(Vec::new, |snapshot| snapshot.tournaments.clone())
    });
    let games = Signal::derive(move || {
        snapshot
            .get()
            .map_or_else(Vec::new, |snapshot| snapshot.games.clone())
    });
    view! {
        <div class="flex flex-col" on:pointerdown=move |event| mark_channel_press.run(event)>
            <Show
                when=move || snapshot.get().is_some()
                fallback=move || {
                    if catalog.loading().get() {
                        Either::Left(
                            view! {
                                <p class="p-3 animate-pulse ui-field-helper">
                                    {t!(i18n, messages.page.loading)}
                                </p>
                            },
                        )
                    } else {
                        Either::Right(
                            view! {
                                <div class="p-3 space-y-2">
                                    <p class="ui-field-error">
                                        {t!(i18n, messages.page.failed_conversations)}
                                    </p>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        on:click=move |_| catalog.retry(chat)
                                    >
                                        {t!(i18n, messages.chat.retry)}
                                    </button>
                                </div>
                            },
                        )
                    }
                }
            >
                <GlobalChannelSection current_path />
                <ChannelListSection
                    title=dms_title
                    empty_label=dms_empty_label
                    items=dms
                    key=dm_channel_key
                    render_item=move |channel| {
                        view! { <DmChannelItem channel current_path /> }
                    }
                />
                <ChannelListSection
                    title=tournaments_title
                    empty_label=tournaments_empty_label
                    items=tournaments
                    key=tournament_channel_key
                    render_item=move |channel| {
                        view! { <TournamentChannelItem channel current_path /> }
                    }
                />
                <ChannelListSection
                    title=games_title
                    empty_label=games_empty_label
                    items=games
                    key=game_channel_key
                    render_item=move |channel| {
                        view! { <GameChannelItem channel current_path /> }
                    }
                />
            </Show>
        </div>
    }
}

fn dm_display_name(dm: &DmConversation, deleted_user: String) -> String {
    if dm.peer_deleted {
        deleted_user
    } else {
        dm.username.clone()
    }
}

#[component]
fn DmChannelItem(channel: DmConversation, current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let unread = chat.unread(&ConversationKey::direct(channel.other_user_id));
    let channel = StoredValue::new(channel);
    let href = Signal::derive(move || message_dm_href(&channel.get_value().username));
    let is_selected = Signal::derive(move || message_path_is(&current_path.get(), &href.get()));
    let unread = Signal::derive(move || unread.get().count);
    let unread_label = Signal::derive(move || {
        let count = unread.get();
        let channel = channel.get_value();
        let conversation = dm_display_name(
            &channel,
            t_string!(i18n, messages.chat.deleted_user).to_string(),
        );
        t_string!(
            i18n,
            messages.chat.unread_badge,
            count = count,
            conversation = conversation
        )
        .to_string()
    });
    view! {
        <MessagesChannelLink href is_selected>
            <span class="truncate">
                {move || dm_display_name(
                    &channel.get_value(),
                    t_string!(i18n, messages.chat.deleted_user).to_string(),
                )}
            </span>
            <UnreadBadge count=unread aria_label=unread_label />
        </MessagesChannelLink>
    }
}

#[component]
fn TournamentChannelItem(
    channel: TournamentChannel,
    current_path: Signal<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let unread = chat.unread(&ConversationKey::tournament(&channel.tournament_id));
    let tournament_id = StoredValue::new(channel.tournament_id.clone());
    let muted = chat.tournament_muted_signal(channel.tournament_id.clone());
    let channel = StoredValue::new(channel);
    let unread = Signal::derive(move || unread.get().count);
    let unread_label = Signal::derive(move || {
        let count = unread.get();
        let conversation = channel.get_value().name;
        t_string!(
            i18n,
            messages.chat.unread_badge,
            count = count,
            conversation = conversation
        )
        .to_string()
    });
    let href = Signal::derive(move || message_tournament_href(&tournament_id.get_value()));
    let is_selected = Signal::derive(move || message_path_is(&current_path.get(), &href.get()));
    view! {
        <MessagesChannelLink href is_selected>
            <span class="flex gap-1 items-center truncate">
                {move || channel.get_value().name} <Show when=muted>
                    <span
                        class="text-gray-400 uppercase dark:text-gray-500 shrink-0 text-[0.65rem]"
                        title=move || t_string!(i18n, messages.sections.muted)
                    >
                        {t!(i18n, messages.sections.muted)}
                    </span>
                </Show>
            </span>
            <UnreadBadge count=unread aria_label=unread_label />
        </MessagesChannelLink>
    }
}

#[component]
fn GameChannelItem(channel: GameChannel, current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let unread = chat.unread(&ConversationKey::game_players(&channel.game_id));
    let game_id = StoredValue::new(channel.game_id.clone());
    let channel = StoredValue::new(channel);
    let href = Signal::derive(move || message_game_href(&game_id.get_value(), GameThread::Players));
    let display_label_with_nanoid = Signal::derive(move || {
        format!("{} ({})", channel.get_value().label, game_id.get_value().0)
    });
    let unread = Signal::derive(move || unread.get().count);
    let unread_label = Signal::derive(move || {
        t_string!(
            i18n,
            messages.chat.unread_badge,
            count = unread.get(),
            conversation = display_label_with_nanoid.get()
        )
        .to_string()
    });
    let is_selected = Signal::derive(move || {
        message_path_matches_game(
            &current_path.get(),
            &game_id.get_value(),
            GameThread::Players,
            channel.get_value().finished,
        )
    });
    view! {
        <MessagesChannelLink href is_selected>
            <span class="truncate" title=move || display_label_with_nanoid.get()>
                {move || display_label_with_nanoid.get()}
            </span>
            <UnreadBadge count=unread aria_label=unread_label />
        </MessagesChannelLink>
    }
}

#[component]
fn GlobalChannelSection(current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let is_selected =
        Signal::derive(move || message_path_is(&current_path.get(), MESSAGE_GLOBAL_PATH));

    view! {
        <section class="flex flex-col min-h-0 ui-messages-sidebar-section">
            <MessagesChannelLink href=MESSAGE_GLOBAL_PATH.to_string() is_selected>
                <span class="truncate">{t!(i18n, messages.sections.recent_announcements)}</span>
            </MessagesChannelLink>
        </section>
    }
}

fn channel_button_class(is_selected: bool) -> String {
    format!(
        "{} {}",
        CHANNEL_BUTTON_BASE_CLASS,
        if is_selected {
            CHANNEL_BUTTON_SELECTED_CLASS
        } else {
            CHANNEL_BUTTON_IDLE_CLASS
        }
    )
}

#[component]
fn MessagesChannelLink(
    #[prop(into)] href: Signal<String>,
    is_selected: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <A
            href=move || href.get()
            prop:replace=true
            scroll=false
            attr:class=move || channel_button_class(is_selected.get())
            attr:aria-current=move || is_selected.get().then_some("page")
        >
            {children()}
        </A>
    }
}

#[component]
fn ChannelListSection<T, F, IV, KF, K>(
    title: Signal<String>,
    empty_label: Signal<String>,
    items: Signal<Vec<T>>,
    key: KF,
    render_item: F,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T) -> IV + Copy + Send + Sync + 'static,
    IV: IntoView + 'static,
    KF: Fn(&T) -> K + Copy + Send + Sync + 'static,
    K: Eq + std::hash::Hash + 'static,
{
    let open = RwSignal::new(true);

    view! {
        <section class="flex flex-col min-h-0 ui-messages-sidebar-section">
            <button
                type="button"
                class=SECTION_HEADER_BUTTON_CLASS
                attr:aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|state| *state = !*state)
            >
                <span>{move || title.get()}</span>
                {move || {
                    if open.get() {
                        Either::Left(
                            view! { <Icon icon=icondata_lu::LuChevronDown attr:class="size-4" /> },
                        )
                    } else {
                        Either::Right(
                            view! { <Icon icon=icondata_lu::LuChevronRight attr:class="size-4" /> },
                        )
                    }
                }}
            </button>
            <Show when=open>
                <div class=SECTION_LIST_MAX_H>
                    {move || {
                        if items.with(Vec::is_empty) {
                            Either::Left(
                                view! { <p class=EMPTY_HINT_CLASS>{move || empty_label.get()}</p> },
                            )
                        } else {
                            Either::Right(view! { <For each=items key=key children=render_item /> })
                        }
                    }}
                </div>
            </Show>
        </section>
    }
}
