use crate::{
    common::RatingChangeInfo,
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use hive_lib::Color;
use leptos::prelude::*;

#[component]
pub fn RatingAndChange(
    #[prop(optional)] extend_tw_classes: &'static str,
    ratings: StoredValue<RatingChangeInfo>,
    side: Color,
) -> impl IntoView {
    let ratings = ratings.get_value();
    let (rating_change, rating) = match side {
        Color::White => (ratings.white_rating_change, ratings.white_rating),

        Color::Black => (ratings.black_rating_change, ratings.black_rating),
    };
    let (sign, style, magnitude) = rating_change_appearance(rating_change);
    let precise = format!("{rating_change:+.2}");

    view! {
        <p class=extend_tw_classes>{rating}</p>
        <p class=format!("{style} cursor-help") title=precise>
            {sign}
            {magnitude}
        </p>
    }
}

#[component]
pub fn RatingAndChangeDynamic(
    #[prop(optional)] extend_tw_classes: &'static str,
    side: Color,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let ratings = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(RatingChangeInfo::from_game_response)
        })
    });
    view! {
        {move || {
            ratings()
                .map(|ratings| {
                    let ratings = StoredValue::new(ratings);
                    view! {
                        <RatingAndChange extend_tw_classes=extend_tw_classes ratings side=side />
                    }
                })
        }}
    }
}

fn rating_change_appearance(change: f64) -> (&'static str, &'static str, i64) {
    let (sign, style) = if change < 0.0 {
        ("-", "text-ladybug-red")
    } else if change > 0.0 {
        ("+", "text-grasshopper-green")
    } else {
        ("+", "text-pillbug-teal")
    };
    (sign, style, change.abs().round() as i64)
}
