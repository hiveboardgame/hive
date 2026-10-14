use crate::{common::SvgPos, providers::Config};
use hive_lib::Position;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlowColor {
    Red,
    Green,
}

impl GlowColor {
    fn gradient_id(self) -> &'static str {
        match self {
            GlowColor::Red => "piece-glow-red",
            GlowColor::Green => "piece-glow-green",
        }
    }
}

/// Cells to light from underneath, provided by pages that want them (puzzles mark both Queens).
#[derive(Clone, Copy)]
pub struct PieceGlows(pub Signal<Vec<(Position, GlowColor)>>);

/// Soft glows drawn before the pieces, so they show around a piece and in the gaps beside it.
#[component]
pub fn PieceGlowLayer(glows: PieceGlows) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let circles = move || {
        let straight = config.with(|config| config.tile.is_three_d());
        glows
            .0
            .get()
            .into_iter()
            .map(|(position, color)| {
                let (cx, cy) = SvgPos::center_for_level(position, 0, straight);
                view! { <circle cx=cx cy=cy r="46" fill=format!("url(#{})", color.gradient_id()) /> }
            })
            .collect_view()
    };
    view! {
        <g pointer-events="none">
            <defs>
                <radialGradient id="piece-glow-red">
                    <stop offset="0.45" stop-color="#d61a35" stop-opacity="0.9" />
                    <stop offset="1" stop-color="#d61a35" stop-opacity="0" />
                </radialGradient>
                <radialGradient id="piece-glow-green">
                    <stop offset="0.45" stop-color="#3cb043" stop-opacity="0.85" />
                    <stop offset="1" stop-color="#3cb043" stop-opacity="0" />
                </radialGradient>
            </defs>
            {circles}
        </g>
    }
}
