use crate::{
    common::{PiecePaint, PieceType, SvgPos},
    hiveground::HivegroundInteraction,
};
use hudsoni::{Piece, Position};
use leptos::prelude::*;

#[component]
pub fn PieceGlyph(
    position: Position,
    level: Signal<usize>,
    paint: Memo<PiecePaint>,
) -> impl IntoView {
    let three_d = move || paint.with(|paint| paint.three_d);
    let center = move || SvgPos::center_for_level(position, level(), three_d());
    let ds_transform = move || format!("translate({},{})", center().0, center().1);
    let position_transform = move || format!("translate({},{})", center().0, center().1);
    let rotate_transform = move || {
        paint.with(|paint| {
            paint
                .rotation
                .map(|degrees| format!("rotate({degrees})"))
                .unwrap_or_default()
        })
    };

    let dot_color = move || paint.with(|paint| format!("color: {}", paint.dot_color));
    let show_ds = move || paint.with(|paint| paint.shadow_href.href());
    let dots = move || {
        paint.with(|paint| {
            paint
                .dots_href
                .as_ref()
                .map(|href| href.0.clone())
                .unwrap_or_default()
        })
    };
    let bug_svg = move || paint.with(|paint| paint.bug_href.0.clone());
    let tile_svg = move || paint.with(|paint| paint.tile_href.0.clone());

    view! {
        <g>
            <g transform=ds_transform>
                <g transform="scale(0.56, 0.56) translate(-67, -64.5)">
                    <use_ href=show_ds></use_>
                </g>
            </g>

            <g transform=position_transform>
                <g transform="scale(0.56, 0.56) translate(-45, -50)" style=dot_color>
                    <use_ href=tile_svg></use_>
                </g>
            </g>

            <g transform=position_transform>
                <g transform=rotate_transform>
                    <g transform="scale(0.56, 0.56) translate(-45, -50)" style=dot_color>
                        <use_ href=bug_svg></use_>
                        <use_ href=dots fill="currentcolor"></use_>
                    </g>
                </g>
            </g>
        </g>
    }
}

#[component]
pub fn Piece(
    piece: Piece,
    position: Position,
    level: Signal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
    paint: Memo<PiecePaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    view! {
        <g on:click=move |evt| interaction.click_piece(evt, piece, position, piece_type)>
            <PieceGlyph position level paint />
        </g>
    }
}
