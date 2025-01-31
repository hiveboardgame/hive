use crate::common::{MoveConfirm, TileDesign, TileDots, TileRotation};
use crate::common::{PieceType, SvgPos};
use crate::components::organisms::analysis::AnalysisSignal;
use crate::pages::play::CurrentConfirm;
use crate::providers::game_state::GameStateSignal;
use crate::providers::Config;
use hive_lib::{Bug, GameStatus, Piece, Position};
use leptos::prelude::*;
use web_sys::MouseEvent;

#[component]
pub fn PieceWithoutOnClick(
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let piece = piece.get_untracked();
    let config = expect_context::<Config>().0;
    let position = position.get_untracked();
    let three_d = move || config().tile_design == TileDesign::ThreeD;
    let center = move || SvgPos::center_for_level(position, level(), three_d());
    let order = piece.order();
    let ds_transform = move || format!("translate({},{})", center().0, center().1);
    let position_transform = move || format!("translate({},{})", center().0, center().1,);
    let rotate_transform = move || {
        if config().tile_rotation == TileRotation::No {
            String::new()
        } else {
            format!("rotate({})", order.saturating_sub(1) * 60)
        }
    };

    let bug = piece.bug();
    let color = piece.color();

    let dot_color = move || match config().tile_design {
        TileDesign::Official | TileDesign::ThreeD => match bug {
            Bug::Ant => "color: #289ee0",
            Bug::Beetle => "color: #9a7fc7",
            Bug::Grasshopper => "color: #42b23c",
            Bug::Spider => "color: #a4572a",
            _ => "color: #FF0000",
        },
        TileDesign::Flat => match bug {
            Bug::Ant => "color: #3574a5",
            Bug::Beetle => "color: #7a4fab",
            Bug::Grasshopper => "color: #3f9b3a",
            Bug::Spider => "color: #993c1e",
            _ => "color: #FF0000",
        },
        TileDesign::HighContrast | TileDesign::Community => match bug {
            Bug::Ant => "color: #3574a5",
            Bug::Beetle => "color: #ac5bb3",
            Bug::Grasshopper => "color: #3f9b3a",
            Bug::Spider => "color: #d66138",
            _ => "color: #FF0000",
        },
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
        let shadow = match config().tile_design {
            TileDesign::ThreeD => "/assets/tiles/3d/shadow.svg#dshadow",
            TileDesign::Community
            | TileDesign::HighContrast
            | TileDesign::Official
            | TileDesign::Flat => "/assets/tiles/common/all.svg#drop_shadow",
        };
        if let Some(active) = active_piece() {
            if active == piece {
                return "#no_ds";
            }
            return shadow;
        };
        if match game_state.signal.get_untracked().state.board.last_move {
            (Some(_), Some(pos)) => position != pos || piece != top_piece,
            (Some(pos), None) => position != pos || piece != top_piece,
            (None, Some(pos)) => position != pos || piece != top_piece,
            _ => true,
        } {
            shadow
        } else {
            "/assets/tiles/common/all.svg#no_ds"
        }
    };

    let dots = move || match config().tile_dots {
        TileDots::No => String::new(),
        TileDots::Angled => format!("/assets/tiles/common/all.svg#a{order}"),
        TileDots::Vertical => format!("/assets/tiles/common/all.svg#v{order}"),
    };

    let bug_svg = move || match config().tile_design {
        TileDesign::Official => format!("/assets/tiles/official/official.svg#{}", bug.name()),
        TileDesign::Flat => format!("/assets/tiles/flat/flat.svg#{}", bug.name()),
        TileDesign::ThreeD => format!("/assets/tiles/3d/3d.svg#{}{}", color.name(), bug.name()),
        TileDesign::HighContrast => format!(
            "/assets/tiles/high-contrast/high-contrast.svg#{}",
            bug.name()
        ),
        TileDesign::Community => format!("/assets/tiles/community/community.svg#{}", bug.name()),
    };

    let tile_svg = move || match config().tile_design {
        TileDesign::Official => format!("/assets/tiles/official/official.svg#{}", color.name()),
        TileDesign::Flat => format!("/assets/tiles/flat/flat.svg#{}", color.name()),
        TileDesign::ThreeD => format!("/assets/tiles/3d/3d.svg#{}", color.name()),
        TileDesign::HighContrast => format!(
            "/assets/tiles/high-contrast/high-contrast.svg#{}",
            color.name(),
        ),
        TileDesign::Community => format!("/assets/tiles/community/community.svg#{}", color.name(),),
    };

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
pub fn PieceWithOnClick(
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
) -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    // TODO: FIX ANALYSIS
    //let analysis = use_context::<AnalysisSignal>()
    //    .unwrap_or(AnalysisSignal(RwSignal::new(None)))
    //    .0;
    //
    let sepia = if piece_type == PieceType::Inactive {
        "sepia-[.75]"
    } else {
        ""
    };

    let onclick = move |evt: MouseEvent| {
        evt.stop_propagation();
        // TODO: FIX ANALYSIS
        // let in_analysis = analysis.get_untracked().is_some();
        let in_analysis = false;
        let is_finished = matches!(
            game_state.signal.get_untracked().state.game_status,
            GameStatus::Finished(_)
        );

        if (in_analysis && !is_finished) || game_state.is_move_allowed() {
            match piece_type {
                PieceType::Board => {
                    game_state.show_moves(piece(), position());
                }
                PieceType::Reserve => {
                    game_state.show_spawns(piece(), position());
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

    view! {
        <g on:click=onclick class=sepia>
            <PieceWithoutOnClick piece position level />
        </g>
    }
}

#[component]
pub fn Piece(
    // WARN piece and position are untracked and might break reactivity if passed in as signals in the future
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
    #[prop(optional, default = false)] simple: bool,
    // TODO: hand in tile_design and don't get it all the time from config
) -> impl IntoView {
    if simple {
        return view! { <PieceWithoutOnClick piece position level /> }.into_any();
    }
    view! { <PieceWithOnClick piece position level piece_type /> }.into_any()
}
