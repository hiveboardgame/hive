use icondata_core;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{rounded_rating, Certainty, GameSpeed};

#[derive(Clone, Default)]
pub struct RatingValue {
    number: Option<i64>,
}

impl From<f64> for RatingValue {
    fn from(rating: f64) -> Self {
        Self {
            number: rounded_rating(rating).map(i64::from),
        }
    }
}

impl From<u64> for RatingValue {
    fn from(rating: u64) -> Self {
        Self {
            number: i64::try_from(rating).ok(),
        }
    }
}

impl From<i32> for RatingValue {
    fn from(rating: i32) -> Self {
        Self {
            number: Some(i64::from(rating)),
        }
    }
}

impl From<u32> for RatingValue {
    fn from(rating: u32) -> Self {
        Self {
            number: Some(i64::from(rating)),
        }
    }
}

impl<T: Into<RatingValue>> From<Option<T>> for RatingValue {
    fn from(rating: Option<T>) -> Self {
        rating.map(Into::into).unwrap_or_default()
    }
}

#[component]
pub fn Rating(
    #[prop(into)] value: RatingValue,
    #[prop(optional_no_strip, into)] certainty: Option<Certainty>,
) -> impl IntoView {
    let certainty_str = match certainty {
        Some(Certainty::Clueless) => "?",
        Some(Certainty::Provisional) => "~",
        Some(Certainty::Rankable) | None => "",
    };
    view! {
        {value.number.map(|rating| rating.to_string()).unwrap_or_else(|| String::from("—"))}
        {certainty_str}
    }
}

#[component]
pub fn RatingWithIcon(
    #[prop(into)] value: RatingValue,
    speed: GameSpeed,
    #[prop(optional_no_strip, into)] certainty: Option<Certainty>,
) -> impl IntoView {
    view! {
        <div class="flex flex-row gap-1 items-center">
            <Icon icon=icon_for_speed(speed) attr:class="size-4" />
            <Rating value certainty />
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
    }
}
