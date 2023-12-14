use hive_lib::color::Color;
use leptos::*;

use crate::{
    functions::games::{game_response::GameStateResponse, get::get_game_from_nanoid},
    providers::game_state::GameStateSignal,
};

#[component]
pub fn RatingAndChange(
    #[prop(optional)] extend_tw_classes: &'static str,
    game: GameStateResponse,
    side: Color,
) -> impl IntoView {
    let (rating_change, rating) = match side {
        Color::White => {
            if game.rated {
                (
                    game.white_rating_change.unwrap_or_default() as i64,
                    game.white_rating.unwrap_or_default() as u64,
                )
            } else {
                (0 as i64, game.white_rating.unwrap_or_default() as u64)
            }
        }
        Color::Black => {
            if game.rated {
                (
                    game.black_rating_change.unwrap_or_default() as i64,
                    game.black_rating.unwrap_or_default() as u64,
                )
            } else {
                (0, game.black_rating.unwrap_or_default() as u64)
            }
        }
    };
    let (sign, style) = if rating_change == 0 {
        ("+", "text-cyan-400")
    } else if rating_change < 0 {
        ("", "text-li-red")
    } else {
        ("+", "text-li-green")
    };

    view! {
        <p class=extend_tw_classes>{rating}</p>
        <p class=move || { style }>{sign} {rating_change}</p>
    }
}

#[component]
pub fn RatingAndChangeDynamic(
    #[prop(optional)] extend_tw_classes: &'static str,
    side: Color,
) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let game_id = move || game_state_signal.signal.get_untracked().game_id;
    view! {
        <Show when=move || {
            game_id().is_some()
        }>
            {move || {
                let game = Resource::once(move || get_game_from_nanoid(
                    (game_id().expect("Some game_id"))(),
                ));
                view! {
                    <Transition>
                        {move || {
                            game()
                                .map(|data| match data {
                                    Err(_) => view! { <pre>"Error"</pre> }.into_view(),
                                    Ok(game) => {
                                        view! {
                                            <RatingAndChange
                                                extend_tw_classes=extend_tw_classes
                                                game=game
                                                side=side
                                            />
                                        }
                                            .into_view()
                                    }
                                })
                        }}

                    </Transition>
                }
            }}

        </Show>
    }
}
