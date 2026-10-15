use crate::{
    common::{
        challenge_displayed_player,
        challenge_is_viewable,
        challenge_viewer_role,
        ChallengeUpdate,
        ChallengeViewerRole,
        ServerMessage,
        ServerResult,
    },
    components::{
        atoms::challenge_details::ChallengeDetails,
        layouts::page_shell::PageShell,
        molecules::{empty_state::EmptyState, panel::Panel},
    },
    functions::challenges::get::get_challenge,
    hooks::clipboard_copy::use_clipboard_copy,
    i18n::{t, t_string, use_i18n},
    providers::{games::GamesSignal, websocket::WebsocketContext, AuthContext},
    responses::ChallengeResponse,
};
use leptos::{either::EitherOf3, prelude::*};
use leptos_icons::*;
use leptos_router::{hooks::use_params, params::Params};
use leptos_use::use_window;
use shared_types::{ChallengeId, ChallengeVisibility, GameId};

#[derive(Params, PartialEq, Eq)]
struct ChallengeParams {
    nanoid: String,
}

struct ChallengePageSummary {
    title: Signal<String>,
    message: Signal<String>,
    displayed_user_label: Signal<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChallengePageState {
    Active,
    GameStarted,
    Removed,
}

fn response_removed_challenge(message: &ServerResult, challenge_id: &ChallengeId) -> bool {
    matches!(
        message,
        ServerResult::Ok(message)
            if matches!(
                message.as_ref(),
                ServerMessage::Challenge(ChallengeUpdate::Removed(removed_id))
                    if removed_id == challenge_id
            )
    )
}

fn challenge_page_summary(
    challenge: &ChallengeResponse,
    viewer_role: ChallengeViewerRole,
    display_username: &str,
) -> ChallengePageSummary {
    let i18n = use_i18n();
    let display_username = StoredValue::new(display_username.to_string());
    let (title, message) = match (viewer_role, &challenge.visibility) {
        (ChallengeViewerRole::Challenger, ChallengeVisibility::Direct) => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.direct_challenge_to,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.waiting_for_to_respond,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
        ),
        (ChallengeViewerRole::Challenger, ChallengeVisibility::Private) => (
            Signal::derive(move || t_string!(i18n, home.challenge.private_challenge).to_string()),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge
                        .share_the_private_link_with_the_player_you_want
                )
                .to_string()
            }),
        ),
        (ChallengeViewerRole::Challenger, ChallengeVisibility::Public) => (
            Signal::derive(move || t_string!(i18n, home.challenge.public_challenge).to_string()),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge
                        .this_challenge_is_listed_publicly_while_it_is_open
                )
                .to_string()
            }),
        ),
        (ChallengeViewerRole::Opponent, ChallengeVisibility::Direct) => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.direct_challenge_from,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge
                        .review_the_settings_below_then_accept_or_decline
                )
                .to_string()
            }),
        ),
        (ChallengeViewerRole::Anonymous, ChallengeVisibility::Private) => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.private_challenge_from,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(i18n, home.challenge.log_in_to_accept_this_challenge).to_string()
            }),
        ),
        (ChallengeViewerRole::Anonymous, _) => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.challenge_from,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(i18n, home.challenge.log_in_to_accept_this_challenge).to_string()
            }),
        ),
        (_, ChallengeVisibility::Private) => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.private_challenge_from,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge
                        .review_the_settings_below_then_accept_when_you_re
                )
                .to_string()
            }),
        ),
        _ => (
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge.challenge_from,
                    display_username = display_username.get_value()
                )
                .to_string()
            }),
            Signal::derive(move || {
                t_string!(
                    i18n,
                    home.challenge
                        .review_the_settings_below_then_accept_when_you_re
                )
                .to_string()
            }),
        ),
    };
    let displayed_user_label =
        if viewer_role == ChallengeViewerRole::Challenger && challenge.opponent.is_some() {
            Signal::derive(move || t_string!(i18n, common.opponent_label).to_string())
        } else {
            Signal::derive(move || t_string!(i18n, home.challenge.challenger).to_string())
        };

    ChallengePageSummary {
        title,
        message,
        displayed_user_label,
    }
}

#[component]
fn ChallengeNotFound() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <EmptyState
            title=move || t_string!(i18n, home.challenge.challenge_not_found)
            message=move || {
                t_string!(i18n, home.challenge.the_challenge_you_re_looking_for_doesn_t_exist)
            }
        />
    }
}

#[component]
fn GameStarted(game_id: GameId) -> impl IntoView {
    let i18n = use_i18n();
    let game_path = format!("/game/{}", game_id.0);

    view! {
        <Panel
            title=move || t_string!(i18n, notifications.push.game_started_title)
            body_class="space-y-3"
        >
            <p class="text-sm text-gray-600 dark:text-gray-300">
                {t!(i18n, home.challenge.this_challenge_has_been_accepted_and_the_game_is)}
            </p>
            <a class="ui-button ui-button-primary ui-button-md no-link-style w-fit" href=game_path>
                {t!(i18n, common.open_game)}
            </a>
        </Panel>
    }
}

#[component]
pub fn ChallengeView() -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params::<ChallengeParams>();
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let identity = auth_context.identity;
    let games = expect_context::<GamesSignal>();
    let websocket = expect_context::<WebsocketContext>();
    let nanoid = Signal::derive(move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    });
    let challenge_id = Signal::derive(move || ChallengeId(nanoid()));
    let game_id = Signal::derive(move || GameId(nanoid()));
    let challenge = OnceResource::new(get_challenge(challenge_id.get_untracked()));
    let challenge_removed = RwSignal::new(false);
    Effect::new(move |_| {
        let Some(message) = websocket.message.get() else {
            return;
        };
        if response_removed_challenge(&message, &challenge_id.get_untracked()) {
            challenge_removed.set(true);
        }
    });
    let challenge_was_removed = Signal::derive(move || challenge_removed.get());
    let accepted_game_started = Signal::derive(move || {
        let id = game_id();
        games.own.with(|own| {
            own.realtime.contains_key(&id)
                || own.correspondence.contains_key(&id)
                || own.untimed.contains_key(&id)
        })
    });

    let challenge_address = move || {
        let origin = use_window()
            .as_ref()
            .and_then(|window| window.location().origin().ok())
            .unwrap_or_default();
        format!("{origin}/challenge/{}", nanoid())
    };
    let clipboard = use_clipboard_copy();
    let copy_state = clipboard.copied;
    let copy_text = clipboard.copy_text;
    let copy = move |_| copy_text(challenge_address());
    let copy_button_class = move || {
        if copy_state.get() {
            "ui-button ui-button-success ui-button-narrow"
        } else {
            "ui-button ui-button-secondary ui-button-narrow"
        }
    };
    view! {
        <PageShell>
            <Suspense fallback=move || {
                view! {
                    <EmptyState
                        title=move || t_string!(i18n, common.loading)
                        message=move || {
                            t_string!(
                                i18n, home.challenge.please_wait_while_we_fetch_the_challenge_details
                            )
                        }
                    />
                }
            }>
                <ErrorBoundary fallback=move |_errors| {
                    view! {
                        <EmptyState
                            title=move || t_string!(i18n, home.challenge.error_loading_challenge)
                            message=move || {
                                t_string!(i18n, home.challenge.challenge_doesn_t_seem_to_exist)
                            }
                        />
                    }
                }>
                    {move || {
                        challenge
                            .get()
                            .map(|data| match data {
                                Err(_) => EitherOf3::A(view! { <ChallengeNotFound /> }),
                                Ok(challenge) => {
                                    if challenge.visibility == ChallengeVisibility::Direct
                                        && identity.get().is_none()
                                    {
                                        return EitherOf3::B(
                                            view! {
                                                <EmptyState
                                                    title=move || t_string!(i18n, common.loading)
                                                    message=move || {
                                                        t_string!(
                                                            i18n, home.challenge.please_wait_while_we_check_your_account
                                                        )
                                                    }
                                                />
                                            },
                                        );
                                    }
                                    let viewer_id = user
                                        .with(|user| user.as_ref().map(|user| user.id));
                                    let viewer_role = challenge_viewer_role(&challenge, viewer_id);
                                    if !challenge_is_viewable(&challenge, viewer_role) {
                                        return EitherOf3::A(view! { <ChallengeNotFound /> });
                                    }
                                    let (display_user, _) = challenge_displayed_player(
                                        &challenge,
                                        viewer_role,
                                    );
                                    let display_username = display_user.username.clone();
                                    let summary = challenge_page_summary(
                                        &challenge,
                                        viewer_role,
                                        &display_username,
                                    );
                                    let show_private_copy_link = viewer_role
                                        == ChallengeViewerRole::Challenger
                                        && challenge.visibility == ChallengeVisibility::Private;
                                    let show_copy_panel = Signal::derive(move || {
                                        show_private_copy_link && !challenge_was_removed()
                                            && !accepted_game_started()
                                    });
                                    let page_state = Signal::derive(move || {
                                        if accepted_game_started() {
                                            ChallengePageState::GameStarted
                                        } else if challenge_was_removed() {
                                            ChallengePageState::Removed
                                        } else {
                                            ChallengePageState::Active
                                        }
                                    });
                                    EitherOf3::C(

                                        view! {
                                            <div class="flex flex-col gap-4 mx-auto w-full max-w-3xl">
                                                {move || match page_state() {
                                                    ChallengePageState::GameStarted => {
                                                        EitherOf3::A(view! { <GameStarted game_id=game_id() /> })
                                                    }
                                                    ChallengePageState::Removed => {
                                                        EitherOf3::B(
                                                            view! {
                                                                <Panel
                                                                    title=move || {
                                                                        t_string!(
                                                                            i18n, home.challenge.challenge_no_longer_available
                                                                        )
                                                                    }
                                                                    body_class="space-y-2"
                                                                >
                                                                    <p class="text-sm text-gray-600 dark:text-gray-300">
                                                                        {t!(i18n, home.challenge.challenge_no_longer_available)}
                                                                    </p>
                                                                </Panel>
                                                            },
                                                        )
                                                    }
                                                    ChallengePageState::Active => {
                                                        let title = summary.title;
                                                        let message = summary.message;
                                                        let displayed_user_label = summary.displayed_user_label;
                                                        let display_username = display_username.to_string();
                                                        let challenge = challenge.clone();
                                                        EitherOf3::C(

                                                            view! {
                                                                <Panel title=title body_class="space-y-3">
                                                                    <p class="text-sm text-gray-600 dark:text-gray-300">
                                                                        {message}
                                                                    </p>
                                                                    <div class="flex flex-wrap gap-2">
                                                                        <span class="inline-flex gap-1 items-center py-1 px-2 text-sm font-semibold text-gray-800 rounded border dark:text-gray-100 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-muted">
                                                                            {displayed_user_label}
                                                                            <span class="font-bold">{display_username}</span>
                                                                        </span>
                                                                    </div>
                                                                </Panel>
                                                                <Show when=show_copy_panel>
                                                                    <Panel
                                                                        title=move || {
                                                                            t_string!(i18n, home.challenge.share_private_link)
                                                                        }
                                                                        body_class="space-y-3"
                                                                    >
                                                                        <p class="text-sm text-gray-600 dark:text-gray-300">
                                                                            {t!(
                                                                                i18n, home.challenge.only_players_with_this_url_can_accept_this_private
                                                                            )}
                                                                        </p>
                                                                        <div class="flex gap-2 max-w-full">
                                                                            <input
                                                                                id="challenge_link"
                                                                                type="text"
                                                                                class="flex-1 ui-field-input"
                                                                                value=challenge_address
                                                                                readonly
                                                                            />
                                                                            <button
                                                                                title=move || t_string!(i18n, home.challenge.copy_link)
                                                                                on:click=copy
                                                                                class=copy_button_class
                                                                            >
                                                                                <Icon
                                                                                    icon=icondata_ai::AiCopyOutlined
                                                                                    attr:class="size-6"
                                                                                />
                                                                            </button>
                                                                        </div>
                                                                    </Panel>
                                                                </Show>
                                                                <Panel title=move || {
                                                                    t_string!(i18n, home.challenge.challenge_details)
                                                                }>
                                                                    <ChallengeDetails challenge=challenge />
                                                                </Panel>
                                                            },
                                                        )
                                                    }
                                                }}
                                            </div>
                                        },
                                    )
                                }
                            })
                    }}
                </ErrorBoundary>
            </Suspense>
        </PageShell>
    }
}
