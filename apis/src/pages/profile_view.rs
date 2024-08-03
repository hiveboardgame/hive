use crate::components::molecules::game_row::GameRow;
use crate::components::organisms::display_profile::DisplayProfile;
use crate::providers::websocket::WebsocketContext;
use crate::providers::ApiRequests;
use crate::responses::{GameResponse, UserResponse};
use chrono::{DateTime, Utc};
use leptos::*;
use leptos_router::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions};
use shared_types::{GamesContextToUpdate, GamesQueryOptions};
use uuid::Uuid;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum ProfileGamesView {
    Unstarted,
    Playing,
    Finished,
}

#[derive(Debug, Clone)]
pub struct ProfileGamesContext {
    pub unstarted: RwSignal<Vec<GameResponse>>,
    pub playing: RwSignal<Vec<GameResponse>>,
    pub finished: RwSignal<Vec<GameResponse>>,
    pub more_finished: RwSignal<bool>,
    pub finished_last_timestamp: RwSignal<Option<DateTime<Utc>>>,
    pub finished_last_id: RwSignal<Option<Uuid>>,
    pub user: RwSignal<Option<UserResponse>>,
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let ws = expect_context::<WebsocketContext>();
    let ctx = expect_context::<ProfileGamesContext>();
    let username = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.username.clone())
                .unwrap_or_default()
        })
    };
    let api = ApiRequests::new();
    create_effect(move |_| {
        //rerun only if empty or new username
        if ws.ready_state.get() == ConnectionReadyState::Open
            && ctx
                .user
                .get()
                .map_or(true, |user| user.username != username())
        {
            ctx.unstarted.set(Vec::new());
            ctx.playing.set(Vec::new());
            ctx.finished.set(Vec::new());
            ctx.more_finished.set(false);
            ctx.finished_last_timestamp.set(None);
            ctx.finished_last_id.set(None);
            ctx.user.set(None);
            api.user_profile(username());
            api.games_search(GamesQueryOptions {
                username: Some(username()),
                last_id: None,
                last_timestamp: None,
                batch_size: Some(5),
                is_finished: Some(true),
                ctx_to_update: GamesContextToUpdate::ProfileFinished,
            });
            api.games_search(GamesQueryOptions {
                username: Some(username()),
                last_id: None,
                last_timestamp: None,
                batch_size: None,
                is_finished: Some(false),
                ctx_to_update: GamesContextToUpdate::ProfilePlaying,
            });
        }
    });
    view! {
        <div class="flex flex-col pt-12 bg-light dark:bg-gray-950">
            <Show when=move || ctx.user.get().is_some()>
                <div class="flex flex-col w-full">
                    <DisplayProfile user=ctx.user.get().unwrap()/>
                    {children()}
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: ProfileGamesView) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let username = move || {
        ctx.user.get().map_or(String::new(), |user| user.username)
    };
    let el = create_node_ref::<html::Div>();
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            let api = ApiRequests::new();
            if tab_view == ProfileGamesView::Finished && ctx.more_finished.get() {
                api.games_search(GamesQueryOptions {
                    username: Some(username()),
                    last_id: ctx.finished_last_id.get(),
                    last_timestamp: ctx.finished_last_timestamp.get(),
                    batch_size: Some(5),
                    is_finished: Some(true),
                    ctx_to_update: GamesContextToUpdate::ProfileFinished,
                });
            }
        },
        UseInfiniteScrollOptions::default().distance(10.0),
    );
    let active = move |view: ProfileGamesView| {
        let button_style = String::from("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded");
        if tab_view == view {
            button_style + " bg-pillbug-teal"
        } else {
            button_style + " bg-button-dawn dark:bg-button-twilight"
        }
    };

    let games = match tab_view {
        ProfileGamesView::Finished => ctx.finished,
        ProfileGamesView::Playing => ctx.playing,
        ProfileGamesView::Unstarted => ctx.unstarted,
    };
    view! {
        <div class="flex gap-1 ml-3">
        <Show when=move || !ctx.unstarted.get().is_empty()>
            <A
                href=format!("/@/{}/unstarted", username())
                class=move || active(ProfileGamesView::Unstarted)
            >
                "Unstarted Tournament Games"
            </A>
        </Show>
        <Show when=move || !ctx.playing.get().is_empty()>
            <A
                href=format!("/@/{}/playing", username())
                class=move || active(ProfileGamesView::Playing)
            >
                "Playing "
            </A>
        </Show>
        <Show when=move || !ctx.finished.get().is_empty()>
            <A
                href=format!("/@/{}/finished", username())
                class=move || active(ProfileGamesView::Finished)
            >
                "Finished Games "
            </A>
        </Show>
    </div>
    <div node_ref=el class="flex flex-col overflow-x-hidden items-center h-[72vh]">
        <For each=games key=|game| (game.uuid) let:game>
            <GameRow game=store_value(game)/>
        </For>
    </div>
    }
}
