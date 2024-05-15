use crate::responses::RatingResponse;
use icondata::Icon;
use leptos::*;
use leptos_icons::*;
use shared_types::{Certainty, GameSpeed};

#[component]
pub fn Rating(rating: RatingResponse) -> impl IntoView {
    let certainty_str = match rating.certainty {
        Certainty::Clueless => "?",
        _ => "",
    };
    view! {
        {rating.rating}
        {certainty_str}
    }
}

#[component]
pub fn RatingWithIcon(rating: StoredValue<RatingResponse>) -> impl IntoView {
    view! {
        <div class="flex flex-row gap-1 items-center">
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
