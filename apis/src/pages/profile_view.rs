use std::str::FromStr;

use crate::components::molecules::game_row::GameRow;
use crate::components::organisms::display_profile::DisplayProfile;
use crate::providers::websocket::WebsocketContext;
use crate::providers::ApiRequests;
use crate::responses::{GameResponse, UserResponse};
use leptix_primitives::toggle_group::{
    ToggleGroupItem, ToggleGroupKind, ToggleGroupMultiple, ToggleGroupRoot,
};
use leptos::*;
use leptos_router::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions};
use shared_types::{BatchInfo, GameSpeed, GamesContextToUpdate, GamesQueryOptions};

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
    pub finished_batch: RwSignal<Option<BatchInfo>>,
    pub user: RwSignal<Option<UserResponse>>,
    pub speeds: RwSignal<Vec<GameSpeed>>,
}

fn first_batch(username: String) {
    let ctx: ProfileGamesContext = expect_context::<ProfileGamesContext>();
    let api = ApiRequests::new();
    ctx.unstarted.set(Vec::new());
    ctx.playing.set(Vec::new());
    ctx.finished.set(Vec::new());
    ctx.more_finished.set(false);
    ctx.finished_batch.set(None);
    ctx.user.set(None);
    api.user_profile(username.clone());
    api.games_search(GamesQueryOptions {
        usernames: vec![username.clone()],
        speeds: ctx.speeds.get(),
        current_batch: None,
        batch_size: Some(5),
        is_finished: Some(true),
        ctx_to_update: GamesContextToUpdate::ProfileFinished,
    });
    api.games_search(GamesQueryOptions {
        usernames: vec![username.clone()],
        speeds: ctx.speeds.get(),
        current_batch: None,
        batch_size: None,
        is_finished: Some(false),
        ctx_to_update: GamesContextToUpdate::ProfilePlaying,
    });
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
    let ws = expect_context::<WebsocketContext>();
    let username = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.username.clone())
                .unwrap_or_default()
        })
    };
    ctx.speeds.set(vec![
        GameSpeed::Bullet,
        GameSpeed::Blitz,
        GameSpeed::Rapid,
        GameSpeed::Classic,
        GameSpeed::Correspondence,
        GameSpeed::Untimed,
    ]);
    create_effect(move |_| {
        //rerun only if empty or new username
        if ws.ready_state.get() == ConnectionReadyState::Open
            && ctx
                .user
                .get()
                .map_or(true, |user| user.username != username())
        {
            first_batch(username());
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
    let username = store_value(ctx.user.get().map_or(String::new(), |user| user.username));
    let el = create_node_ref::<html::Div>();
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            let api = ApiRequests::new();
            if tab_view == ProfileGamesView::Finished && ctx.more_finished.get() {
                api.games_search(GamesQueryOptions {
                    usernames: vec![username()],
                    speeds: ctx.speeds.get(),
                    is_finished: Some(true),
                    current_batch: ctx.finished_batch.get(),
                    batch_size: Some(5),
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
    let toggle_group_item_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded bg-button-dawn dark:bg-button-twilight data-[state=on]:bg-pillbug-teal";
    view! {
        <div class="flex gap-1 ml-3">
            <div class="flex">
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
        </div>
        <ToggleGroupRoot
            attr:class="flex"
            kind=ToggleGroupKind::Multiple {
                value: ctx.speeds.get().iter().map(|s| s.to_string()).collect::<Vec<_>>().into(),
                default_value: ToggleGroupMultiple::none().into(),
                on_value_change: Some(
                    Callback::from(move |value: Vec<String>| {
                        ctx.speeds
                            .set(
                                value
                                    .into_iter()
                                    .map(|s| GameSpeed::from_str(&s).unwrap())
                                    .collect(),
                            );
                        first_batch(username());
                    }),
                ),
            }
        >

            <ToggleGroupItem value="Bullet" attr:class=toggle_group_item_classes>
                <button>"Bullet"</button>
            </ToggleGroupItem>
            <ToggleGroupItem value="Blitz" attr:class=toggle_group_item_classes>
                <button>"Blitz"</button>
            </ToggleGroupItem>
            <ToggleGroupItem value="Rapid" attr:class=toggle_group_item_classes>
                <button>"Rapid"</button>
            </ToggleGroupItem>
            <ToggleGroupItem value="Classic" attr:class=toggle_group_item_classes>
                <button>"Classic"</button>
            </ToggleGroupItem>
            <ToggleGroupItem value="Correspondence" attr:class=toggle_group_item_classes>
                <button>"Correspondence"</button>
            </ToggleGroupItem>
            <ToggleGroupItem value="Untimed" attr:class=toggle_group_item_classes>
                <button>"Untimed"</button>
            </ToggleGroupItem>
        </ToggleGroupRoot>
        <div node_ref=el class="flex flex-col overflow-x-hidden items-center h-[72vh]">
            <For each=games key=|game| (game.uuid) let:game>
                <GameRow game=store_value(game)/>
            </For>
        </div>
    }
}
