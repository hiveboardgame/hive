use hive_lib::Color;
use leptos::{either::Either, prelude::*};
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
    #[prop(into)] side: Signal<Color>,
    #[prop(optional)] text_color: &'static str,
    #[prop(optional)] vertical: bool,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let game_response = create_read_slice(game_state.signal, |gs| gs.game_response.clone());
    let player = Signal::derive(move || match side() {
        Color::White => game_response.with(|g| g.as_ref().map(|g| g.white_player.clone())),
        Color::Black => game_response.with(|g| g.as_ref().map(|g| g.black_player.clone())),
    });
    let speed = move || {
        game_response.with(|g| {
            g.as_ref().map(|resp| match resp.speed {
                GameSpeed::Untimed => GameSpeed::Correspondence,
                _ => resp.speed.clone(),
            })
        })
    };
    let username = Signal::derive(move || {
        player.with(|p| p.as_ref().map_or(String::new(), |p| p.username.clone()))
    });
    let patreon = move || player.with(|p| p.as_ref().is_some_and(|p| p.patreon));
    let bot = move || player.with(|p| p.as_ref().is_some_and(|p| p.bot));
    let rating = move || {
        player.with(|p| {
            p.as_ref().and_then(|player| {
                speed().map(|speed| {
                    Either::Left(view! { <Rating rating=player.ratings.get(&speed).expect("Valid rating from speed").clone() /> })
                })
            }).unwrap_or_else(|| Either::Right(view! { "" }))
        })
    };
    view! {
        <div class=format!(
            "ml-1 flex items-center {} justify-center",
            if vertical { "flex-row gap-1" } else { "flex-col" },
        )>
            {move || {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=username() />
                        <ProfileLink
                            patreon=patreon()
                            username=username()
                            bot=bot()
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

                <RatingAndChangeDynamic extend_tw_classes=text_color side=side() />
            </Show>
        </div>
    }
}
