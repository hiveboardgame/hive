use hive_lib::color::Color;
use leptos::*;

use crate::functions::games::game_response::GameStateResponse;

#[component]
pub fn FinishedRating(
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
    let (sign, diff_style) = if rating_change == 0 {
        ("+", "text-cyan-400")
    } else if rating_change < 0 {
        ("", "text-red-400")
    } else {
        ("+", "text-green-400")
    };
    view! {
        <p class=extend_tw_classes>{rating}</p>
        <p class=move || format!("{diff_style}")>{sign} {rating_change}</p>
    }
}
