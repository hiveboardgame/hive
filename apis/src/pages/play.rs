use crate::{
    common::time_control::TimeControl,
    components::organisms::{board::Board, side_board::SideboardTabs, timer::DisplayTimer},
    functions::games::get::get_game_from_nanoid,
    providers::{auth_context::AuthContext, game_state::GameStateSignal},
};
use hive_lib::{
    color::Color, game_result::GameResult, game_status::GameStatus, history::History,
    position::Position, state::State,
};
use leptos::logging::log;
use leptos::*;
use leptos_router::*;
use std::time::Duration;
use uuid::Uuid;

#[derive(Params, PartialEq, Eq)]
struct PlayParams {
    nanoid: String,
}

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));

    let auth_context = expect_context::<AuthContext>();
    let params = use_params::<PlayParams>();
    // TODO: move the time_control to the gamestate
    // let time_control = store_value(TimeControl::RealTime(
    //     Duration::from_secs(60),
    //     Duration::from_secs(10),
    // ));
    let time_control = store_value(TimeControl::Untimed);
    let nanoid = move || {
        params.with_untracked(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };
    let game = create_local_resource(nanoid, move |_| get_game_from_nanoid(nanoid()));
    let user_uuid = move || match untrack(auth_context.user) {
        Some(Ok(Some(user))) => user.id,
        _ => {
            log! {
                "Generating random uuid for anon"
            }
            Uuid::new_v4()
        }
    };

    view! {
        <Transition>
            {move || {
                game()
                    .map(|data| match data {
                        Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                        Ok(game) => {
                            let mut game_state = expect_context::<GameStateSignal>();
                            game_state.set_game_id(store_value(nanoid()));
                            let result = match game.game_status {
                                GameStatus::NotStarted | GameStatus::InProgress => {
                                    GameResult::Unknown
                                }
                                GameStatus::Finished(result) => result,
                            };
                            let state = State::new_from_history(
                                    &History::new_from_gamestate(
                                        game.history,
                                        result,
                                        game.game_type,
                                    ),
                                )
                                .expect("State to be valid");
                            game_state
                                .set_state(state, game.white_player.uid, game.black_player.uid);
                            game_state.join(user_uuid());
                            let white_player = store_value(game.white_player);
                            let black_player = store_value(game.black_player);
                            view! {
                                <div class=format!(
                                    "grid grid-cols-10 grid-rows-6 max-h-[94svh] {extend_tw_classes}",
                                )>

                                    <Board/>
                                    <DisplayTimer
                                        side=Color::White
                                        player=white_player
                                        time_control=time_control()
                                    />
                                    <SideboardTabs extend_tw_classes="border-blue-200"/>
                                    <DisplayTimer
                                        side=Color::Black
                                        player=black_player
                                        time_control=time_control()
                                    />
                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}

