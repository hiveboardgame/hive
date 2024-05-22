use crate::common::SvgPos;
use crate::common::{TileDesign, TileDots, TileRotation};
use crate::providers::game_state::GameStateSignal;
use crate::providers::Config;
use hive_lib::{Bug, Piece, Position};
use leptos::*;

#[component]
pub fn SimplePiece(
    // WARN piece and position are untracked and might break reactivity if passed in as signals in the future
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let config = expect_context::<Config>();
    let piece = piece.get_untracked();
    let position = position.get_untracked();
    let center = move || SvgPos::center_for_level(position, level());
    let order = piece.order();
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
        <g style=dot_color>
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
