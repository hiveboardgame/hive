use crate::providers::auth_context::AuthContext;
use crate::providers::game_controller::GameControllerSignal;
use crate::{
    common::time_control::TimeControl,
    components::organisms::{board::Board, side_board::SideboardTabs, timer::DisplayTimer},
};
use hive_lib::{color::Color, position::Position};
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos::*;
use leptos_router::use_params;
use leptos_router::*;
use regex::Regex;

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
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };
    // TODO: move the time_control to the gamestate
    // let time_control = store_value(TimeControl::RealTime(
    //     Duration::from_secs(60),
    //     Duration::from_secs(10),
    // ));
    let time_control = store_value(TimeControl::Untimed);
    lazy_static! {
        static ref NANOID: Regex =
            Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
    }

    create_effect(move |_| {
        let auth_context = expect_context::<AuthContext>();
        let acc_resp = (auth_context.user)();
        let username = if let Some(Ok(Some(user))) = acc_resp {
            Some(user.username)
        } else {
            None
        };
        log!("User: {:?}", username);
        let mut game_controller = expect_context::<GameControllerSignal>();
        let nanoid = nanoid();
        game_controller.join(nanoid, username);
    });

    view! {
        <div class=format!(
            "grid grid-cols-10 grid-rows-6 h-full w-full max-h-[93vh] min-h-[93vh] {extend_tw_classes}",
        )>

            <Board/>
            <DisplayTimer side=Color::White time_control=time_control()/>
            <SideboardTabs/>
            <DisplayTimer side=Color::Black time_control=time_control()/>
        </div>
    }
}
