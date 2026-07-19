use crate::{
    common::ChatSendError,
    i18n::*,
    providers::{
        chat::{Chat, ConversationHandle, InitialHistoryStatus, SendIssue},
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        AuthIdentity,
    },
};
use leptos::{html, prelude::*};
use shared_types::{ConversationKey, GameThread, MAX_CHAT_MESSAGE_LENGTH};

#[component]
pub fn ChatInput(
    conversation: ConversationHandle,
    #[prop(into)] mode: Signal<ComposerMode>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let game_state = use_context::<GameStateStore>();
    let key = StoredValue::new(conversation.key().clone());
    let turn = move || {
        let game_id = match key.get_value() {
            ConversationKey::Game { game_id, .. } => game_id,
            _ => return None,
        };
        game_state.and_then(|game_state| {
            if game_state.game_id().get_untracked().as_ref() != Some(&game_id) {
                return None;
            }
            Some(game_state.state().with_untracked(|state| state.turn))
        })
    };
    let displayed_draft = conversation.draft();
    let conversation = StoredValue::new(conversation);
    let send = move || {
        if mode.get() != ComposerMode::Enabled {
            return;
        }
        chat.send(&conversation.get_value(), turn());
    };
    let placeholder = move || match mode.get() {
        ComposerMode::AdminOnly => t_string!(i18n, messages.chat.admin_only).to_string(),
        ComposerMode::PeerUnavailable => {
            t_string!(i18n, messages.chat.peer_unavailable).to_string()
        }
        ComposerMode::Enabled => match key.get_value() {
            ConversationKey::Game {
                thread: GameThread::Players,
                ..
            } => t_string!(i18n, messages.chat.with_opponent).to_string(),
            ConversationKey::Game {
                thread: GameThread::Spectators,
                ..
            } => t_string!(i18n, messages.chat.with_spectators).to_string(),
            _ => t_string!(i18n, messages.chat.placeholder).to_string(),
        },
    };
    let composer_label = move || t_string!(i18n, messages.chat.composer_label).to_string();
    let input_ref = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = input_ref.get_untracked().map(|input| input.focus());
    });

    view! {
        <input
            node_ref=input_ref
            type="text"
            prop:disabled=move || mode.get() != ComposerMode::Enabled
            class="overscroll-contain py-3 disabled:opacity-50 disabled:cursor-not-allowed ui-field-input box-border shrink-0 touch-pan-x"
            prop:value=move || displayed_draft.get()
            prop:placeholder=placeholder
            aria-label=composer_label
            on:input=move |event| {
                chat.set_draft_message(&conversation.get_value(), event_target_value(&event));
            }
            on:keydown=move |event| {
                if event.key() == "Enter" && mode.get() == ComposerMode::Enabled {
                    event.prevent_default();
                    send();
                }
            }
            maxlength=MAX_CHAT_MESSAGE_LENGTH.to_string()
        />
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComposerMode {
    #[default]
    Enabled,
    AdminOnly,
    PeerUnavailable,
}

#[component]
pub(super) fn SendErrorMessage(error: SendIssue) -> impl IntoView {
    let i18n = use_i18n();
    match error {
        SendIssue::LoginRequired => t_string!(i18n, messages.chat.login_to_send).to_string(),
        SendIssue::ConnectionUnavailable => {
            t_string!(i18n, messages.chat.connection_unavailable).to_string()
        }
        SendIssue::Server(error) => match error {
            ChatSendError::ClientIdConflict => {
                t_string!(i18n, messages.chat.client_id_conflict).to_string()
            }
            ChatSendError::RateLimited => {
                t_string!(i18n, messages.chat.send_rate_limited).to_string()
            }
            ChatSendError::DirectRestricted => {
                t_string!(i18n, messages.chat.dm_send_restricted).to_string()
            }
            ChatSendError::AdminOnly => t_string!(i18n, messages.chat.admin_only).to_string(),
            ChatSendError::TournamentRestricted => {
                t_string!(i18n, messages.chat.tournament_send_restricted).to_string()
            }
            ChatSendError::PlayersRestricted => {
                t_string!(i18n, messages.chat.players_send_restricted).to_string()
            }
            ChatSendError::SpectatorsRestricted => {
                t_string!(i18n, messages.chat.spectator_send_restricted).to_string()
            }
            ChatSendError::Unavailable => {
                t_string!(i18n, messages.chat.delivery_failed).to_string()
            }
        },
    }
}

#[component]
pub(super) fn Composer(
    conversation: ConversationHandle,
    #[prop(into)] composer_mode: Signal<ComposerMode>,
    compact: bool,
    visible_thread_error: Signal<Option<String>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let initial_history = conversation.initial();
    let visible_send_error = conversation.send_error();
    let conversation = StoredValue::new(conversation);
    let composer_class = if compact {
        "ui-chat-composer p-2"
    } else {
        "ui-chat-composer p-3"
    };
    let composer_inner_class = "mx-auto w-full max-w-6xl";

    view! {
        <div class=composer_class>
            <div class=composer_inner_class>
                <Show
                    when=move || { initial_history.get() != InitialHistoryStatus::AccessDenied }
                    fallback=move || {
                        view! {
                            <Show when=move || visible_thread_error.get().is_some()>
                                <div class="ui-warning-notice" role="alert">
                                    {move || visible_thread_error.get().unwrap_or_default()}
                                </div>
                            </Show>
                        }
                    }
                >
                    <div class="flex flex-col gap-2">
                        <ShowLet
                            some={
                                let visible_send_error = visible_send_error.clone();
                                move || visible_send_error.get()
                            }
                            let:error
                        >
                            <div class="ui-danger-notice" role="alert">
                                <SendErrorMessage error />
                            </div>
                        </ShowLet>
                        <Show
                            when=move || auth.identity.get().is_some()
                            fallback=move || {
                                view! {
                                    <div
                                        class="py-3 text-sm text-center ui-field-helper"
                                        aria-busy="true"
                                    >
                                        {t!(i18n, messages.page.loading)}
                                    </div>
                                }
                            }
                        >
                            <Show
                                when=move || {
                                    matches!(auth.identity.get(), Some(AuthIdentity::User(_)))
                                }
                                fallback=move || {
                                    view! {
                                        <a
                                            href="/login"
                                            class="w-full ui-button ui-button-primary ui-button-md min-h-11"
                                        >
                                            {t!(i18n, messages.chat.login_to_send)}
                                        </a>
                                    }
                                }
                            >
                                <ChatInput
                                    conversation=conversation.get_value()
                                    mode=composer_mode
                                />
                            </Show>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}
