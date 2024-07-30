use crate::functions::users::get::{
    get_finished_games_in_batches, get_ongoing_games, get_user_by_username,
};
use crate::responses::GameResponse;
use crate::responses::UserResponse;
use chrono::{DateTime, Utc};
use hive_lib::GameStatus;
use leptos::*;
use leptos_router::*;
use shared_types::GameStart;
use uuid::Uuid;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[derive(Clone, PartialEq, Copy)]
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
    pub user: UserResponse,
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
    let last_timestamp = RwSignal::new(None);
    let last_id = RwSignal::new(None);
    let children = store_value(children);
    let first_batch_finished = Resource::new(username, move |_| {
        get_finished_games_in_batches(
            username(),
            last_timestamp.get_untracked(),
            last_id.get_untracked(),
            5,
        )
    });
    let ongoing_games = Resource::new(username, move |_| get_ongoing_games(username()));
    view! {
        <div class="flex flex-col pt-12 bg-light dark:bg-gray-950">
            <Suspense>
                {move || {
                    let (first_batch, more_finished) = first_batch_finished()
                        .and_then(|games| games.ok())
                        .unwrap_or((Vec::new(), false));
                    let mut ongoing_games = ongoing_games()
                        .and_then(|games| games.ok())
                        .unwrap_or(Vec::new());
                    let mut unstarted = Vec::new();
                    ongoing_games
                        .retain(|gr| {
                            if gr.game_start == GameStart::Ready
                                && gr.game_status == GameStatus::NotStarted
                            {
                                unstarted.push(gr.clone());
                                false
                            } else {
                                true
                            }
                        });
                    let first_batch = store_value(first_batch);
                    let playing = RwSignal::new(ongoing_games);
                    let unstarted = RwSignal::new(unstarted);
                    let finished = RwSignal::new(first_batch());
                    last_id.update(move |v| { *v = first_batch().last().map(|gr| gr.uuid) });
                    last_timestamp
                        .update(move |v| { *v = first_batch().last().map(|gr| gr.updated_at) });
                    user()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                            Ok(user) => {
                                provide_context(ProfileGamesContext {
                                    unstarted,
                                    finished,
                                    playing,
                                    finished_last_timestamp: last_timestamp,
                                    finished_last_id: last_id,
                                    more_finished: RwSignal::new(more_finished),
                                    user,
                                });
                                children()().into_view()
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}
