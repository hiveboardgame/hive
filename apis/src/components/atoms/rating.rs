use crate::responses::RatingResponse;
use icondata_core;
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
            <Icon icon=icon_for_speed(rating.with_value(|r| r.speed)) attr:class="size-4" />
            <Rating rating=rating.get_value() />
        </div>
    }
}

pub fn icon_for_speed(speed: GameSpeed) -> &'static icondata_core::IconData {
    match speed {
        GameSpeed::Untimed => icondata_bi::BiInfiniteRegular,
        GameSpeed::Blitz => icondata_bs::BsLightningFill,
        GameSpeed::Bullet => icondata_fa::FaGunSolid,
        GameSpeed::Rapid => icondata_lu::LuRabbit,
        GameSpeed::Classic => icondata_lu::LuTurtle,
        GameSpeed::Correspondence => icondata_ai::AiMailOutlined,
        GameSpeed::Puzzle => icondata_ti::TiPuzzle,
        GameSpeed::AllSpeeds => icondata_fa::FaAsteriskSolid,
    }
}
