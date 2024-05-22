use crate::common::{MoveConfirm, TileDesign, TileDots, TileRotation};
use crate::common::{PieceType, SvgPos};
use crate::components::organisms::analysis::AnalysisSignal;
use crate::pages::play::CurrentConfirm;
use crate::providers::game_state::GameStateSignal;
use crate::providers::Config;
use hive_lib::{Bug, Piece, Position};
use leptos::*;
use web_sys::MouseEvent;

#[component]
pub fn Piece(
    // WARN piece and position are untracked and might break reactivity if passed in as signals in the future
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
) -> impl IntoView {
    let piece = piece.get_untracked();
    let position = position.get_untracked();
    let center = move || SvgPos::center_for_level(position, level());
    let order = piece.order();
    let config = expect_context::<Config>();
    let ds_transform = move || format!("translate({},{})", center().0, center().1);
    let transform = move || {
        if (config.tile_rotation.preferred_tile_rotation)() == TileRotation::Yes {
            format!(
                "translate({},{}) rotate({})",
                center().0,
                center().1,
                order.saturating_sub(1) * 60
            )
        } else {
            format!("translate({},{})", center().0, center().1)
        }
    };
    let bug = piece.bug();
    let color = piece.color();

    let dot_color = move || {
        if (config.tile_design.preferred_tile_design)() == TileDesign::Official {
            match bug {
                Bug::Ant => " color: #289ee0",
                Bug::Beetle => " color: #9a7fc7",
                Bug::Grasshopper => " color: #42b23c",
                Bug::Spider => " color: #a4572a",
                _ => " color: #FF0000",
            }
        } else {
            match bug {
                Bug::Ant => " color: #3574a5",
                Bug::Beetle => " color: #7a4fab",
                Bug::Grasshopper => " color: #3f9b3a",
                Bug::Spider => " color: #993c1e",
                _ => " color: #FF0000",
            }
        }
    };

    let sepia = if piece_type == PieceType::Inactive {
        "sepia-[.75]"
    } else {
        ""
    };

    let mut game_state = expect_context::<GameStateSignal>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let analysis = use_context::<AnalysisSignal>()
        .unwrap_or(AnalysisSignal(RwSignal::new(None)))
        .0;
    let onclick = move |evt: MouseEvent| {
        evt.stop_propagation();
        let in_analysis = analysis.get_untracked().is_some();
        if in_analysis || game_state.is_move_allowed() {
            match piece_type {
                PieceType::Board => {
                    game_state.show_moves(piece, position);
                }
                PieceType::Reserve => {
                    game_state.show_spawns(piece, position);
                }
                PieceType::Move | PieceType::Spawn => {
                    if current_confirm() == MoveConfirm::Double {
                        game_state.move_active();
                    }
                }
                _ => {}
            };
        }
    };

    let bug_transform = move || {
        if (config.tile_design.preferred_tile_design)() == TileDesign::Official {
            "scale(0.56, 0.56) translate(-45, -50)"
        } else {
            "scale(0.56, 0.56) translate(-50, -45)"
        }
    };

    let bug_svg = move || {
        if (config.tile_design.preferred_tile_design)() == TileDesign::Official {
            format!("#{}", bug)
        } else {
            format!("#f{}", bug)
        }
    };

    let tile_svg = move || {
        if (config.tile_design.preferred_tile_design)() == TileDesign::Official {
            format!("#{}", color)
        } else {
            format!("#f{}", color)
        }
    };

    let dots = move || {
        if (config.tile_dots.preferred_tile_dots)() == TileDots::Yes {
            if (config.tile_design.preferred_tile_design)() == TileDesign::Official {
                return format!("#{}", order);
            } else {
                return format!("#f{}", order);
            }
        }
        String::new()
    };

    let top_piece = game_state
        .signal
        .get_untracked()
        .state
        .board
        .top_piece(position)
        .unwrap_or(piece);

    let active_piece = create_read_slice(game_state.signal, |gs| gs.move_info.active);
    let show_ds = move || {
        if let Some(active) = active_piece() {
            if active == piece {
                return "#no_ds";
            }
            return "#ds";
        };
        if match game_state.signal.get_untracked().state.board.last_move {
            (Some(_), Some(pos)) => position != pos || piece != top_piece,
            (Some(pos), None) => position != pos || piece != top_piece,
            (None, Some(pos)) => position != pos || piece != top_piece,
            _ => true,
        } {
            "#ds"
        } else {
            "#no_ds"
        }
    };

    view! {
        <g on:click=onclick class=sepia style=dot_color>
            <g transform=ds_transform>
                <use_ href=show_ds transform="scale(0.56, 0.56) translate(-67, -64.5)"></use_>
            </g>
            <g transform=transform>
                <use_ href=tile_svg transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
                <use_ href=bug_svg transform=bug_transform></use_>
                <use_ href=dots transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
            </g>
        </g>
    }
}
