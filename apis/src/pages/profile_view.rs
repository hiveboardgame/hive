use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::components::molecules::user_row::UserRow;
use crate::functions::games::get::get_batch_from_options;
use crate::functions::users::get_profile;
use crate::i18n::*;
use crate::responses::GameResponse;
use crate::components::{molecules::game_row::GameRow, organisms::display_profile::DisplayProfile};
use hive_lib::Color;
use hooks::use_params;
use leptos::either::Either;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::{params::Params, *};
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions,};
use shared_types::{BatchInfo, GameProgress, GameSpeed, GamesQueryOptions, ResultType};


#[derive(Debug, Clone, Default)]
struct ProfileControls {
    pub color: Option<Color>,
    pub result: Option<ResultType>,
    pub speeds: Vec<GameSpeed>,
    pub tab_view: GameProgress,
}

#[derive(Debug, Clone)]
struct ProfileGamesContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub controls: RwSignal<ProfileControls>,
}

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[component]
fn Controls(username: String, ctx: ProfileGamesContext) -> impl IntoView {
    let username = Signal::derive(move || username.clone());
    let controls = ctx.controls;
    let i18n = use_i18n();
    let toggle_classes = |active| {
        format!("flex hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-3 rounded bg-button-dawn dark:bg-button-twilight {}", if active { "bg-pillbug-teal" } else { "" })
    };
    let radio_classes = |active| {
        format!("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-2 rounded bg-button-dawn dark:bg-button-twilight {}", if active { "bg-pillbug-teal" } else { "" })
    };
    let delay = 250.0;
    let toggle_speeds = move |speed| {
        controls.update(|c| {
            if c.speeds.contains(&speed) {
                c.speeds.retain(|s| s != &speed);
            } else {
                c.speeds.push(speed);
            }
        });
        ctx.games.set(Vec::new());
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
                on:click=move |_| {controls.update(|c| c.color = Some(Color::Black)); ctx.games.set(Vec::new());}
                class=move || radio_classes(controls().color == Some(Color::Black))>
                    {t!(i18n, profile.color_buttons.black)}
                </button>
                <button
                on:click=move |_| {controls.update(|c| c.color = Some(Color::White)); ctx.games.set(Vec::new());}
                class=move || radio_classes(controls().color == Some(Color::White))>
                    {t!(i18n, profile.color_buttons.white)}
                </button>
                <button
                on:click=move |_| {controls.update(|c| c.color = None); ctx.games.set(Vec::new());}
                class=move || radio_classes(controls().color.is_none())>
                    {t!(i18n, profile.color_buttons.both)}
                </button>
            </div>
            <Show when=move || controls().tab_view == GameProgress::Finished>
                <div class="font-bold text-md">{t!(i18n, profile.game_result)}</div>
                <div class="flex gap-1">
                    <button class=move || radio_classes(controls().result == Some(ResultType::Win))
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Win)); ctx.games.set(Vec::new());}>
                        {t!(i18n, profile.result_buttons.win)}
                    </button>
                    <button
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Loss)); ctx.games.set(Vec::new());}
                    class=move || radio_classes(controls().result == Some(ResultType::Loss))>
                        {t!(i18n, profile.result_buttons.loss)}
                    </button>
                    <button
                    on:click=move |_| {controls.update(|c| c.result = Some(ResultType::Draw)); ctx.games.set(Vec::new());}
                     class=move || radio_classes(controls().result == Some(ResultType::Draw))>
                        {t!(i18n, profile.result_buttons.draw)}
                    </button>
                    <button
                    on:click=move |_| {controls.update(|c| c.result = None); ctx.games.set(Vec::new());}
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
    let params = use_params::<UsernameParams>();
    let username = move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let user = LocalResource::new(move || get_profile(username()));
    provide_context(
        ProfileGamesContext {
            controls: RwSignal::new(ProfileControls {
                speeds:vec![
                    GameSpeed::Bullet,
                    GameSpeed::Blitz,
                    GameSpeed::Rapid,
                    GameSpeed::Classic,
                    GameSpeed::Correspondence,
                    GameSpeed::Untimed,
                ],
                ..Default::default()
            }),
            games: RwSignal::new(Vec::new()),
        }
    );
    let ctx = expect_context::<ProfileGamesContext>();
    view! {
        <div class="flex flex-col pt-12 mx-3 bg-light dark:bg-gray-950">
        <Suspense
        fallback=move || view! { <p>"Loading Profile..."</p> }
    >
    {move ||
        user.get().as_deref().map(|user| {
            if let Ok(user) = user {
                Either::Left(view! {
                    <div class="flex justify-center w-full text-lg sm:text-xl">
            <UserRow
                actions=vec![UserAction::Challenge]
                user=StoredValue::new(user.clone())
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
                })
            } else {
                Either::Right(view! { <p>"User not found"</p> })
            }
        })
    }
    </Suspense>
        </div>
    }
}

#[component]
pub fn DisplayGames(tab_view: GameProgress) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let params = use_params::<UsernameParams>();
    let username = move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let el = NodeRef::<html::Div>::new();
    el.on_load(move |_| {
        ctx.controls.update(|c| {
            c.tab_view = tab_view;
            if tab_view != GameProgress::Finished {
                c.result = None;
            };
        });
        ctx.games.set(Vec::new());
    });
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            let batch_info = ctx.games.get().last().map(|game| BatchInfo {
                id: game.uuid,
                timestamp: game.updated_at,
            });
            let controls = ctx.controls.get();
            let user_info =  (username(), controls.color, controls.result);

            let batch_size = if batch_info.is_some() { Some(4) } else { Some(6) };
            let options = GamesQueryOptions {
                players: vec![user_info],
                speeds: controls.speeds,
                current_batch: batch_info,
                batch_size,
                game_progress: tab_view,
            };
            let next_batch = get_batch_from_options(options).await;
            if let Ok(next_batch) = next_batch {
                ctx.games.update(|games| games.extend(next_batch));
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
