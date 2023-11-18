use crate::common::{piece_type::PieceType, svg_pos::SvgPos};
use crate::providers::game_state::GameStateSignal;
use hive_lib::{bug::Bug, game_status::GameStatus, piece::Piece, position::Position};
use leptos::logging::log;
use leptos::*;

#[component]
pub fn Piece(
    // WARN piece and position are untracked and might break reactivity if passed in as signals in the future
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
) -> impl IntoView {
    let center = move || SvgPos::center_for_level(position.get_untracked(), level());
    let transform = move || format!("translate({},{})", center().0, center().1);
    let bug = piece.get_untracked().bug();
    let color = piece.get_untracked().color();
    let order = piece.get_untracked().order();
    //IMPORTANT drop-shadow-b drop-shadow-w leave this comment for TW
    let mut filter = String::from("drop-shadow-");
    filter.push_str(&color.to_string());
    if piece_type == PieceType::Inactive {
        filter.push_str(" sepia");
    }

    let mut dot_color = String::from(" color: #");
    dot_color.push_str(match bug {
        Bug::Ant => "3574a5",
        Bug::Beetle => "7a4fab",
        Bug::Grasshopper => "3f9b3a",
        Bug::Spider => "993c1e",
        _ => "FF0000",
    });

    let mut game_state_signal = expect_context::<GameStateSignal>();

    let onclick = move |_| match (game_state_signal.signal)().state.game_status {
        GameStatus::Finished(_) => { /* Don't attach any on:click for finished games */ }
        _ => {
            match piece_type {
                PieceType::Board => {
                    log!("Board piece");
                    game_state_signal.show_moves(piece.get_untracked(), position.get_untracked());
                }
                PieceType::Reserve => {
                    log!("Reserve piece");
                    game_state_signal.show_spawns(piece.get_untracked(), position.get_untracked());
                }
                PieceType::Move => {
                    log!("Moving piece {}", piece.get_untracked());
                    game_state_signal.move_active();
                }
                PieceType::Spawn => {
                    log!("Spawning piece {}", piece.get_untracked());
                    game_state_signal.spawn_active();
                }
                _ => log!("Piece is {}", piece_type),
            };
        }
    };

    view! {
        <g on:click=onclick class=filter style=dot_color>
            <g transform=transform>
                <use_
                    href=format!("#{}", color)
                    transform="scale(0.56, 0.56) translate(-45, -50)"
                ></use_>
                <use_
                    href=format!("#{}", bug)
                    transform="scale(0.56, 0.56) translate(-50, -45)"
                ></use_>
                <use_
                    href=format!("#{}", order)
                    transform="scale(0.56, 0.56) translate(-45, -50)"
                ></use_>
            </g>
        </g>
    }
}
