use crate::{
    common::challenge_action::ChallengeVisibility,
    providers::{api_requests::ApiRequests, color_scheme::ColorScheme},
};
use hive_lib::{color::ColorChoice, game_type::GameType};
use leptos::*;
use leptos_icons::{
    BsIcon::{BsHexagon, BsHexagonFill, BsHexagonHalf},
    Icon,
};

#[derive(Debug, Clone)]
pub struct ChallengeParams {
    pub rated: bool,
    pub game_type: GameType,
    pub visibility: ChallengeVisibility,
    pub opponent: Option<String>,
    pub color_choice: ColorChoice,
}

#[component]
pub fn ChallengeCreate(close: Callback<()>) -> impl IntoView {
    let params = RwSignal::new(ChallengeParams {
        rated: true,
        game_type: GameType::MLP,
        visibility: ChallengeVisibility::Public,
        opponent: None,
        color_choice: ColorChoice::Random,
    });
    let rated = move |_| params.update_untracked(|p| p.rated = true);
    let unrated = move |_| params.update_untracked(|p| p.rated = false);
    let base = move |_| params.update_untracked(|p| p.game_type = GameType::Base);
    let mlp = move |_| params.update_untracked(|p| p.game_type = GameType::MLP);
    let public = move |_| params.update_untracked(|p| p.visibility = ChallengeVisibility::Public);
    let private = move |_| params.update_untracked(|p| p.visibility = ChallengeVisibility::Private);
    let create_challenge = move || {
        let api = ApiRequests::new();
        api.challenge_new_with_params(params.get_untracked());
        close(());
    };
    let white = move |_| {
        params.update_untracked(|p| p.color_choice = ColorChoice::White);
        create_challenge()
    };
    let random = move |_| {
        params.update_untracked(|p| p.color_choice = ColorChoice::Random);
        create_challenge()
    };
    let black = move |_| {
        params.update_untracked(|p| p.color_choice = ColorChoice::Black);
        create_challenge()
    };
    let color_context = expect_context::<ColorScheme>;
    let icon = move |color_choice: ColorChoice| match color_choice {
        ColorChoice::Random => {
            view! { <Icon icon=Icon::from(BsHexagonHalf)/> }
        }
        ColorChoice::White => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-white"/> }
            } else {
                view! { <Icon icon=Icon::from(BsHexagon) class="stroke-black"/> }
            }
        }
        ColorChoice::Black => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=Icon::from(BsHexagon) class="stroke-white"/> }
            } else {
                view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-black"/> }
            }
        }
    };
    let create_buttons_style = "mx-1 w-14 h-14 p-2";

    view! {
        <div class="flex flex-col">
            <div>
                <button on:click=rated>Rated</button>
                <button on:click=unrated>Casual</button>
            </div>
            <div>
                <button on:click=base>Base</button>
                <button on:click=mlp>MLP</button>
            </div>
            <div>
                <button on:click=public>Public</button>
                <button on:click=private>Private</button>
            </div>
            <div>
                <button class=create_buttons_style on:click=white>
                    {move || { icon(ColorChoice::White) }}
                </button>
                <button class="place-content-center mx-1 w-20 h-20 p-2" on:click=random>
                    {move || { icon(ColorChoice::Random) }}
                </button>
                <button class=create_buttons_style on:click=black>
                    {move || { icon(ColorChoice::Black) }}
                </button>
            </div>
        </div>
    }
}
