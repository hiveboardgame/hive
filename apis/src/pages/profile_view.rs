use crate::{
    components::{molecules::game_row::GameRow, organisms::display_profile::DisplayProfile},
    providers::{
        games_search::{ProfileControls, ProfileGamesContext},
        navigation_controller::{NavigationControllerSignal, ProfileNavigationControllerState},
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
use leptos_dom::helpers::TimeoutHandle;
use leptos_router::*;
use leptos_use::{
    core::ConnectionReadyState, use_infinite_scroll_with_options, UseInfiniteScrollOptions,
};
use shared_types::{GameProgress, GameSpeed, GamesContextToUpdate, GamesQueryOptions, ResultType};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    str::FromStr,
    time::Duration,
};

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

fn first_batch(user: String, c: ProfileControls) {
    let api = ApiRequests::new();

    api.games_search(GamesQueryOptions {
        players: vec![(user.clone(), c.color, c.result)],
        speeds: c.speeds,
        ctx_to_update: GamesContextToUpdate::Profile(user),
        current_batch: None,
        batch_size: Some(3),
        game_progress: c.tab_view,
    });
}

// Based on leptos_dom::helpers::debounce https://docs.rs/leptos_dom/latest/src/leptos_dom/helpers.rs.html#288-339
// which returns an FnMut that is not compatible with the Callback that leptix_primitives RadioGroupRoot expects
fn debounce_cb<T: 'static + Clone>(
    delay: Duration,
    cb: impl Fn(T) + 'static + Clone,
) -> Callback<T> {
    let cb = Rc::new(RefCell::new(cb));

    let timer = Rc::new(Cell::new(None::<TimeoutHandle>));

    on_cleanup({
        let timer = Rc::clone(&timer);
        move || {
            if let Some(timer) = timer.take() {
                timer.clear();
            }
        }
    });

    Callback::new(move |arg: T| {
        if let Some(timer) = timer.take() {
            timer.clear();
        }
        let handle = set_timeout_with_handle(
            {
                let cb = Rc::clone(&cb);
                let arg = arg.clone();
                move || {
                    cb.borrow()(arg);
                }
            },
            delay,
        );
        if let Ok(handle) = handle {
            timer.set(Some(handle));
        }
    })
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    let params = use_params::<UsernameParams>();
    let ws = expect_context::<WebsocketContext>();
    let controls = ctx.controls;
    let username = Signal::derive(move || {
        params.with(|p| {
            p.as_ref()
                .ok()
                .map(|p| p.username.clone())
                .map_or(String::new(), |user| user)
        })
    });
    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = ApiRequests::new();
            batch(move || {
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
            });
            api.user_profile(username.get_untracked());
        }
    });
    let delay = Duration::from_millis(600);
    let debouced_first_batch = debounce_cb(delay, move |()| {
        first_batch(username(), controls());
    });
    let toggle_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded bg-button-dawn dark:bg-button-twilight data-[state=on]:bg-pillbug-teal";
    let radio_classes = "hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded bg-button-dawn dark:bg-button-twilight data-[state=checked]:bg-pillbug-teal";
    view! {
        <div class="flex flex-col pt-12 mx-3 bg-light dark:bg-gray-950">
            <Show when=move || ctx.user.get().is_some()>
                <div class="flex flex-col m-1 w-full">
                    <DisplayProfile user=ctx.user.get().unwrap()/>
                    <div class="flex flex-col m-1 w-full">
                        <RadioGroupRoot
                            attr:class="flex flex-wrap gap-1"
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
                            attr:class="flex flex-wrap gap-1"
                            value=Signal::derive(move || {
                                controls().color.map_or("Both".to_string(), |c| c.to_string())
                            })

                            on_value_change=Callback::new(move |v: String| {
                                controls
                                    .update(|c| {
                                        c.color = Color::from_str(v.as_str()).ok();
                                    });
                                debouced_first_batch(());
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
                        <Show when=move || controls().tab_view == GameProgress::Finished>
                            <span class="font-bold text-md">Game Result:</span>
                            <RadioGroupRoot
                                attr:class="flex flex-wrap gap-1"
                                value=Signal::derive(move || {
                                    controls().result.map_or("All".to_string(), |c| c.to_string())
                                })

                                on_value_change=Callback::new(move |v: String| {
                                    controls
                                        .update(|c| {
                                            c.result = ResultType::from_str(v.as_str()).ok();
                                        });
                                    debouced_first_batch(());
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
                        </Show>
                        <span class="font-bold text-md">Included speeds:</span>
                        <ToggleGroupRoot
                            attr:class="flex flex-wrap gap-1 mb-1"
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
                                    Callback::new(move |v: Vec<String>| {
                                        controls
                                            .update(|c| {
                                                c
                                                    .speeds = v
                                                    .iter()
                                                    .map(|s| GameSpeed::from_str(s).unwrap())
                                                    .collect();
                                            });
                                        debouced_first_batch(());
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
                    </div>
                    {children()}
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: GameProgress) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
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
            let api = ApiRequests::new();

            if ctx.more_games.get() {
                let controls = ctx.controls.get();
                api.games_search(GamesQueryOptions {
                    players: vec![user_info()],
                    speeds: controls.speeds,
                    current_batch: ctx.batch_info.get(),
                    batch_size: Some(5),
                    ctx_to_update: GamesContextToUpdate::Profile(username()),
                    game_progress: controls.tab_view,
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
