use crate::{
    components::{
        atoms::block_toggle_button::BlockToggleButton,
        molecules::game_thread_toggle::{GameThreadToggle, GameThreadToggleSize},
    },
    functions::blocks_mutes::set_tournament_chat_muted,
    i18n::*,
    providers::chat::{Chat, ChatSessionToken},
};
use leptos::{either::Either, prelude::*};
use leptos_router::{components::A, hooks::use_navigate, NavigateOptions};
use shared_types::{GameChatCapabilities, GameId, GameThread, TournamentId};
use uuid::Uuid;

use super::message_game_href;

const HEADER_ACTION_BUTTON_PRIMARY: &str =
    "no-link-style ui-button ui-button-secondary ui-button-sm";
const MESSAGES_SUBHEADER_CLASS: &str = "flex shrink-0 items-center justify-between gap-3 border-b border-black/10 bg-light px-3 py-2.5 dark:border-white/10 dark:bg-surface-muted xs:px-4 xs:py-3";

#[derive(Clone)]
struct TournamentMuteRequest {
    tournament_id: TournamentId,
    currently_muted: bool,
    session: ChatSessionToken,
}

#[component]
pub(super) fn DmActions(
    other_user_id: Uuid,
    username: String,
    peer_deleted: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let profile_href = format!("/@/{username}");
    view! {
        <div class=MESSAGES_SUBHEADER_CLASS>
            {if peer_deleted {
                Either::Left(
                    view! {
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            {t!(i18n, messages.chat.deleted_account)}
                        </p>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <div class="flex flex-wrap gap-2 items-center">
                            <A href=profile_href attr:class=HEADER_ACTION_BUTTON_PRIMARY>
                                {t!(i18n, messages.page.view_profile)}
                            </A>
                            <BlockToggleButton blocked_user_id=other_user_id />
                        </div>
                    },
                )
            }}
        </div>
    }
}

#[component]
pub(super) fn TournamentActions(tournament_id: TournamentId) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let tournament_id = StoredValue::new(tournament_id);
    let muted = chat.tournament_muted_signal(tournament_id.get_value());
    let error = RwSignal::new(None::<String>);
    let toggle = Action::new(move |request: &TournamentMuteRequest| {
        let request = request.clone();
        async move {
            let result = set_tournament_chat_muted(
                request.tournament_id.0.clone(),
                !request.currently_muted,
            )
            .await
            .map_err(|error| error.to_string());
            (request, result)
        }
    });
    Effect::watch(
        toggle.version(),
        move |_, _, _| {
            let Some((request, result)) = toggle.value().get_untracked() else {
                return;
            };
            if !chat.is_current(request.session) {
                return;
            }
            match result {
                Ok(new_muted) => {
                    error.set(None);
                    chat.set_tournament_muted_authoritative(&request.tournament_id, new_muted);
                }
                Err(err) => error.set(Some(err)),
            }
        },
        false,
    );
    let button_label = Signal::derive(move || {
        if toggle.pending().get() {
            t_string!(i18n, messages.page.loading)
        } else if muted.get() {
            t_string!(i18n, messages.page.unmute_tournament_chat)
        } else {
            t_string!(i18n, messages.page.mute_tournament_chat)
        }
    });
    view! {
        <div class=MESSAGES_SUBHEADER_CLASS>
            <div class="flex flex-wrap gap-2 items-center">
                <A
                    href=move || format!("/tournament/{}", tournament_id.get_value().0)
                    attr:class=HEADER_ACTION_BUTTON_PRIMARY
                >
                    {t!(i18n, messages.page.view_tournament)}
                </A>
                <button
                    type="button"
                    disabled=toggle.pending()
                    aria-busy=move || toggle.pending().get().to_string()
                    class=move || {
                        if muted.get() {
                            "ui-button ui-button-secondary ui-button-sm"
                        } else {
                            "ui-button ui-button-danger ui-button-sm"
                        }
                    }
                    on:click=move |_| {
                        error.set(None);
                        let Some(session) = chat.current_session_token() else {
                            return;
                        };
                        toggle
                            .dispatch(TournamentMuteRequest {
                                tournament_id: tournament_id.get_value(),
                                currently_muted: muted.get_untracked(),
                                session,
                            });
                    }
                >
                    {button_label}
                </button>
                <ShowLet some=move || error.get() let:error>
                    <span class="ui-field-error">{error}</span>
                </ShowLet>
            </div>
        </div>
    }
}

#[component]
pub(super) fn GameActions(
    game_id: GameId,
    thread: GameThread,
    access: GameChatCapabilities,
) -> impl IntoView {
    let i18n = use_i18n();
    let navigate = use_navigate();
    let selected = RwSignal::new(thread);
    let game_href = format!("/game/{}", game_id.0);
    let selected_game_id = game_id.clone();
    let on_select = Callback::new(move |thread| {
        let href = message_game_href(&selected_game_id, thread);
        navigate(
            &href,
            NavigateOptions {
                replace: true,
                scroll: false,
                ..Default::default()
            },
        );
    });
    let spectator_unlock_needed =
        access.can_toggle_embedded_threads() && !access.can_read(GameThread::Spectators);
    view! {
        <div class=MESSAGES_SUBHEADER_CLASS>
            <div class="flex flex-col gap-2">
                <div class="flex flex-wrap gap-2 items-center">
                    <A href=game_href attr:class=HEADER_ACTION_BUTTON_PRIMARY>
                        {t!(i18n, messages.page.view_game)}
                    </A>
                    <GameThreadToggle
                        selected
                        players_enabled=access.can_read(GameThread::Players)
                        spectators_enabled=access.can_read(GameThread::Spectators)
                        size=GameThreadToggleSize::Route
                        on_select
                    />
                </div>
                <Show when=move || spectator_unlock_needed>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                        {t!(i18n, messages.chat.spectator_unlock)}
                    </p>
                </Show>
            </div>
        </div>
    }
}
