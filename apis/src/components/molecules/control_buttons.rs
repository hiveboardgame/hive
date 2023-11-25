use hive_lib::game_status::GameStatus;
use leptos::*;
use leptos_icons::{
    AiIcon::AiFlagOutlined, AiIcon::AiStopOutlined, BiIcon::BiUndoRegular,
    FaIcon::FaHandshakeSimpleSolid,
};

use crate::{
    components::atoms::confirm_button::ConfirmButton, providers::game_state::GameStateSignal,
};
use leptos::logging::log;

#[component]
pub fn ControlButtons() -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let is_started = move || match (game_state.signal)().state.game_status {
        GameStatus::NotStarted => false,
        _ => true,
    };
    let is_finished = move || match (game_state.signal)().state.game_status {
        GameStatus::Finished(_) => true,
        _ => false,
    };
    let do_the_thing = Callback::<()>::from(move |_| log!("Abort/Undo/Draw/Resign"));

    view! {
        <Show
            when=is_finished
            fallback=move || {
                view! {
                    <div class="flex justify-around items-center min-w-fit min-h-fit">
                        <Show
                            when=is_started
                            fallback=move || {
                                view! {
                                    <ConfirmButton
                                        icon=leptos_icons::Icon::Ai(AiStopOutlined)
                                        action=do_the_thing
                                    />
                                }
                            }
                        >

                            <ConfirmButton
                                icon=leptos_icons::Icon::Bi(BiUndoRegular)
                                action=do_the_thing
                            />
                        </Show>

                        <ConfirmButton
                            icon=leptos_icons::Icon::Fa(FaHandshakeSimpleSolid)
                            action=do_the_thing
                        />
                        <ConfirmButton
                            icon=leptos_icons::Icon::Ai(AiFlagOutlined)
                            action=do_the_thing
                        />
                    </div>
                }
            }
        >

            Rematch button/new game button
        </Show>
    }
}

