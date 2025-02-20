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
use leptix_primitives::{
    radio_group::{RadioGroupItem, RadioGroupRoot},
    toggle_group::{ToggleGroupItem, ToggleGroupKind, ToggleGroupMultiple, ToggleGroupRoot},
};
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::{params::Params, *};
use leptos_use::{
    core::ConnectionReadyState, signal_debounced, use_infinite_scroll_with_options,
    UseInfiniteScrollOptions,
};
use shared_types::{GameProgress, GameSpeed, GamesContextToUpdate, GamesQueryOptions, ResultType};
use std::str::FromStr;

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
    let toggle_classes = "flex hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-3 rounded bg-button-dawn dark:bg-button-twilight data-[state=on]:bg-pillbug-teal";
    let radio_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-2 rounded bg-button-dawn dark:bg-button-twilight data-[state=checked]:bg-pillbug-teal";
    let delay = 250.0;
    // WHY CAN'T COMPILER FIGURE OUT THE TYPES IN THE VIEW MACRO?
    let controls_string: Signal<String> = signal_debounced(
        Signal::derive(move || controls().tab_view.to_string()),
        delay,
    );
    let color_string: Signal<String> = signal_debounced(
        Signal::derive(move || {
            controls()
                .color
                .map_or("Both".to_string(), |c| c.to_string())
        }),
        delay,
    );
    let results_string: Signal<String> = signal_debounced(
        Signal::derive(move || {
            controls()
                .result
                .map_or("All".to_string(), |c| c.to_string())
        }),
        delay,
    );
    let speeds_strings: Signal<Vec<String>> = signal_debounced(
        Signal::derive(move || {
            controls()
                .speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        }),
        delay,
    );
    view! {
        <div class="flex flex-col m-1 text-md sm:text-lg">
            <RadioGroupRoot
                attr:class="flex gap-1"
                value=controls_string
            >

                <RadioGroupItem value="Unstarted" attr:class=radio_classes>
                    <a href="unstarted">{t!(i18n, profile.game_buttons.unstarted)}</a>
                </RadioGroupItem>
                <RadioGroupItem value="Playing" attr:class=radio_classes>
                    <a href="playing">{t!(i18n, profile.game_buttons.playing)}</a>
                </RadioGroupItem>
                <RadioGroupItem value="Finished" attr:class=radio_classes>
                    <a href="finished">{t!(i18n, profile.game_buttons.finished)}</a>
                </RadioGroupItem>
            </RadioGroupRoot>
            <div class="font-bold text-md">{t!(i18n, profile.player_color)}</div>
            <RadioGroupRoot
                attr:class="flex gap-1"
                value=color_string

                on_value_change=Callback::new(move |v: String| {
                    controls
                        .update(|c| {
                            c.color = Color::from_str(v.as_str()).ok();
                        });
                    first_batch(username(), controls());
                })
            >

                <RadioGroupItem value="b" attr:class=radio_classes>
                    {t!(i18n, profile.color_buttons.black)}
                </RadioGroupItem>
                <RadioGroupItem value="w" attr:class=radio_classes>
                    {t!(i18n, profile.color_buttons.white)}
                </RadioGroupItem>
                <RadioGroupItem value="Both" attr:class=radio_classes>
                    {t!(i18n, profile.color_buttons.both)}
                </RadioGroupItem>
            </RadioGroupRoot>
            <Show when=move || controls().tab_view == GameProgress::Finished>
                <div class="font-bold text-md">{t!(i18n, profile.game_result)}</div>
                <RadioGroupRoot
                    attr:class="flex gap-1"
                    value=results_string

                    on_value_change=Callback::new(move |v: String| {
                        controls
                            .update(|c| {
                                c.result = ResultType::from_str(v.as_str()).ok();
                            });
                        first_batch(username(), controls());
                    })
                >

                    <RadioGroupItem value="Win" attr:class=radio_classes>
                        {t!(i18n, profile.result_buttons.win)}
                    </RadioGroupItem>
                    <RadioGroupItem value="Loss" attr:class=radio_classes>
                        {t!(i18n, profile.result_buttons.loss)}
                    </RadioGroupItem>
                    <RadioGroupItem value="Draw" attr:class=radio_classes>
                        {t!(i18n, profile.result_buttons.draw)}
                    </RadioGroupItem>
                    <RadioGroupItem value="All" attr:class=radio_classes>
                        {t!(i18n, profile.result_buttons.all)}
                    </RadioGroupItem>
                </RadioGroupRoot>
            </Show>
            <div class="font-bold text-md">{t!(i18n, profile.include_speeds)}</div>
            <ToggleGroupRoot
                attr:class="flex gap-1 mb-1"
                kind=ToggleGroupKind::Multiple {
                    value: speeds_strings
                        .into(),
                    default_value: ToggleGroupMultiple::none().into(),
                    on_value_change: Some(
                        Callback::new(move |v: Vec<String>| {
                            controls
                                .update(|c| {
                                    c.speeds = v
                                        .iter()
                                        .map(|s| GameSpeed::from_str(s).unwrap())
                                        .collect();
                                });
                            first_batch(username(), controls());
                        }),
                    ),
                }
            >

                <ToggleGroupItem value="Bullet" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Bullet) />
                </ToggleGroupItem>
                <ToggleGroupItem value="Blitz" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Blitz) />
                </ToggleGroupItem>
                <ToggleGroupItem value="Rapid" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Rapid) />
                </ToggleGroupItem>
                <ToggleGroupItem value="Classic" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Classic) />
                </ToggleGroupItem>
                <ToggleGroupItem value="Correspondence" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Correspondence) />
                </ToggleGroupItem>
                <ToggleGroupItem value="Untimed" attr:class=toggle_classes>
                    <Icon icon=icon_for_speed(&GameSpeed::Untimed) />
                </ToggleGroupItem>
            </ToggleGroupRoot>
        </div>
    }
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    let params = use_params::<UsernameParams>();
    let ws = expect_context::<WebsocketContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
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
