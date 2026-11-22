use crate::{
    common::{StackNumberStyle, SvgPos},
    components::atoms::render_layer_view::{display_level, RenderLayerView},
    hiveground::{
        ExpandedStackLevel,
        HivegroundInteraction,
        HivegroundPaint,
        PieceShadow,
        RenderLayer,
        RenderLayerKind,
    },
    providers::Config,
};
use hive_lib::{Color, Piece, Position};
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

pub const BADGE_OFFSET_X: f32 = 21.0;
pub const BADGE_OFFSET_Y: f32 = -19.0;
pub const BADGE_HALF_EXTENT: f32 = 10.0;
const BADGE_CIRCLE_RADIUS: &str = "8";
const BADGE_HEX_POINTS: &str = "0,-10 8.7,-5 8.7,5 0,10 -8.7,5 -8.7,-5";
const BADGE_STROKE_WIDTH: &str = "0.8";
const BADGE_FONT_SIZE: &str = "11";
const BADGE_SHADOW: &str = "filter: drop-shadow(0 1px 1px rgba(40, 32, 20, 0.35))";

struct BadgePalette {
    fill: &'static str,
    stroke: &'static str,
    text: &'static str,
}

fn badge_palette(color: Color) -> BadgePalette {
    match color {
        Color::White => BadgePalette {
            fill: "#2e2a22",
            stroke: "#d9d0b4",
            text: "#f3edd9",
        },
        Color::Black => BadgePalette {
            fill: "#f3edd9",
            stroke: "#8a7f66",
            text: "#3a342a",
        },
    }
}

struct NumberedPiece {
    number: usize,
    base_level: usize,
    expanded_stack_level: ExpandedStackLevel,
    piece: Piece,
    shadow: PieceShadow,
}

fn numbered_pieces(layers: &[RenderLayer], expanded: bool) -> Vec<NumberedPiece> {
    let mut pieces = layers
        .iter()
        .filter_map(|layer| match &layer.kind {
            RenderLayerKind::Piece {
                piece,
                shadow,
                piece_type,
            } if piece_type.counts_towards_stack_height() => Some((layer, *piece, *shadow)),
            _ => None,
        })
        .collect::<Vec<_>>();
    pieces.sort_by_key(|(layer, ..)| layer.level);
    let count = pieces.len();
    if count < 2 {
        return Vec::new();
    }
    let skip = if expanded { 0 } else { count - 1 };
    pieces
        .into_iter()
        .enumerate()
        .skip(skip)
        .map(|(index, (layer, piece, shadow))| NumberedPiece {
            number: if expanded { index + 1 } else { count },
            base_level: layer.base_level(),
            expanded_stack_level: layer.expanded_stack_level(),
            piece,
            shadow,
        })
        .collect()
}

#[component]
pub fn StackCountBadge(
    position: Position,
    layers: Signal<Vec<RenderLayer>>,
    paint: Memo<HivegroundPaint>,
    interaction: HivegroundInteraction,
    badge_scale: f32,
) -> impl IntoView {
    let Config(config, _) = expect_context();
    view! {
        {move || {
            let style = config.with(|config| config.stack_number_style);
            let expanded = interaction.stack_level_multiplier(position) > 1;
            layers
                .with(|layers| numbered_pieces(layers, expanded))
                .into_iter()
                .map(|numbered| {
                    let level = display_level(
                        numbered.base_level,
                        numbered.expanded_stack_level,
                        interaction.stack_level_multiplier(position),
                    );
                    let three_d = paint
                        .with(|paint| paint.piece(numbered.piece, numbered.shadow).three_d);
                    let (x, y) = SvgPos::center_for_level(position, level, three_d);
                    let palette = badge_palette(numbered.piece.color());
                    view! {
                        <g
                            class="pointer-events-none"
                            aria-hidden="true"
                            transform=format!(
                                "translate({},{}) scale({badge_scale})",
                                x + BADGE_OFFSET_X,
                                y + BADGE_OFFSET_Y,
                            )
                            style=BADGE_SHADOW
                        >
                            {match style {
                                StackNumberStyle::Hexes => {
                                    view! {
                                        <polygon
                                            points=BADGE_HEX_POINTS
                                            fill=palette.fill
                                            stroke=palette.stroke
                                            stroke-width=BADGE_STROKE_WIDTH
                                            stroke-linejoin="round"
                                        />
                                    }
                                        .into_any()
                                }
                                StackNumberStyle::Circles => {
                                    view! {
                                        <circle
                                            r=BADGE_CIRCLE_RADIUS
                                            fill=palette.fill
                                            stroke=palette.stroke
                                            stroke-width=BADGE_STROKE_WIDTH
                                        />
                                    }
                                        .into_any()
                                }
                            }}
                            <text
                                text-anchor="middle"
                                dominant-baseline="central"
                                font-size=BADGE_FONT_SIZE
                                font-weight="700"
                                fill=palette.text
                            >
                                {numbered.number}
                            </text>
                        </g>
                    }
                })
                .collect_view()
        }}
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
                source: None,
            },
        };
        let reserve_active = RenderLayer {
            level: 1,
            kind: RenderLayerKind::Active {
                state: ActiveMarkerState::Reserve,
                source: None,
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

    #[test]
    fn collapsed_stack_shows_one_badge_with_the_height() {
        let layers = [
            model_piece(PieceType::Covered, 1),
            model_piece(PieceType::Board, 2),
            model_piece(PieceType::Covered, 0),
        ];
        let badges = numbered_pieces(&layers, false);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].number, 3);
        assert_eq!(badges[0].base_level, 2);
    }

    #[test]
    fn expanded_stack_numbers_every_piece_from_the_bottom() {
        let layers = [
            model_piece(PieceType::Board, 2),
            model_piece(PieceType::Covered, 0),
            model_piece(PieceType::Covered, 1),
        ];
        let numbers = numbered_pieces(&layers, true)
            .iter()
            .map(|numbered| (numbered.number, numbered.base_level))
            .collect::<Vec<_>>();
        assert_eq!(numbers, vec![(1, 0), (2, 1), (3, 2)]);
    }

    #[test]
    fn single_pieces_and_overlays_get_no_badge() {
        let target = RenderLayer {
            level: 1,
            kind: RenderLayerKind::Target,
        };
        assert!(numbered_pieces(&[model_piece(PieceType::Board, 0)], false).is_empty());
        assert!(numbered_pieces(&[model_piece(PieceType::Board, 0), target], true).is_empty());
    }

    #[test]
    fn move_previews_count_towards_the_height() {
        let layers = [
            model_piece(PieceType::Covered, 0),
            model_piece(PieceType::Board, 1),
            model_piece(PieceType::Move, 2),
        ];
        assert_eq!(numbered_pieces(&layers, false)[0].number, 3);
    }

    #[test]
    fn premove_ghosts_do_not_count_towards_the_height() {
        let layers = [
            model_piece(PieceType::Board, 0),
            model_piece(PieceType::Premove, 1),
        ];
        assert!(numbered_pieces(&layers, false).is_empty());
    }

    #[test]
    fn spawn_ghosts_do_not_count_towards_the_height() {
        let layers = [
            model_piece(PieceType::Board, 0),
            model_piece(PieceType::Spawn, 1),
        ];
        assert!(numbered_pieces(&layers, false).is_empty());
        assert!(numbered_pieces(&layers, true).is_empty());
    }

    #[test]
    fn reserve_and_inactive_pieces_count_towards_the_height() {
        let layers = [
            model_piece(PieceType::Inactive, 0),
            model_piece(PieceType::Reserve, 1),
        ];
        assert_eq!(numbered_pieces(&layers, false)[0].number, 2);
    }
}
