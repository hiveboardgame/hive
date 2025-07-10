use crate::components::layouts::base_layout::OrientationSignal;
use crate::components::molecules::hamburger::Hamburger;
use crate::components::organisms::chat::ChatWindow;
use crate::providers::chat::Chat;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, SimpleDestination};

#[component]
pub fn ChatDropdown(destination: SimpleDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let hamburger_show = expect_context::<OrientationSignal>().chat_dropdown_open;
    let chat_style = "absolute z-50 flex-col w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let button_color = move || {
        if hamburger_show() {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        } else if chat.has_messages(game_id()) {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    };

    Effect::new(move |_| {
        hamburger_show();
        chat.seen_messages(game_id());
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                format!(
                    "{} transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded",
                    button_color(),
                )
            })

            dropdown_style=chat_style
            content=view! { <Icon icon=icondata::BiChatRegular attr:class="w-4 h-4" /> }
            id="chat"
        >
            <ChatWindow destination=destination.clone() />
        </Hamburger>
    }
}
