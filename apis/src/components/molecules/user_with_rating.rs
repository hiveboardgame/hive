use hive_lib::Color;
use leptos::*;
use shared_types::GameSpeed;

use crate::{
    components::{
        atoms::{profile_link::ProfileLink, rating::Rating, status_indicator::StatusIndicator},
        molecules::rating_and_change::RatingAndChangeDynamic,
    },
    providers::game_state::GameStateSignal,
};

#[component]
pub fn UserWithRating(
    side: Color,
    #[prop(optional)] text_color: &'static str,
    #[prop(optional)] is_tall: Signal<bool>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_response = create_read_slice(game_state.signal, |gs| gs.game_response.clone());
    let player = move || match side {
        Color::White => game_response().map(|g| g.white_player),
        Color::Black => game_response().map(|g| g.black_player),
    };
    let speed = move || {
        game_response().map(|resp| match resp.speed {
            GameSpeed::Untimed => GameSpeed::Correspondence,
            _ => resp.speed,
        })
    };
    let username = move || player().map_or(String::new(), |p| p.username);
    let patreon = move || player().map_or(false, |p| p.patreon);
    let rating = move || match (player(), speed()) {
        (Some(player), Some(speed)) => {
            view! { <Rating rating=player.ratings.get(&speed).expect("Valid rating from speed").clone() /> }
        }
        _ => view! { "" }.into_view(),
    };
    view! {
        <div class=move || {
            format!(
                "ml-1 flex items-center {} justify-center",
                if is_tall() { "flex-row gap-1" } else { "flex-col" },
            )
        }>
            {move || {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=username() />
                        <ProfileLink
                            patreon=patreon()
                            username=username()
                            extend_tw_classes=text_color
                        />
                    </div>
                }
            }}
            <Show
                when=game_state.is_finished()
                fallback=move || {
                    view! { <div class=format!("{text_color}")>{rating}</div> }
                }
            >

                <RatingAndChangeDynamic extend_tw_classes=text_color side=side />
            </Show>
        </div>
    }
}
