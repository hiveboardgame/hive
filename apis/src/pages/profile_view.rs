use crate::{
    components::organisms::display_profile::DisplayProfile,
    functions::users::get::{get_user_by_username, get_user_games},
    responses::game::GameResponse,
};
use hive_lib::game_status::GameStatus;
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[derive(Clone, PartialEq)]
pub enum ProfileGamesView {
    Playing,
    Finished,
}

#[derive(Debug, Clone)]
pub struct AllUserGames {
    pub playing: Vec<GameResponse>,
    pub finished: Vec<GameResponse>,
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let username = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.username.clone())
                .unwrap_or_default()
        })
    };
    let user = Resource::new(username, move |_| get_user_by_username(username()));
    let games = Resource::new(username, move |_| get_user_games(username()));
    let stored_children = store_value(children);
    let tab_view = create_rw_signal(ProfileGamesView::Playing);
    let active = move |view: ProfileGamesView| {
        let button_style = String::from("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded");
        if tab_view() == view {
            button_style + " bg-pillbug-teal"
        } else {
            button_style + " bg-ant-blue"
        }
    };
    provide_context(tab_view);

    view! {
        <div class="bg-light dark:bg-dark pt-12">
            <Transition>
                {move || {
                    let partitioned_games = games()
                        .and_then(|games| games.ok())
                        .map(|mut games| {
                            games.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                            games
                                .into_iter()
                                .partition(|game| {
                                    matches!(game.game_status, GameStatus::Finished(_))
                                })
                        })
                        .unwrap_or((Vec::new(), Vec::new()));
                    provide_context(AllUserGames {
                        finished: partitioned_games.0,
                        playing: partitioned_games.1,
                    });
                    user()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                            Ok(user) => {
                                view! {
                                    // TODO: in the future data will come from a WS call but for now:

                                    <DisplayProfile user=store_value(user)/>
                                    <div class="flex gap-1 ml-3">
                                        <A
                                            href="playing"
                                            class=move || active(ProfileGamesView::Playing)
                                        >
                                            "Playing "
                                        </A>
                                        <A
                                            href="finished"
                                            class=move || active(ProfileGamesView::Finished)
                                        >
                                            "Finished Games "
                                        </A>
                                    </div>
                                    {stored_children()()}
                                }
                                    .into_view()
                            }
                        })
                }}

            </Transition>
        </div>
    }
}
