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
use hive_lib::Position;
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
        RenderLayerKind::Active { state, source } => {
            let active_paint = Memo::new(move |_| paint.with(HivegroundPaint::active));
            EitherOf4::A(
                view! { <Active position level active_state=state source paint=active_paint interaction /> },
            )
        }
        RenderLayerKind::Target => {
            let target_paint = Memo::new(move |_| paint.with(HivegroundPaint::target));
            EitherOf4::B(view! { <Target position level paint=target_paint interaction /> })
        }
        RenderLayerKind::VacateTarget => {
            let target_paint = Memo::new(move |_| paint.with(HivegroundPaint::vacate_target));
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
        RenderLayerKind::Premove { direction } => {
            let premove_paint = Memo::new(move |_| paint.with(|paint| paint.premove(direction)));
            EitherOf4::D(view! { <OverlayGlyph position level paint=premove_paint /> })
        }
    }
}

pub fn display_level(
    level: usize,
    expanded_stack_level: ExpandedStackLevel,
    stack_level_multiplier: usize,
) -> usize {
    match expanded_stack_level {
        ExpandedStackLevel::Fixed => level,
        ExpandedStackLevel::Separated => level * stack_level_multiplier,
        ExpandedStackLevel::Attached => level.saturating_sub(1) * stack_level_multiplier + 1,
    }
}

fn display_level_signal(
    level: usize,
    expanded_stack_level: ExpandedStackLevel,
    interaction: HivegroundInteraction,
    position: Position,
) -> Signal<usize> {
    Signal::derive(move || {
        display_level(
            level,
            expanded_stack_level,
            interaction.stack_level_multiplier(position),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_layers_ignore_the_stack_multiplier() {
        assert_eq!(display_level(2, ExpandedStackLevel::Fixed, 1), 2);
        assert_eq!(display_level(2, ExpandedStackLevel::Fixed, 13), 2);
    }

    #[test]
    fn separated_layers_spread_with_the_stack_multiplier() {
        assert_eq!(display_level(2, ExpandedStackLevel::Separated, 1), 2);
        assert_eq!(display_level(2, ExpandedStackLevel::Separated, 13), 26);
    }

    #[test]
    fn attached_layers_sit_one_level_above_the_spread_stack() {
        assert_eq!(display_level(0, ExpandedStackLevel::Attached, 13), 1);
        assert_eq!(display_level(3, ExpandedStackLevel::Attached, 1), 3);
        assert_eq!(display_level(3, ExpandedStackLevel::Attached, 13), 27);
    }
}
