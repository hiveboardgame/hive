use crate::{
    common::{PiecePaint, PieceType, SvgPos},
    hiveground::HivegroundInteraction,
    i18n::*,
};
use hive_lib::{Bug, Color, Piece, Position};
use leptos::prelude::*;
use leptos_i18n::I18nContext;

fn accessible_piece_label(
    i18n: I18nContext<Locale, I18nKeys>,
    piece: Piece,
    piece_type: PieceType,
) -> String {
    let order = if piece.order() == 0 {
        String::new()
    } else {
        format!(" {}", piece.order())
    };
    let color = match piece.color() {
        Color::White => t_string!(i18n, common.white),
        Color::Black => t_string!(i18n, common.black),
    };
    let bug = match piece.bug() {
        Bug::Ant => t_string!(i18n, game.board.pieces.ant),
        Bug::Beetle => t_string!(i18n, game.board.pieces.beetle),
        Bug::Grasshopper => t_string!(i18n, game.board.pieces.grasshopper),
        Bug::Ladybug => t_string!(i18n, game.board.pieces.ladybug),
        Bug::Mosquito => t_string!(i18n, game.board.pieces.mosquito),
        Bug::Pillbug => t_string!(i18n, game.board.pieces.pillbug),
        Bug::Queen => t_string!(i18n, game.board.pieces.queen),
        Bug::Spider => t_string!(i18n, game.board.pieces.spider),
    };
    match piece_type {
        PieceType::Board | PieceType::Covered | PieceType::History => t_string!(
            i18n,
            game.board.on_board,
            color = color,
            piece = bug,
            order = order
        )
        .to_string(),
        PieceType::Reserve | PieceType::Inactive => t_string!(
            i18n,
            game.board.in_reserve,
            color = color,
            piece = bug,
            order = order
        )
        .to_string(),
        PieceType::Move | PieceType::Premove | PieceType::Spawn => t_string!(
            i18n,
            game.board.move_preview,
            color = color,
            piece = bug,
            order = order
        )
        .to_string(),
    }
}

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
    let i18n = use_i18n();
    let aria_label = move || accessible_piece_label(i18n, piece, piece_type);
    view! {
        <g
            role="button"
            aria-label=aria_label
            data-position=format!("{}, {}", position.q, position.r)
            data-stack-level=move || level.get().to_string()
            class:opacity-50=piece_type == PieceType::Premove
            on:click=move |evt| interaction.click_piece(evt, piece, position, piece_type)
            on:pointerdown=move |evt| interaction.press_piece(evt, piece, position, piece_type)
        >
            <PieceGlyph position level paint />
        </g>
    }
}
