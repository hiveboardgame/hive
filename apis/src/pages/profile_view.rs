use crate::{
    components::organisms::display_profile::DisplayProfile,
    functions::users::get::{
        get_finished_games_in_batches, get_ongoing_games, get_user_by_username,
    },
    responses::GameResponse,
};
use hive_lib::GameStatus;
use leptos::{ev::scroll, *};
use leptos_router::*;
use leptos_use::{use_document, use_event_listener, use_throttle_fn, use_window};
use shared_types::GameStart;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[derive(Clone, PartialEq)]
pub enum ProfileGamesView {
    Unstarted,
    Playing,
    Finished,
}

#[derive(Debug, Clone)]
pub struct AllUserGames {
    pub unstarted: RwSignal<Vec<GameResponse>>,
    pub playing: RwSignal<Vec<GameResponse>>,
    pub finished: RwSignal<Vec<GameResponse>>,
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    //TODO: @ion fix finished.clear()
    let finished: RwSignal<Vec<GameResponse>> = RwSignal::new(Vec::new());
    let username = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.username.clone())
                .unwrap_or_default()
        })
    };
    let user = Resource::new(username, move |_| get_user_by_username(username()));
    let get_more = RwSignal::new(0);
    let last_timestamp = RwSignal::new(None);
    let last_id = RwSignal::new(None);
    let finished_games = Resource::new(
        move || (username(), get_more()),
        move |_| {
            get_finished_games_in_batches(
                username(),
                last_timestamp.get_untracked(),
                last_id.get_untracked(),
                3,
            )
        },
    );
    let ongoing_games = Resource::new(username, move |_| get_ongoing_games(username()));
    let stored_children = store_value(children);
    let tab_view = create_rw_signal(ProfileGamesView::Playing);
    let active = move |view: ProfileGamesView| {
        let button_style = String::from("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded");
        if tab_view() == view {
            button_style + " bg-pillbug-teal"
        } else {
            button_style + " bg-button-dawn dark:bg-button-twilight"
        }
    };
    let still_more_games = RwSignal::from(true);
    let throttled_more_games = use_throttle_fn(
        move || {
            if tab_view.get_untracked() == ProfileGamesView::Finished
                && is_end_of_page()
                && still_more_games.get_untracked()
            {
                get_more.update(|v| *v += 1)
            }
        },
        500.0,
    );
    provide_context(tab_view);
    _ = use_event_listener(use_window(), scroll, move |_| {
        throttled_more_games();
    });

    view! {
        <div class="flex flex-col pt-12 bg-light dark:bg-gray-950">
            <Transition>
                {move || {
                    let (current_finished_games, more_games) = finished_games()
                        .and_then(|games| games.ok())
                        .unwrap_or((Vec::new(), false));
                    let mut ongoing_games = ongoing_games()
                        .and_then(|games| games.ok())
                        .unwrap_or(Vec::new());
                    let mut unstarted = Vec::new();
                    ongoing_games.retain(|gr| {
                        if gr.game_start == GameStart::Ready && gr.game_status == GameStatus::NotStarted {
                            unstarted.push(gr.clone());
                            false
                        } else {
                            true
                        }
                    });
                    finished.update(move |v| v.extend(current_finished_games));
                    still_more_games.set(more_games);
                    let playing = RwSignal::from(ongoing_games);
                    let unstarted = RwSignal::from(unstarted);
                    last_id.update(move |v| { *v = finished().last().map(|gr| gr.uuid) });
                    last_timestamp
                        .update(move |v| { *v = finished().last().map(|gr| gr.updated_at) });
                    provide_context(AllUserGames {
                        unstarted,
                        finished,
                        playing,
                    });
                    user()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                            Ok(user) => {
                                view! {
                                    <DisplayProfile user=store_value(user)/>
                                    <div class="flex gap-1 ml-3">
                                        <Show when= move || !unstarted().is_empty()>
                                        <A
                                            href="unstarted"
                                            class=move || active(ProfileGamesView::Unstarted)
                                            on:click=move |_| finished.update(|v| v.clear())
                                        >
                                            "Unstarted Tournament Games"
                                        </A>
                                        </Show>
                                        <Show when= move || !playing().is_empty()>
                                        <A
                                            href="playing"
                                            class=move || active(ProfileGamesView::Playing)
                                            on:click=move |_| finished.update(|v| v.clear())
                                        >
                                            "Playing "
                                        </A>
                                        </Show>
                                        <Show when= move || !finished().is_empty()>
                                        <A
                                            href="finished"
                                            class=move || active(ProfileGamesView::Finished)
                                            on:click=move |_| finished.update(|v| v.clear())
                                        >
                                            "Finished Games "
                                        </A>
                                        </Show>
                                    </div>
                                    {stored_children()()}
                                    <Show when=finished_games.loading()>
                                        <div class="place-self-center p-4 w-5 h-5 rounded-full border-t-2 border-b-2 border-blue-500 animate-spin"></div>
                                    </Show>
                                }
                                    .into_view()
                            }
                        })
                }}

            </Transition>
        </div>
    }
}

fn is_end_of_page() -> bool {
    let document = use_document();
    const OFFSET_PX: f64 = 200.0;
    let inner_height = window()
        .inner_height()
        .expect("window")
        .as_f64()
        .expect("Converted to f64");
    let page_y_offset = window().page_y_offset().expect("window again");
    let body_offset_height = document.body().expect("Body").offset_height() as f64;

    inner_height + page_y_offset >= body_offset_height - OFFSET_PX
}
