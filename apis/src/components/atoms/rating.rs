use crate::responses::RatingResponse;
use icondata::Icon;
use leptos::prelude::*;
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
            <Icon icon=icon_for_speed(&rating.get_value().speed) attr:class="w-4 h-4" />
            <Rating rating=rating.get_value() />
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
