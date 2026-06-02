use crate::{
    components::atoms::render_layer_view::RenderLayerView,
    hiveground::{HivegroundInteraction, HivegroundPaint, RenderLayer},
};
use hive_lib::Position;
use leptos::prelude::*;

#[component]
pub fn HivegroundStack(
    position: Position,
    layers: Signal<Vec<RenderLayer>>,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
) -> impl IntoView {
    let is_expandable = move || stack_is_expandable(layers, interaction);

    view! {
        <g
            data-hg-stack-q=position.q
            data-hg-stack-r=position.r
            data-hg-stack-expandable=move || is_expandable().to_string()
        >
            <For
                each=move || layers()
                key=move |layer| (position, layer.clone())
                children=move |layer| {
                    view! { <RenderLayerView position layer paint interaction /> }
                }
            />
        </g>
    }
}

fn stack_is_expandable(
    layers: Signal<Vec<RenderLayer>>,
    interaction: HivegroundInteraction,
) -> bool {
    interaction.can_inspect_stacks()
        && layers.with(|layers| layers.iter().any(RenderLayer::is_stack_expandable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::PieceType,
        hiveground::{ActiveMarkerState, LastMoveDirection, PieceShadow, RenderLayerKind},
    };
    use hive_lib::Piece;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    fn model_piece(piece_type: PieceType, level: usize) -> RenderLayer {
        let piece = piece("wA2");
        RenderLayer {
            level,
            kind: RenderLayerKind::Piece {
                piece,
                piece_type,
                shadow: PieceShadow::for_piece_type(piece_type),
            },
        }
    }

    #[test]
    fn reserve_and_inactive_model_pieces_are_not_expandable() {
        assert!(!model_piece(PieceType::Reserve, 1).is_stack_expandable());
        assert!(!model_piece(PieceType::Inactive, 1).is_stack_expandable());
        assert!(model_piece(PieceType::History, 1).is_stack_expandable());
    }

    #[test]
    fn model_overlays_expand_only_when_their_level_and_state_require_it() {
        let board_active = RenderLayer {
            level: 0,
            kind: RenderLayerKind::Active {
                state: ActiveMarkerState::Board,
            },
        };
        let reserve_active = RenderLayer {
            level: 1,
            kind: RenderLayerKind::Active {
                state: ActiveMarkerState::Reserve,
            },
        };
        let ground_target = RenderLayer {
            level: 0,
            kind: RenderLayerKind::Target,
        };
        let stack_target = RenderLayer {
            level: 1,
            kind: RenderLayerKind::Target,
        };
        let last_move = RenderLayer {
            level: 1,
            kind: RenderLayerKind::LastMove {
                direction: LastMoveDirection::To,
            },
        };

        assert!(board_active.is_stack_expandable());
        assert!(!reserve_active.is_stack_expandable());
        assert!(!ground_target.is_stack_expandable());
        assert!(stack_target.is_stack_expandable());
        assert!(!last_move.is_stack_expandable());
    }
}
