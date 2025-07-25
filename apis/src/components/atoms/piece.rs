use crate::common::{MoveConfirm, TileDesign, TileDots, TileRotation};
use crate::common::{PieceType, SvgPos};
use crate::pages::play::CurrentConfirm;
use crate::providers::analysis::AnalysisSignal;
use crate::providers::config::TileOptions;
use crate::providers::game_state::GameStateSignal;
use crate::providers::{ApiRequestsProvider, AuthContext, Config};
use hive_lib::{Bug, Color, Piece, Position};
use leptos::either::Either;
use leptos::prelude::*;
use web_sys::MouseEvent;

#[component]
pub fn PieceWithoutOnClick(
    #[prop(into)] piece: Signal<Piece>,
    #[prop(into)] position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(into)] tile_opts: TileOptions,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let piece = piece.get_untracked();
    let tile_opts = StoredValue::new(tile_opts);
    let three_d = move || tile_opts.read_value().is_three_d();
    let center = move || SvgPos::center_for_level(position, level(), three_d());
    let order = piece.order();
    let ds_transform = move || format!("translate({},{})", center().0, center().1);
    let position_transform = move || format!("translate({},{})", center().0, center().1,);
    let rotate_transform = move || {
        if tile_opts.with_value(|t| t.rotation.clone()) == TileRotation::No {
            String::new()
        } else {
            format!("rotate({})", order.saturating_sub(1) * 60)
        }
    };

    let bug = piece.bug();
    let color = piece.color();

    let dot_color = move || match tile_opts.with_value(|t| t.design.clone()) {
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
        TileDesign::Pride => match color {
            // For white tiles (#ead9b6), use dark color for dots
            Color::White => "color: #3a3a3a",
            // For black tiles (#3a3a3a), use light color for dots
            Color::Black => "color: #ead9b6",
        },
        TileDesign::Carbon3D => match color {
            // For white tiles (white background), use dark gray for dots (matches converted white bugs)
            Color::White => "color: #2d2d2d",
            // For black tiles (dark gray background), use light gray for dots (matches converted black bugs)
            Color::Black => "color: #dcdcdc",
        },
        TileDesign::Carbon => match color {
            // For white tiles (white background), use dark gray for dots (matches converted white bugs)
            Color::White => "color: #2d2d2d",
            // For black tiles (dark gray background), use light gray for dots (matches converted black bugs)
            Color::Black => "color: #dcdcdc",
        },
    };

    let active_piece = create_read_slice(game_state.signal, |gs| gs.move_info.active);
    let last_move = create_read_slice(game_state.signal, |gs| gs.state.board.last_move);
    let board_state = create_read_slice(game_state.signal, |gs| gs.state.board.clone());
    let top_piece = move || board_state.with(|board| board.top_piece(position).unwrap_or(piece));
    let show_ds = move || {
        let top_piece = top_piece();
        let shadow = match tile_opts.with_value(|t| t.design.clone()) {
            TileDesign::ThreeD | TileDesign::Carbon3D => "/assets/tiles/3d/shadow.svg#dshadow",
            TileDesign::Community
            | TileDesign::HighContrast
            | TileDesign::Official
            | TileDesign::Flat
            | TileDesign::Pride
            | TileDesign::Carbon => "/assets/tiles/common/all.svg#drop_shadow",
        };
        if let Some((active, _)) = active_piece() {
            if active == piece {
                return "#no_ds";
            }
            return shadow;
        };
        if match last_move() {
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

    let dots = move || match tile_opts.with_value(|t| t.dots.clone()) {
        TileDots::No => String::new(),
        TileDots::Angled => format!("/assets/tiles/common/all.svg#a{order}"),
        TileDots::Vertical => format!("/assets/tiles/common/all.svg#v{order}"),
    };

    let bug_svg = move || match tile_opts.with_value(|t| t.design.clone()) {
        TileDesign::Official => format!("/assets/tiles/official/official.svg#{}", bug.name()),
        TileDesign::Flat => format!("/assets/tiles/flat/flat.svg#{}", bug.name()),
        TileDesign::ThreeD => format!("/assets/tiles/3d/3d.svg#{}{}", color.name(), bug.name()),
        TileDesign::HighContrast => format!(
            "/assets/tiles/high-contrast/high-contrast.svg#{}",
            bug.name()
        ),
        TileDesign::Community => format!("/assets/tiles/community/community.svg#{}", bug.name()),
        TileDesign::Pride => format!(
            "/assets/tiles/lgbtq/lgbtq.svg#{}{}",
            color.name(),
            bug.name()
        ),
        TileDesign::Carbon3D => format!(
            "/assets/tiles/carbon-3d/carbon-3d.svg#{}{}",
            color.name(),
            bug.name()
        ),
        TileDesign::Carbon => format!(
            "/assets/tiles/carbon/carbon.svg#{}{}",
            color.name(),
            bug.name()
        ),
    };

    let tile_svg = move || match tile_opts.with_value(|t| t.design.clone()) {
        TileDesign::Official => format!("/assets/tiles/official/official.svg#{}", color.name()),
        TileDesign::Flat => format!("/assets/tiles/flat/flat.svg#{}", color.name()),
        TileDesign::ThreeD => format!("/assets/tiles/3d/3d.svg#{}", color.name()),
        TileDesign::HighContrast => format!(
            "/assets/tiles/high-contrast/high-contrast.svg#{}",
            color.name(),
        ),
        TileDesign::Community => format!("/assets/tiles/community/community.svg#{}", color.name(),),
        TileDesign::Pride => format!("/assets/tiles/lgbtq/lgbtq.svg#{}", color.name()),
        TileDesign::Carbon3D => format!("/assets/tiles/carbon-3d/carbon-3d.svg#{}", color.name()),
        TileDesign::Carbon => format!("/assets/tiles/carbon/carbon.svg#{}", color.name()),
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
    #[prop(into)] piece: Signal<Piece>,
    #[prop(into)] position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
    tile_opts: TileOptions,
) -> impl IntoView {
    let analysis = use_context::<AnalysisSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let onclick = move |evt: MouseEvent| {
        evt.stop_propagation();
        let piece_value = piece.get_untracked();
        let in_analysis = analysis.is_some();
        let current_turn_color = game_state.signal.with_untracked(|gs| gs.state.turn_color);

        let is_selectable_piece = config.get_untracked().allow_preselect
            && match piece_type {
                PieceType::Board => true,
                PieceType::Inactive | PieceType::Reserve => {
                    !piece_value.is_color(current_turn_color)
                }
                _ => false,
            };
        let is_current_player = game_state.signal.with_untracked(|gs| {
            auth_context
                .user
                .with_untracked(|a| gs.uid_is_player(a.as_ref().map(|u| u.id)))
        });
        if game_state.is_move_allowed(in_analysis) {
            match piece_type {
                PieceType::Board => {
                    game_state.show_moves(piece_value, position);
                }
                PieceType::Reserve => {
                    game_state.show_spawns(piece_value, position);
                }
                PieceType::Move | PieceType::Spawn => {
                    if matches!(current_confirm.get_untracked(), MoveConfirm::Double) {
                        game_state.move_active(None, api.get_untracked());
                    }
                }
                _ => {}
            };
        } else if !in_analysis && is_current_player && is_selectable_piece {
            game_state.signal.update(|v| {
                v.move_info.active = Some((piece_value, piece_type));
                if piece_type == PieceType::Board {
                    v.move_info.current_position = Some(position);
                } else {
                    v.move_info.reserve_position = Some(position);
                }
            })
        }
    };

    view! {
        <g on:click=onclick>
            <PieceWithoutOnClick piece position level tile_opts />
        </g>
    }
}

#[component]
pub fn Piece(
    // WARN piece and position are untracked and might break reactivity if passed in as signals in the future
    #[prop(into)] piece: Signal<Piece>,
    #[prop(into)] position: Position,
    #[prop(into)] level: Signal<usize>,
    #[prop(default = false)] simple: bool,
    #[prop(optional, into)] piece_type: PieceType,
    tile_opts: TileOptions,
) -> impl IntoView {
    if simple {
        Either::Right(view! { <PieceWithoutOnClick piece position level tile_opts /> })
    } else {
        Either::Left(view! { <PieceWithOnClick piece position level tile_opts piece_type /> })
    }
}
