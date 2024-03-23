use crate::responses::rating::RatingResponse;
use icondata::Icon;
use leptos::*;
use leptos_icons::*;
use shared_types::{certainty::Certainty, game_speed::GameSpeed};

#[component]
pub fn Rating(rating: RatingResponse) -> impl IntoView {
    let certainty_str = match rating.certainty {
        Certainty::Rankable => "",
        _ => "?",
    };
    view! {
        {rating.rating}
        {certainty_str}
    }
}

#[component]
pub fn RatingWithIcon(rating: StoredValue<RatingResponse>) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center gap-1">
            <Icon icon=icon_for_speed(&rating().speed)/>
            <Rating rating=rating()/>
        </div>
    }
}

pub fn icon_for_speed(speed: &GameSpeed) -> Icon {
    match speed {
        GameSpeed::Untimed => icondata::BiInfiniteRegular,
        GameSpeed::Blitz => icondata::BsLightningFill,
        GameSpeed::Bullet => icondata::FaGunSolid,
        GameSpeed::Rapid => icondata::LuRabbit,
        GameSpeed::Classic => icondata::LuTurtle,
        GameSpeed::Correspondence => icondata::AiMailOutlined,
        GameSpeed::Puzzle => icondata::TiPuzzle,
    }
}
