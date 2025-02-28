use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::components::molecules::user_row::UserRow;
use crate::i18n::*;
use crate::providers::ApiRequestsProvider;
use crate::{
    components::{molecules::game_row::GameRow, organisms::display_profile::DisplayProfile},
    providers::{
        games_search::{ProfileControls, ProfileGamesContext},
        navigation_controller::{NavigationControllerSignal, ProfileNavigationControllerState},
        websocket::WebsocketContext,
    },
};
use hive_lib::Color;
use hooks::use_params;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::{params::Params, *};
use leptos_use::{
    core::ConnectionReadyState, use_infinite_scroll_with_options,
    UseInfiniteScrollOptions,
};
use shared_types::{GameProgress, GameSpeed, GamesContextToUpdate, GamesQueryOptions, ResultType};

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

fn first_batch(user: String, c: ProfileControls) {
    let api = expect_context::<ApiRequestsProvider>().0.get();

    api.games_search(GamesQueryOptions {
        players: vec![(user.clone(), c.color, c.result)],
        speeds: c.speeds,
        ctx_to_update: GamesContextToUpdate::Profile(user),
        current_batch: None,
        batch_size: Some(6),
        game_progress: c.tab_view,
    });
}

#[component]
fn Controls(username: Signal<String>) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let controls = ctx.controls;
    let i18n = use_i18n();
    let toggle_classes = |active| format!("flex hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-3 rounded bg-button-dawn dark:bg-button-twilight {}", if active { "bg-pillbug-teal" } else { "" });
    let radio_classes = |active| format!("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-2 rounded bg-button-dawn dark:bg-button-twilight {}", if active { "bg-pillbug-teal" } else { "" });
    let delay = 250.0;
    let toggle_speeds = move |speed| {
        controls.update(|c| {
            if c.speeds.contains(&speed) {
                c.speeds.retain(|s| s != &speed);
            } else {
                c.speeds.push(speed);
            }
        }); 
        first_batch(username(), controls());
    };
    view! {
        <div class="flex flex-col m-1 text-md sm:text-lg">
            <div
                class="flex gap-1"
            >
                <a href=format!("/@/{}/unstarted",username()) class=move || radio_classes(controls().tab_view == GameProgress::Unstarted) >{t!(i18n, profile.game_buttons.unstarted)}</a>
                <a href=format!("/@/{}/playing",username()) class=move || radio_classes(controls().tab_view == GameProgress::Playing)>{t!(i18n, profile.game_buttons.playing)}</a>
                <a href=format!("/@/{}/finished",username()) class=move || radio_classes(controls().tab_view == GameProgress::Finished)>{t!(i18n, profile.game_buttons.finished)}</a>

            </div>
            <div class="font-bold text-md">{t!(i18n, profile.player_color)}</div>
            <div class="flex gap-1">
                <button
                on:click=move |_| {controls.update(|c| c.color = Some(Color::Black)); first_batch(username(), controls());}
                class=move || radio_classes(controls().color == Some(Color::Black))>
                    {t!(i18n, profile.color_buttons.black)}
                </button>
                <button
                on:click=move |_| {controls.update(|c| c.color = Some(Color::White)); first_batch(username(), controls());}
                class=move || radio_classes(controls().color == Some(Color::White))>
                    {t!(i18n, profile.color_buttons.white)}
                </button>
                <button
                on:click=move |_| {controls.update(|c| c.color = None); first_batch(username(), controls());}
                class=move || radio_classes(controls().color.is_none())>
                    {t!(i18n, profile.color_buttons.both)}
                </button>
            </div>
            <Show when=move || controls().tab_view == GameProgress::Finished>
                <div class="font-bold text-md">{t!(i18n, profile.game_result)}</div>
                <div class="flex gap-1">
                    <button class=move || radio_classes(controls().result == Some(ResultType::Win))
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Win)); first_batch(username(), controls());}>
                        {t!(i18n, profile.result_buttons.win)}
                    </button>
                    <button 
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Loss)); first_batch(username(), controls());}
                    class=move || radio_classes(controls().result == Some(ResultType::Loss))>
                        {t!(i18n, profile.result_buttons.loss)}
                    </button>
                    <button
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Draw)); first_batch(username(), controls());}
                     class=move || radio_classes(controls().result == Some(ResultType::Draw))>
                        {t!(i18n, profile.result_buttons.draw)}
                    </button>
                    <button
                    on:click=move |_| {controls.update(|c| c.result = None); first_batch(username(), controls());}
                     class=move || radio_classes(controls().result.is_none())>
                        {t!(i18n, profile.result_buttons.all)}
                    </button>
                </div>
            </Show>
            <div class="font-bold text-md">{t!(i18n, profile.include_speeds)}</div>
            <div class="flex gap-1 mb-1">
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Bullet); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Bullet))>
                    <Icon icon=icon_for_speed(&GameSpeed::Bullet) />
                </button>
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Blitz); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Blitz))>
                    <Icon icon=icon_for_speed(&GameSpeed::Blitz) />
                </button>
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Rapid); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Rapid))>
                    <Icon icon=icon_for_speed(&GameSpeed::Rapid) />
                </button>
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Classic); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Classic))>
                    <Icon icon=icon_for_speed(&GameSpeed::Classic) />
                </button>
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Correspondence); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Correspondence))>
                    <Icon icon=icon_for_speed(&GameSpeed::Correspondence) />
                </button>
                <button
                on:click=move |_| { toggle_speeds(GameSpeed::Untimed); }
                class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Untimed))>
                    <Icon icon=icon_for_speed(&GameSpeed::Untimed) />
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    let ws = expect_context::<WebsocketContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let params = use_params::<UsernameParams>();
    let username = Signal::derive(move || {
        params.with(|p| {
            p.as_ref()
                .ok()
                .map(|p| p.username.clone())
                .map_or(String::new(), |user| user)
        })
    });
    Effect::new(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = api.get();
            navi.profile_signal.update(|v| {
                *v = ProfileNavigationControllerState {
                    username: Some(username()),
                }
            });
            ctx.controls.update(|c| {
                c.speeds = vec![
                    GameSpeed::Bullet,
                    GameSpeed::Blitz,
                    GameSpeed::Rapid,
                    GameSpeed::Classic,
                    GameSpeed::Correspondence,
                    GameSpeed::Untimed,
                ];
            });
            api.user_profile(username.get_untracked());
        }
    });
    view! {
        <div class="flex flex-col pt-12 mx-3 bg-light dark:bg-gray-950">
            <Show when=move || ctx.user.get().is_some()>
                <div class="flex justify-center w-full text-lg sm:text-xl">
                    <UserRow
                        actions=vec![UserAction::Challenge]
                        user=StoredValue::new(ctx.user.get().unwrap())
                        on_profile=true
                    />
                </div>
                <div class="flex flex-col-reverse m-1 w-full sm:flex-row">
                    <Controls username />
                    <div class="text-md sm:w-2/3 sm:text-lg">
                        <DisplayProfile user=ctx.user.get().unwrap() />
                    </div>
                </div>
                {children()}
            </Show>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: GameProgress) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let username = Signal::derive(move || {
        params.with(|p| {
            p.as_ref()
                .ok()
                .map(|p| p.username.clone())
                .map_or(String::new(), |user| user)
        })
    });
    let user_info = create_read_slice(ctx.controls, move |c| (username(), c.color, c.result));
    let el = NodeRef::<html::Div>::new();
    el.on_load(move |_| {
        ctx.controls.update(|c| {
            c.tab_view = tab_view;
            if tab_view != GameProgress::Finished {
                c.result = None;
            };
        });
        first_batch(username(), ctx.controls.get_untracked());
    });

    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            let api = api.get();

            if ctx.more_games.get() {
                let controls = ctx.controls.get();
                api.games_search(GamesQueryOptions {
                    players: vec![user_info()],
                    speeds: controls.speeds,
                    current_batch: ctx.batch_info.get(),
                    batch_size: Some(4),
                    ctx_to_update: GamesContextToUpdate::Profile(username()),
                    game_progress: controls.tab_view,
                });
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(300.0),
    );
    view! {
        <div
            node_ref=el
            class="overflow-x-hidden items-center sm:grid sm:grid-cols-2 sm:gap-1 h-[53vh] sm:h-[66vh]"
        >
            <For each=ctx.games key=move |game| (game.uuid, tab_view) let:game>
                <GameRow game=StoredValue::new(game) />
            </For>
        </div>
    }
}
