use crate::{
    components::{molecules::game_row::GameRow, organisms::display_profile::DisplayProfile},
    providers::{
        games_search::{ProfileControls, ProfileGamesContext, ProfileGamesView},
        websocket::WebsocketContext,
        ApiRequests,
    },
};
use hive_lib::Color;
use leptix_primitives::{
    radio_group::{RadioGroupItem, RadioGroupRoot},
    toggle_group::{ToggleGroupItem, ToggleGroupKind, ToggleGroupMultiple, ToggleGroupRoot},
};
use leptos::*;
use leptos_router::*;
use leptos_use::{
    core::ConnectionReadyState, use_infinite_scroll_with_options, UseInfiniteScrollOptions,
};
use shared_types::{GameSpeed, GamesContextToUpdate, GamesQueryOptions, ResultType};
use std::str::FromStr;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

fn first_batch(user: String, c: ProfileControls) {
    let api = ApiRequests::new();

    api.games_search(GamesQueryOptions {
        players: vec![(user, c.color, c.result)],
        speeds: c.speeds,
        finished: Some(c.tab_view == ProfileGamesView::Finished),
        ctx_to_update: GamesContextToUpdate::Profile,
        unstarted: c.tab_view == ProfileGamesView::Unstarted,
        current_batch: None,
        batch_size: Some(3),
    });
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
    let ws = expect_context::<WebsocketContext>();
    let api = ApiRequests::new();
    let controls = ctx.controls;
    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            params.with(|p| {
                let _ = p
                    .as_ref()
                    .map(|p| p.username.clone())
                    .map(|u| api.user_profile(u));
            });
        }
    });
    let username = move || ctx.user.get().map_or(String::new(), |user| user.username);

    controls.update(|c| {
        c.speeds = vec![
            GameSpeed::Bullet,
            GameSpeed::Blitz,
            GameSpeed::Rapid,
            GameSpeed::Classic,
            GameSpeed::Correspondence,
            GameSpeed::Untimed,
        ];
    });
    let toggle_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded bg-button-dawn dark:bg-button-twilight data-[state=on]:bg-pillbug-teal";
    let radio_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded bg-button-dawn dark:bg-button-twilight data-[state=checked]:bg-pillbug-teal";
    view! {
        <div class="flex flex-col pt-12 bg-light dark:bg-gray-950">
            <Show when=move || ctx.user.get().is_some()>
                <div class="flex flex-col w-full">
                    <DisplayProfile user=ctx.user.get().unwrap()/>
                    <RadioGroupRoot
                        attr:class="flex gap-1 ml-3"
                        value=Signal::derive(move || { controls().tab_view.to_string() })
                    >

                        <RadioGroupItem value="Unstarted" attr:class=radio_classes>
                            <A href="unstarted">"Unstarted Tournament Games"</A>
                        </RadioGroupItem>
                        <RadioGroupItem value="Playing" attr:class=radio_classes>
                            <A href="playing">"Playing"</A>
                        </RadioGroupItem>
                        <RadioGroupItem value="Finished" attr:class=radio_classes>
                            <A href="finished">"Finished Games"</A>
                        </RadioGroupItem>
                    </RadioGroupRoot>
                    <span class="font-bold text-md">Player color:</span>
                    <RadioGroupRoot
                        attr:class="flex"
                        value=Signal::derive(move || {
                            controls().color.map_or("Both".to_string(), |c| c.to_string())
                        })

                        on_value_change=Callback::from(move |v: String| {
                            controls.update(|c| { c.color = Color::from_str(v.as_str()).ok() });
                            first_batch(username(), controls());
                        })
                    >

                        <RadioGroupItem value="b" attr:class=radio_classes>
                            "Black"
                        </RadioGroupItem>
                        <RadioGroupItem value="w" attr:class=radio_classes>
                            "White"
                        </RadioGroupItem>
                        <RadioGroupItem value="Both" attr:class=radio_classes>
                            "Both"
                        </RadioGroupItem>
                    </RadioGroupRoot>
                    <span class="font-bold text-md">Game Result:</span>
                    <RadioGroupRoot
                        attr:class="flex"
                        value=Signal::derive(move || {
                            controls().result.map_or("All".to_string(), |c| c.to_string())
                        })

                        on_value_change=Callback::from(move |v: String| {
                            controls
                                .update(|c| {
                                    c.result = ResultType::from_str(v.as_str()).ok();
                                });
                            first_batch(username(), controls());
                        })
                    >

                        <RadioGroupItem value="Win" attr:class=radio_classes>
                            "Win"
                        </RadioGroupItem>
                        <RadioGroupItem value="Loss" attr:class=radio_classes>
                            "Loss"
                        </RadioGroupItem>
                        <RadioGroupItem value="Draw" attr:class=radio_classes>
                            "Draw"
                        </RadioGroupItem>
                        <RadioGroupItem value="All" attr:class=radio_classes>
                            "All"
                        </RadioGroupItem>
                    </RadioGroupRoot>
                    <span class="font-bold text-md">Included speeds:</span>
                    <ToggleGroupRoot
                        attr:class="flex"
                        kind=ToggleGroupKind::Multiple {
                            value: Signal::derive(move || {
                                    controls()
                                        .speeds
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect::<Vec<_>>()
                                })
                                .into(),
                            default_value: ToggleGroupMultiple::none().into(),
                            on_value_change: Some(
                                Callback::from(move |v: Vec<String>| {
                                    controls
                                        .update(|c| {
                                            c
                                                .speeds = v
                                                .iter()
                                                .map(|s| GameSpeed::from_str(s).unwrap())
                                                .collect()
                                        });
                                    first_batch(username(), controls());
                                }),
                            ),
                        }
                    >

                        <ToggleGroupItem value="Bullet" attr:class=toggle_classes>
                            "Bullet"
                        </ToggleGroupItem>
                        <ToggleGroupItem value="Blitz" attr:class=toggle_classes>
                            "Blitz"
                        </ToggleGroupItem>
                        <ToggleGroupItem value="Rapid" attr:class=toggle_classes>
                            "Rapid"
                        </ToggleGroupItem>
                        <ToggleGroupItem value="Classic" attr:class=toggle_classes>
                            "Classic"
                        </ToggleGroupItem>
                        <ToggleGroupItem value="Correspondence" attr:class=toggle_classes>
                            "Correspondence"
                        </ToggleGroupItem>
                        <ToggleGroupItem value="Untimed" attr:class=toggle_classes>
                            "Untimed"
                        </ToggleGroupItem>
                    </ToggleGroupRoot>
                    {children()}
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: ProfileGamesView) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let username = move || ctx.user.get().map_or(String::new(), |user| user.username);
    let user_info = create_read_slice(ctx.controls, move |c| (username(), c.color, c.result));
    let el = create_node_ref::<html::Div>();
    let ws = expect_context::<WebsocketContext>();

    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            ctx.controls.update(|c| {
                c.tab_view = tab_view;
            });
            first_batch(username(), ctx.controls.get_untracked());
        }
    });
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            let api = ApiRequests::new();

            if ctx.more_games.get() {
                api.games_search(GamesQueryOptions {
                    players: vec![user_info()],
                    speeds: ctx.controls.get().speeds,
                    finished: Some(tab_view == ProfileGamesView::Finished),
                    current_batch: ctx.batch_info.get(),
                    batch_size: Some(5),
                    ctx_to_update: GamesContextToUpdate::Profile,
                    unstarted: tab_view == ProfileGamesView::Unstarted,
                });
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(150.0),
    );
    view! {
        <div node_ref=el class="flex flex-col overflow-x-hidden items-center h-[51vh]">
            <For each=ctx.games key=move |game| (game.uuid, tab_view) let:game>
                <GameRow game=store_value(game)/>
            </For>
        </div>
    }
}
