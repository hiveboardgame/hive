use crate::common::RatingChangeInfo;
use crate::providers::game_state::GameStateSignal;
use hive_lib::Color;
use leptos::*;
use std::cmp::Ordering;

#[component]
pub fn RatingAndChange(
    #[prop(optional)] extend_tw_classes: &'static str,
    ratings: StoredValue<RatingChangeInfo>,
    side: Color,
) -> impl IntoView {
    let ratings = ratings();
    let (rating_change, rating) = match side {
        Color::White => (ratings.white_rating_change, ratings.white_rating),

        Color::Black => (ratings.black_rating_change, ratings.black_rating),
    };
    let (sign, style) = match rating_change.cmp(&0_i64) {
        Ordering::Equal => ("+", "text-pillbug-teal"),
        Ordering::Less => ("", "text-ladybug-red"),
        Ordering::Greater => ("+", "text-grasshopper-green"),
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
    let game_state = expect_context::<GameStateSignal>();
    let ratings = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map(RatingChangeInfo::from_game_response)
    });
    view! {
        {move || {
            ratings()
                .map(|ratings| {
                    let ratings = StoredValue::new(ratings);
                    view! {
                        <RatingAndChange extend_tw_classes=extend_tw_classes ratings side=side/>
                    }
                })
        }}
    }
}
