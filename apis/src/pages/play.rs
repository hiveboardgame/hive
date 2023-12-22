use crate::{
    common::time_control::TimeControl,
    components::organisms::{board::Board, side_board::SideboardTabs, timer::DisplayTimer},
    functions::games::get::get_game_from_nanoid,
    providers::{
        auth_context::AuthContext,
        game_state::GameStateSignal,
        games_controller::{self, GamesController, GamesControllerSignal},
        web_socket::WebsocketContext,
    },
};
use hive_lib::{color::Color, position::Position};
use http::uri::Authority;
use leptos::ev::load;
use leptos::logging::log;
use leptos::*;
use leptos_router::*;
use leptos_use::{use_document, use_event_listener};
use leptos_use::{use_websocket, use_window};

#[derive(Params, PartialEq, Eq)]
struct PlayParams {
    nanoid: String,
}

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let params = use_params::<PlayParams>();
    // TODO: move the time_control to the gamestate
    // let time_control = store_value(TimeControl::RealTime(
    //     Duration::from_secs(60),
    //     Duration::from_secs(10),
    // ));
    let time_control = store_value(TimeControl::Untimed);
    let nanoid =
        move || params.with(|params| params.as_ref().map(|params| params.nanoid.clone()).ok());

    let websocket_present = Signal::derive(move || use_context::<WebsocketContext>().is_some());

    let username = move || {
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(user))) = (auth_context.user)() {
            return Some(user.username);
        }
        None
    };

    let get_game = move || {
        let mut games_controller = expect_context::<GamesControllerSignal>();
        if let Some(nanoid) = nanoid() {
            games_controller.join(username(), nanoid);
        }
    };

    create_effect(move |_| {
        log!("Runs the effect");
        if websocket_present.get() {
            log!("Websocket is here now");
            get_game();
        }
    });

    view! {
        <Transition>
            {move || {
                view! {
                    <div
                        class=format!(
                        "grid grid-cols-10 grid-rows-6 h-full w-full max-h-[93vh] min-h-[93vh] {extend_tw_classes}",
                    )>
                        <Board/>
                        <DisplayTimer side=Color::White time_control=time_control()/>
                        <SideboardTabs/>
                        <DisplayTimer side=Color::Black time_control=time_control()/>
                    </div>
                }
            }}

        </Transition>
    }
}
