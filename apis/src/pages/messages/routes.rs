use crate::{
    functions::chat::{
        get_game_chat_route_data,
        get_tournament_route_data,
        resolve_dm_route_user,
        DmRouteResponse,
        GameChatRouteResponse,
        TournamentRouteResponse,
    },
    i18n::*,
    providers::chat::{Chat, ChatSessionToken},
};
use leptos::{
    either::{Either, EitherOf4},
    prelude::*,
};
use leptos_router::{components::Redirect, hooks::use_params_map, NavigateOptions};
use shared_types::{GameId, GameThread, TournamentId};
use std::future::Future;
use uuid::Uuid;

use super::{
    message_game_href,
    thread::{
        MessagesResolvedDmView,
        MessagesResolvedGameView,
        MessagesResolvedTournamentView,
        MessagesStatusFrame,
    },
};

#[derive(Clone)]
enum RouteState<V> {
    Missing,
    AccessDenied,
    Failed,
    Ready(V),
}

#[derive(Clone, PartialEq)]
struct DmRouteData {
    other_user_id: Uuid,
    username: String,
    peer_deleted: bool,
}

#[derive(Clone)]
struct RouteResolution<K, V> {
    token: ChatSessionToken,
    key: Option<K>,
    state: RouteState<V>,
}

fn route_param(name: &'static str) -> Signal<Option<String>> {
    let params = use_params_map();
    Signal::derive(move || params.get().get(name))
}

fn resolve_route<K, V, Fetch, Fut, E>(
    chat: Chat,
    current_key: Signal<Option<K>>,
    fetch: Fetch,
) -> LocalResource<Option<RouteResolution<K, V>>>
where
    K: Clone + PartialEq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    Fetch: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<RouteState<V>, E>> + 'static,
    E: std::fmt::Display + 'static,
{
    LocalResource::new(move || {
        let route_request = chat
            .current_session_token()
            .map(|token| (token, current_key.get()));
        let fetch_future = match &route_request {
            Some((_, Some(key))) => Some(fetch(key.clone())),
            _ => None,
        };
        async move {
            let (request, current_key) = route_request?;
            let Some(key) = current_key else {
                return Some(RouteResolution {
                    token: request,
                    key: None,
                    state: RouteState::Missing,
                });
            };

            let fetch_future = fetch_future?;
            let state = match fetch_future.await {
                Ok(state) => state,
                Err(error) => {
                    log::error!("message route resolution failed: {error}");
                    RouteState::Failed
                }
            };
            Some(RouteResolution {
                token: request,
                key: Some(key),
                state,
            })
        }
    })
}

#[component]
fn ResolvedRouteShell<K, V, F, IV>(
    current_key: Signal<Option<K>>,
    resource: LocalResource<Option<RouteResolution<K, V>>>,
    loading_title: Signal<String>,
    loading_message: Signal<String>,
    missing_title: Signal<String>,
    missing_message: Signal<String>,
    access_denied_message: Signal<String>,
    render_ready: F,
) -> impl IntoView
where
    K: Clone + PartialEq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    F: Fn(K, V) -> IV + Send + Sync + 'static,
    IV: IntoView + 'static,
{
    let chat = expect_context::<Chat>();
    let render_ready = StoredValue::new(render_ready);
    move || {
        let Some(Some(RouteResolution {
            token: request,
            key: resolved_key,
            state,
        })) = resource.get()
        else {
            return EitherOf4::A(
                view! { <MessagesStatusFrame title=loading_title message=loading_message /> },
            );
        };
        if resolved_key != current_key.get() || !chat.is_current(request) {
            return EitherOf4::A(
                view! { <MessagesStatusFrame title=loading_title message=loading_message /> },
            );
        }
        match state {
            RouteState::Missing => EitherOf4::B(
                view! { <MessagesStatusFrame title=missing_title message=missing_message /> },
            ),
            RouteState::AccessDenied => EitherOf4::B(
                view! { <MessagesStatusFrame title=missing_title message=access_denied_message /> },
            ),
            RouteState::Failed => EitherOf4::C(view! {
                <MessagesStatusFrame
                    title=missing_title
                    message=missing_message
                    retry=Callback::new(move |_| {
                        if chat.identity_untracked().is_some() {
                            resource.refetch();
                        }
                    })
                />
            }),
            RouteState::Ready(value) => {
                let Some(key) = resolved_key else {
                    return EitherOf4::A(
                        view! { <MessagesStatusFrame title=loading_title message=loading_message /> },
                    );
                };
                EitherOf4::D(render_ready.with_value(|render_ready| render_ready(key, value)))
            }
        }
    }
}

#[component]
pub fn MessagesDmThread() -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_username = route_param("username");
    let loading_message =
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string());
    let failed_message =
        Signal::derive(move || t_string!(i18n, messages.page.failed_conversations).to_string());
    let resolved_resource = resolve_route(chat, route_username, |username| async move {
        resolve_dm_route_user(username)
            .await
            .map(|response| match response {
                DmRouteResponse::Ready {
                    other_user_id,
                    username,
                    peer_deleted,
                } => RouteState::Ready(DmRouteData {
                    other_user_id,
                    username,
                    peer_deleted,
                }),
                DmRouteResponse::NotFound => RouteState::Missing,
            })
    });

    view! {
        <ResolvedRouteShell
            current_key=route_username
            resource=resolved_resource
            loading_title=loading_message
            loading_message
            missing_title=failed_message
            missing_message=failed_message
            access_denied_message=failed_message
            render_ready=move |_, route_data| {
                view! {
                    <MessagesResolvedDmView
                        loading_message
                        failed_message
                        other_user_id=route_data.other_user_id
                        username=route_data.username
                        peer_deleted=route_data.peer_deleted
                    />
                }
            }
        />
    }
}

#[component]
pub fn MessagesTournamentThread() -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_nanoid = route_param("nanoid");
    let route_tournament_id = Signal::derive(move || route_nanoid.get().map(TournamentId));
    let title = Signal::derive(move || t_string!(i18n, messages.chat.tournament_title).to_string());
    let resolved_resource = resolve_route(
        chat,
        route_tournament_id,
        move |tournament_id: TournamentId| async move {
            get_tournament_route_data(tournament_id.0.clone())
                .await
                .map(|response| match response {
                    TournamentRouteResponse::Ready(name) => RouteState::Ready(name),
                    TournamentRouteResponse::NotFound => RouteState::Missing,
                    TournamentRouteResponse::AccessDenied => RouteState::AccessDenied,
                })
        },
    );

    view! {
        <ResolvedRouteShell
            current_key=route_tournament_id
            resource=resolved_resource
            loading_title=title
            loading_message=Signal::derive(move || {
                t_string!(i18n, messages.page.loading).to_string()
            })
            missing_title=title
            missing_message=Signal::derive(move || {
                t_string!(i18n, messages.page.failed_conversations).to_string()
            })
            access_denied_message=Signal::derive(move || {
                t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
            })
            render_ready=|tournament_id, name| {
                view! { <MessagesResolvedTournamentView tournament_id title=name /> }
            }
        />
    }
}

#[component]
pub fn MessagesGameThread(thread: GameThread) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_nanoid = route_param("nanoid");
    let title = Signal::derive(move || match thread {
        GameThread::Players => t_string!(i18n, messages.chat.players_chat).to_string(),
        GameThread::Spectators => t_string!(i18n, messages.chat.spectator_chat).to_string(),
    });
    let failed = Signal::derive(move || t_string!(i18n, messages.page.failed_game).to_string());
    let route_game_thread =
        Signal::derive(move || route_nanoid.get().map(|nanoid| (GameId(nanoid), thread)));
    let resolved_resource = resolve_route(chat, route_game_thread, |(game_id, _)| async move {
        get_game_chat_route_data(game_id)
            .await
            .map(|response| match response {
                GameChatRouteResponse::Ready(access) => RouteState::Ready(access),
                GameChatRouteResponse::NotFound => RouteState::Missing,
            })
    });
    view! {
        <ResolvedRouteShell
            current_key=route_game_thread
            resource=resolved_resource
            loading_title=title
            loading_message=Signal::derive(move || {
                t_string!(i18n, messages.page.loading).to_string()
            })
            missing_title=title
            missing_message=failed
            access_denied_message=failed
            render_ready=|(game_id, thread), access| {
                if thread == GameThread::Players && !access.can_read(GameThread::Players)
                    && access.can_read(GameThread::Spectators)
                {
                    Either::Left(
                        view! {
                            <Redirect
                                path=message_game_href(&game_id, GameThread::Spectators)
                                options=NavigateOptions {
                                    replace: true,
                                    ..Default::default()
                                }
                            />
                        },
                    )
                } else {
                    Either::Right(view! { <MessagesResolvedGameView game_id thread access /> })
                }
            }
        />
    }
}
