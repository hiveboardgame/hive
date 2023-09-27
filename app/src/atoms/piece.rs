use crate::common::{game_state::GameStateSignal, piece_type::PieceType, svg_pos::SvgPos};
use hive_lib::{bug::Bug, piece::Piece, position::Position};
use leptos::logging::log;
use leptos::*;

#[component]
pub fn Piece(
    #[prop(into)] piece: MaybeSignal<Piece>,
    #[prop(into)] position: MaybeSignal<Position>,
    #[prop(into)] level: MaybeSignal<usize>,
    #[prop(optional, into)] piece_type: PieceType,
) -> impl IntoView {
    let center = SvgPos::center_for_level(position.get(), level.get());
    let transform = format!("translate({},{})", center.0, center.1);
    // drop-shadow-b drop-shadow-w leave this comment for TW
    let mut filter = String::from("drop-shadow-");
    filter.push_str(&piece.get().color().to_string());
    if piece_type == PieceType::Inactive {
        filter.push_str(" sepia");
    }
    let color = piece.get().color().to_string();
    let bug = piece.get().bug().to_string();
    let order = piece.get().order().to_string();

    let mut dot_color = String::from(" color: #");
    dot_color.push_str(match piece.get().bug() {
        Bug::Ant => "3574a5",
        Bug::Beetle => "7a4fab",
        Bug::Grasshopper => "3f9b3a",
        Bug::Spider => "993c1e",
        _ => "FF0000",
    });

    let game_state_signal = use_context::<RwSignal<GameStateSignal>>()
        .expect("there to be a `GameState` signal provided");

    let onclick = move |_| {
        let mut game_state = game_state_signal.get();
        match piece_type {
            PieceType::Board => {
                log!("Board piece");
                game_state.show_moves(piece.get(), position.get());
            }
            PieceType::Reserve => {
                log!("Reserve piece");
                game_state.show_spawns(piece.get(), position.get());
            }
            PieceType::Spawn => {
                log!("Spawning piece {}", piece.get());
                game_state.play_active_piece();
            }
            _ => log!("Piece is {}", piece_type),
        }
    };

    view! {
        <g on:click=onclick class=filter style=dot_color>
            <g transform=format!("{}", transform)>
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
