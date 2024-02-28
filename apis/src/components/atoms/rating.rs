use crate::responses::rating::RatingResponse;
use leptos::*;
use leptos_icons::*;
use shared_types::game_speed::GameSpeed;

#[component]
pub fn Rating(rating: Option<RatingResponse>) -> impl IntoView {
    // TODO: @ion please style this nicely <3
    // maybe do a IconRating and (Plain)Rating?
    if let Some(rating) = rating {
        use GameSpeed::*;
        // TODO: find some nice icons for the different speeds
        let icon = move || match rating.speed {
            Untimed => icondata::BiInfiniteRegular,
            Blitz => icondata::BiStopwatchRegular,
            Bullet => icondata::BiStopwatchRegular,
            Rapid => icondata::BiStopwatchRegular,
            Classic => icondata::AiMailOutlined,
            Correspondence => icondata::AiMailOutlined,
        };
        return view! {
            <p> <Icon icon=icon() class="w-full h-full"/> {rating.rating}</p>
        }
        .into_view();
    }
    view! {}.into_view()
}
