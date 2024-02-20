use crate::common::config_options::{MoveConfirm, TileDesign, TileDots, TileRotation};
use crate::common::{piece_type::PieceType, svg_pos::SvgPos};

use crate::pages::analysis::InAnalysis;
use crate::providers::config::config::Config;
use crate::providers::game_state::GameStateSignal;
use hive_lib::{bug::Bug, piece::Piece, position::Position};
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
    let game_state_signal = expect_context::<GameStateSignal>();
    let center = move || SvgPos::center_for_level(position.get_untracked(), level());
    let order = piece.get_untracked().order();
    let config = expect_context::<Config>();
    let transform = move || {
        if (config.tile_rotation.preferred_tile_rotation)() == TileRotation::Yes {
            format!(
                "translate({},{}) rotate({})",
                center().0,
                center().1,
                order.saturating_sub(1) * 60
            )
        } else {
            format!("translate({},{})", center().0, center().1,)
        }
    };
    let bug = piece.get_untracked().bug();
    let color = piece.get_untracked().color();

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

    //IMPORTANT drop-shadow-b drop-shadow-w leave this comment for TW
    let mut filter = match game_state_signal
        .signal
        .get_untracked()
        .state
        .board
        .last_move
    {
        (Some(_), Some(to)) => {
            if position.get_untracked() == to {
                String::new()
            } else {
                format!(
                    "duration-300 translateZ(0) transform-gpu drop-shadow-{}",
                    &color.to_string()
                )
            }
        }
        (Some(pos), None) => {
            if position.get_untracked() == pos {
                String::new()
            } else {
                format!(
                    "duration-300 translateZ(0) transform-gpu drop-shadow-{}",
                    &color.to_string()
                )
            }
        }
        (None, Some(pos)) => {
            if position.get_untracked() == pos {
                String::new()
            } else {
                format!(
                    "duration-300 translateZ(0) transform-gpu drop-shadow-{}",
                    &color.to_string()
                )
            }
        }
        _ => format!(
            "duration-300 translateZ(0) transform-gpu drop-shadow-{}",
            &color.to_string()
        ),
    };
    if piece_type == PieceType::Inactive {
        filter.push_str(" sepia-[.75]");
    }

    let mut game_state_signal = expect_context::<GameStateSignal>();
    let in_analysis = use_context::<InAnalysis>().unwrap_or(InAnalysis(RwSignal::new(false)));

    let onclick = move |evt: MouseEvent| {
        evt.stop_propagation();
        let in_analysis = in_analysis.0.get_untracked();
        if in_analysis || game_state_signal.is_move_allowed() {
            match piece_type {
                PieceType::Board => {
                    game_state_signal.show_moves(piece.get_untracked(), position.get_untracked());
                }
                PieceType::Reserve => {
                    game_state_signal.show_spawns(piece.get_untracked(), position.get_untracked());
                }
                PieceType::Move | PieceType::Spawn => {
                    if in_analysis
                        || matches!(
                            (config.confirm_mode.preferred_confirm)(),
                            MoveConfirm::Double
                        )
                    {
                        game_state_signal.move_active();
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

    view! {
        <g on:click=onclick class=filter style=dot_color>
            <g transform=transform>
                <use_ href=tile_svg transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
                // TODO: Fix svgs to have the same numbers
                <use_ href=bug_svg transform=bug_transform></use_>
                <use_ href=dots transform="scale(0.56, 0.56) translate(-45, -50)"></use_>
            </g>
        </g>
    }
}
