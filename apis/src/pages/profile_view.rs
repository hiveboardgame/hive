use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::components::molecules::user_row::UserRow;
use crate::components::{molecules::game_row::GameRow, organisms::display_profile::DisplayProfile};
use crate::functions::games::get::GetBatchFromOptions;
use crate::functions::users::get_profile;
use crate::i18n::*;
use crate::responses::GameResponse;
use hive_lib::Color;
use hooks::use_params;
use leptos::either::Either;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::{params::Params, *};
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions};
use shared_types::{BatchInfo, GameProgress, GameSpeed, GamesQueryOptions, ResultType};

#[derive(Debug, Clone, Default)]
struct ProfileControls {
    pub color: Option<Color>,
    pub result: Option<ResultType>,
    pub speeds: Vec<GameSpeed>,
    pub tab_view: GameProgress,
}

#[derive(Clone)]
struct ProfileGamesContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub controls: RwSignal<ProfileControls>,
    pub next_batch: ServerAction<GetBatchFromOptions>,
    pub is_first_batch: StoredValue<bool>,
    pub has_more: StoredValue<bool>,
}
#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

fn load_games(
    controls: ProfileControls,
    username: String,
    batch_info: Option<BatchInfo>,
    action: ServerAction<GetBatchFromOptions>,
) {
    let user_info = (username, controls.color, controls.result);
    let batch_size = if batch_info.is_none() {
        Some(6)
    } else {
        Some(4)
    };
    let options = GamesQueryOptions {
        players: vec![user_info],
        speeds: controls.speeds,
        current_batch: batch_info,
        batch_size,
        game_progress: controls.tab_view,
    };
    action.dispatch(GetBatchFromOptions { options });
}

#[component]
fn Controls(username: String, ctx: ProfileGamesContext) -> impl IntoView {
    let username = Signal::derive(move || username.clone());
    let controls = ctx.controls;
    let i18n = use_i18n();
    let toggle_classes = |active| {
        format!("flex hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-3 rounded {}", if active { "bg-pillbug-teal" } else { "bg-button-dawn dark:bg-button-twilight" })
    };
    let radio_classes = |active| {
        format!("no-link-style hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-2 rounded {}", if active { "bg-pillbug-teal" } else { "bg-button-dawn dark:bg-button-twilight" })
    };
    let set_first_batch = move || {
        ctx.has_more.set_value(true);
        ctx.is_first_batch.set_value(true);
        load_games(ctx.controls.get(), username(), None, ctx.next_batch);
    };
    let toggle_speeds = move |speed| {
        controls.update(|c| {
            if c.speeds.contains(&speed) {
                c.speeds.retain(|s| s != &speed);
            } else {
                c.speeds.push(speed);
            }
        });
        set_first_batch();
    };
    view! {
        <div class="flex flex-col m-1 text-md sm:text-lg">
            <div class="flex flex-wrap gap-1">
                <a
                    href=format!("/@/{}/unstarted", username())
                    class=move || radio_classes(controls().tab_view == GameProgress::Unstarted)
                >
                    {t!(i18n, profile.game_buttons.unstarted)}
                </a>
                <a
                    href=format!("/@/{}/playing", username())
                    class=move || radio_classes(controls().tab_view == GameProgress::Playing)
                >
                    {t!(i18n, profile.game_buttons.playing)}
                </a>
                <a
                    href=format!("/@/{}/finished", username())
                    class=move || radio_classes(controls().tab_view == GameProgress::Finished)
                >
                    {t!(i18n, profile.game_buttons.finished)}
                </a>

            </div>
            <div class="font-bold text-md">{t!(i18n, profile.player_color)}</div>
            <div class="flex flex-wrap gap-1">
                <button
                    on:click=move |_| {
                        controls.update(|c| c.color = Some(Color::Black));
                        set_first_batch();
                    }
                    class=move || radio_classes(controls().color == Some(Color::Black))
                >
                    {t!(i18n, profile.color_buttons.black)}
                </button>
                <button
                    on:click=move |_| {
                        controls.update(|c| c.color = Some(Color::White));
                        set_first_batch();
                    }
                    class=move || radio_classes(controls().color == Some(Color::White))
                >
                    {t!(i18n, profile.color_buttons.white)}
                </button>
                <button
                    on:click=move |_| {
                        controls.update(|c| c.color = None);
                        set_first_batch();
                    }
                    class=move || radio_classes(controls().color.is_none())
                >
                    {t!(i18n, profile.color_buttons.both)}
                </button>
            </div>
            <Show when=move || controls().tab_view == GameProgress::Finished>
                <div class="font-bold text-md">{t!(i18n, profile.game_result)}</div>
                <div class="flex flex-wrap gap-1">
                    <button
                        class=move || radio_classes(controls().result == Some(ResultType::Win))
                        on:click=move |_| {
                            controls.update(|c| c.result = Some(ResultType::Win));
                            set_first_batch();
                        }
                    >
                        {t!(i18n, profile.result_buttons.win)}
                    </button>
                    <button
                        on:click=move |_| {
                            controls.update(|c| c.result = Some(ResultType::Loss));
                            set_first_batch();
                        }
                        class=move || radio_classes(controls().result == Some(ResultType::Loss))
                    >
                        {t!(i18n, profile.result_buttons.loss)}
                    </button>
                    <button
                        on:click=move |_| {
                            controls.update(|c| c.result = Some(ResultType::Draw));
                            set_first_batch();
                        }
                        class=move || radio_classes(controls().result == Some(ResultType::Draw))
                    >
                        {t!(i18n, profile.result_buttons.draw)}
                    </button>
                    <button
                        on:click=move |_| {
                            controls.update(|c| c.result = None);
                            set_first_batch();
                        }
                        class=move || radio_classes(controls().result.is_none())
                    >
                        {t!(i18n, profile.result_buttons.all)}
                    </button>
                </div>
            </Show>
            <div class="font-bold text-md">{t!(i18n, profile.include_speeds)}</div>
            <div class="flex flex-wrap gap-1 mb-1">
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Bullet);
                    }
                    class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Bullet))
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Bullet) />
                </button>
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Blitz);
                    }
                    class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Blitz))
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Blitz) />
                </button>
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Rapid);
                    }
                    class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Rapid))
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Rapid) />
                </button>
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Classic);
                    }
                    class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Classic))
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Classic) />
                </button>
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Correspondence);
                    }
                    class=move || toggle_classes(
                        controls().speeds.contains(&GameSpeed::Correspondence),
                    )
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Correspondence) />
                </button>
                <button
                    on:click=move |_| {
                        toggle_speeds(GameSpeed::Untimed);
                    }
                    class=move || toggle_classes(controls().speeds.contains(&GameSpeed::Untimed))
                >
                    <Icon icon=icon_for_speed(&GameSpeed::Untimed) />
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let username =
        move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let user = LocalResource::new(move || get_profile(username()));
    provide_context(ProfileGamesContext {
        controls: RwSignal::new(ProfileControls {
            speeds: GameSpeed::all_games(),
            ..Default::default()
        }),
        games: RwSignal::new(Vec::new()),
        has_more: StoredValue::new(true),
        next_batch: ServerAction::new(),
        is_first_batch: StoredValue::new(true),
    });
    let ctx = expect_context::<ProfileGamesContext>();
    Effect::watch(
        ctx.next_batch.version(),
        move |_, _, _| {
            let next_batch = if let Some(Ok(next_batch)) = ctx.next_batch.value().get_untracked() {
                next_batch
            } else {
                vec![]
            };
            if next_batch.is_empty() {
                ctx.has_more.set_value(false);
            }
            ctx.games.update(|games| {
                if ctx.is_first_batch.get_value() {
                    *games = next_batch;
                } else {
                    games.extend(next_batch);
                }
            });
        },
        true,
    );

    view! {
        <div class="flex flex-col pt-12 mx-3 bg-light dark:bg-gray-950 h-[100vh]">
            <Transition fallback=move || {
                view! { <p>"Loading Profile..."</p> }
            }>
                {move || {
                    user.get()
                        .as_ref()
                        .map(|user| {
                            if let Ok(user) = user {
                                Either::Left(
                                    view! {
                                        <div class="flex justify-center w-full text-lg sm:text-xl">
                                            <UserRow
                                                actions=vec![UserAction::Challenge]
                                                user=user.clone()
                                                on_profile=true
                                            />
                                        </div>
                                        <div class="flex flex-col-reverse m-1 w-full sm:flex-row">
                                            <Controls username=user.username.clone() ctx=ctx.clone() />
                                            <div class="text-md sm:w-2/3 sm:text-lg">
                                                <DisplayProfile user=user.clone() />
                                            </div>
                                        </div>
                                        {children()}
                                    },
                                )
                            } else {
                                Either::Right(view! { <p>"User not found"</p> })
                            }
                        })
                }}
            </Transition>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: GameProgress) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
    let username =
        move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let el = NodeRef::<html::Div>::new();
    el.on_load(move |_| {
        ctx.controls.update(|c| {
            c.tab_view = tab_view;
            if tab_view != GameProgress::Finished {
                c.result = None;
            };
        });
        ctx.has_more.set_value(true);
        ctx.is_first_batch.set_value(true);
        ctx.games.set(vec![]);
    });
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| {
            let controls = ctx.controls.get();
            let username = username();
            let batch_info = ctx.games.get().last().map(|game| BatchInfo {
                id: game.uuid,
                timestamp: game.updated_at,
            });
            ctx.is_first_batch.set_value(batch_info.is_none());
            async move {
                if !ctx.has_more.get_value() || ctx.next_batch.pending().get() {
                    return;
                }
                load_games(controls, username, batch_info, ctx.next_batch);
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(300.0),
    );
    view! {
        <div
            node_ref=el
            class="overflow-x-hidden items-center h-full sm:grid sm:grid-cols-2 sm:gap-1"
        >
            {move || {
                ctx.games.get().into_iter().map(|game| view! { <GameRow game /> }).collect_view()
            }}
        </div>
    }
}
