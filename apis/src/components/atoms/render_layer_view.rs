use crate::{
    components::atoms::{active::Active, overlay::OverlayGlyph, piece::Piece, target::Target},
    hiveground::{
        ExpandedStackLevel,
        HivegroundInteraction,
        HivegroundPaint,
        RenderLayer,
        RenderLayerKind,
    },
};
use hudsoni::Position;
use leptos::{either::EitherOf4, prelude::*};

#[component]
pub fn RenderLayerView(
    position: Position,
    layer: RenderLayer,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let fallback_level = layer.base_level();
    let expanded_stack_level = layer.expanded_stack_level();
    let level = display_level_signal(fallback_level, expanded_stack_level, interaction, position);

    match layer.kind {
        RenderLayerKind::Active { state } => {
            let active_paint = Memo::new(move |_| paint.with(HivegroundPaint::active));
            EitherOf4::A(
                view! { <Active position level active_state=state paint=active_paint interaction /> },
            )
        }
        RenderLayerKind::Target => {
            let target_paint = Memo::new(move |_| paint.with(HivegroundPaint::target));
            EitherOf4::B(view! { <Target position level paint=target_paint interaction /> })
        }
        RenderLayerKind::Piece {
            piece,
            piece_type,
            shadow,
        } => {
            let piece_paint = Memo::new(move |_| paint.with(|paint| paint.piece(piece, shadow)));
            EitherOf4::C(
                view! { <Piece piece position level piece_type paint=piece_paint interaction /> },
            )
        }
        RenderLayerKind::LastMove { direction } => {
            let last_move_paint =
                Memo::new(move |_| paint.with(|paint| paint.last_move(direction)));
            EitherOf4::D(view! { <OverlayGlyph position level paint=last_move_paint /> })
        }
    }
}

fn display_level_signal(
    level: usize,
    expanded_stack_level: ExpandedStackLevel,
    interaction: HivegroundInteraction,
    position: Position,
) -> Signal<usize> {
    Signal::derive(move || match expanded_stack_level {
        ExpandedStackLevel::Fixed => level,
        ExpandedStackLevel::Separated => level * interaction.stack_level_multiplier(position),
        ExpandedStackLevel::Attached => {
            level.saturating_sub(1) * interaction.stack_level_multiplier(position) + 1
        }
    })
}
