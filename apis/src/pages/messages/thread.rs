use crate::{
    components::{
        molecules::empty_state::EmptyState,
        organisms::chat::{ComposerMode, ResolvedChatWindow},
    },
    i18n::*,
    providers::{AuthContext, AuthIdentity},
};
use leptos::{
    either::{Either, EitherOf3},
    prelude::*,
};
use leptos_icons::Icon;
use leptos_router::components::A;
use shared_types::{ConversationKey, GameChatCapabilities, GameId, GameThread, TournamentId};
use uuid::Uuid;

use super::{
    actions::{DmActions, GameActions, TournamentActions},
    MESSAGES_PRIMARY_HEADER_CLASS,
    MESSAGE_ROOT_PATH,
};

const MESSAGES_THREAD_PANE_CLASS: &str =
    "flex min-h-0 flex-1 flex-col overflow-hidden bg-light dark:bg-surface-muted";
const MESSAGES_INDEX_PANE_CLASS: &str =
    "hidden min-h-0 flex-1 flex-col overflow-hidden bg-light dark:bg-surface-muted sm:flex";
const MESSAGES_CHAT_BODY_CLASS: &str =
    "overflow-hidden flex-1 min-h-0 bg-even-light/95 dark:bg-surface-panel";

#[component]
pub fn MessagesIndex() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class=MESSAGES_INDEX_PANE_CLASS>
            <div class="flex flex-1 justify-center items-center p-4">
                <EmptyState
                    title=move || t_string!(i18n, messages.page.select_conversation).to_string()
                    message=move || t_string!(i18n, messages.page.choose_conversation).to_string()
                    class="max-w-sm"
                />
            </div>
        </div>
    }
}

#[component]
pub fn MessagesGlobalThread() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let composer_mode = Signal::derive(move || {
        if auth.admin.get() == Some(true) {
            ComposerMode::Enabled
        } else {
            ComposerMode::AdminOnly
        }
    });
    view! {
        <MessagesThreadFrame title=Signal::derive(move || {
            t_string!(i18n, messages.sections.recent_announcements).to_string()
        })>
            <div class=MESSAGES_CHAT_BODY_CLASS>
                <ResolvedChatWindow conversation=ConversationKey::Global composer_mode />
            </div>
        </MessagesThreadFrame>
    }
}

#[component]
pub(super) fn MessagesResolvedDmView(
    loading_message: Signal<String>,
    failed_message: Signal<String>,
    other_user_id: Uuid,
    username: String,
    peer_deleted: bool,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let title = if peer_deleted {
        t_string!(i18n, messages.chat.deleted_user).to_string()
    } else {
        username.clone()
    };
    let composer_mode = Signal::derive(move || {
        if peer_deleted {
            ComposerMode::PeerUnavailable
        } else {
            ComposerMode::Enabled
        }
    });
    let conversation = ConversationKey::Direct(other_user_id);
    let current_user_id =
        Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let unavailable_message = Signal::derive(move || {
        if auth.identity.get().is_none() {
            loading_message.get()
        } else {
            failed_message.get()
        }
    });
    let self_dm_error =
        Signal::derive(move || t_string!(i18n, messages.chat.self_dm_unsupported).to_string());
    view! {
        <MessagesThreadFrame title=Signal::derive(move || {
            title.clone()
        })>
            {move || match current_user_id.get() {
                Some(current_user_id) if current_user_id != other_user_id => {
                    EitherOf3::A(
                        view! {
                            <DmActions other_user_id username=username.clone() peer_deleted />
                            <div class=MESSAGES_CHAT_BODY_CLASS>
                                <ResolvedChatWindow
                                    conversation=conversation.clone()
                                    composer_mode
                                />
                            </div>
                        },
                    )
                }
                Some(_) => EitherOf3::B(view! { <MessagesStatusContent message=self_dm_error /> }),
                None => {
                    EitherOf3::C(view! { <MessagesStatusContent message=unavailable_message /> })
                }
            }}
        </MessagesThreadFrame>
    }
}

#[component]
pub(super) fn MessagesResolvedTournamentView(
    tournament_id: TournamentId,
    title: String,
) -> impl IntoView {
    let conversation = ConversationKey::Tournament(tournament_id.clone());
    view! {
        <MessagesThreadFrame title=Signal::derive(move || title.clone())>
            <TournamentActions tournament_id=tournament_id />
            <div class=MESSAGES_CHAT_BODY_CLASS>
                <ResolvedChatWindow conversation />
            </div>
        </MessagesThreadFrame>
    }
}

#[component]
pub(super) fn MessagesResolvedGameView(
    game_id: GameId,
    thread: GameThread,
    access: GameChatCapabilities,
) -> impl IntoView {
    let i18n = use_i18n();
    let conversation = ConversationKey::game(&game_id, thread);
    let title = Signal::derive(move || match thread {
        GameThread::Players => t_string!(i18n, messages.chat.players_chat).to_string(),
        GameThread::Spectators => t_string!(i18n, messages.chat.spectator_chat).to_string(),
    });
    view! {
        <MessagesThreadFrame title>
            <GameActions game_id thread access />
            {if access.can_read(thread) {
                Either::Left(
                    view! {
                        <div class=MESSAGES_CHAT_BODY_CLASS>
                            <ResolvedChatWindow conversation />
                        </div>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <MessagesStatusContent message=Signal::derive(move || {
                            if thread == GameThread::Players {
                                t_string!(i18n, messages.chat.players_chat_only).to_string()
                            } else {
                                t_string!(i18n, messages.chat.spectator_unlock).to_string()
                            }
                        }) />
                    },
                )
            }}
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesThreadFrame(title: Signal<String>, children: Children) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class=MESSAGES_THREAD_PANE_CLASS>
            <div class=MESSAGES_PRIMARY_HEADER_CLASS>
                <A
                    href=MESSAGE_ROOT_PATH
                    prop:replace=true
                    scroll=false
                    attr:class="no-link-style ui-button ui-button-secondary ui-button-sm flex-shrink-0 sm:hidden"
                    attr:aria-label=move || t_string!(i18n, messages.page.back_to_conversations)
                >
                    <Icon icon=icondata_lu::LuArrowLeft attr:class="size-4 shrink-0" />
                    <span>{t!(i18n, messages.page.conversations)}</span>
                </A>
                <h2 class="flex-1 min-w-0 text-lg font-bold text-gray-900 dark:text-gray-100 truncate">
                    {move || title.get()}
                </h2>
            </div>
            {children()}
        </div>
    }
}

#[component]
pub(super) fn MessagesStatusFrame(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] message: Signal<String>,
    #[prop(optional)] retry: Option<Callback<web_sys::MouseEvent>>,
) -> impl IntoView {
    view! {
        <MessagesThreadFrame title>
            <MessagesStatusContent message retry />
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesStatusContent(
    #[prop(into)] message: Signal<String>,
    #[prop(optional_no_strip)] retry: Option<Callback<web_sys::MouseEvent>>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex flex-1 justify-center items-center p-4 bg-even-light/95 dark:bg-surface-panel">
            <div class="max-w-sm ui-empty-state" role="status">
                <p class="text-sm font-medium">{move || message.get()}</p>
                <ShowLet some=move || retry let:retry>
                    <button
                        type="button"
                        class="mt-3 ui-button ui-button-secondary ui-button-sm"
                        on:click=move |event| retry.run(event)
                    >
                        {t!(i18n, messages.chat.retry)}
                    </button>
                </ShowLet>
            </div>
        </div>
    }
}
