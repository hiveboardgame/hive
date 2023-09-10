use std::str::FromStr;

use crate::common::piece_type::PieceType;
use crate::molecules::piece_stack::PieceStack;
use crate::{atoms::svgs::Svgs, common::game_state::GameState};
use hive_lib::{bug::Bug, color::Color, piece::Piece, position::Position, state::State};
use leptos::*;

fn piece_active(state: &State, piece: &Piece) -> bool {
    // #TODO make this come from global state
    if !piece.is_color(state.turn_color) {
        return false;
    };
    // first and second turn
    // -> disable queen
    if piece.bug() == Bug::Queen && state.turn < 2 {
        return false;
    };
    // if queen_required
    // -> disable all but queen
    if state.board.queen_required(state.turn, state.turn_color) && piece.bug() != Bug::Queen {
        return false;
    };
    true
}

#[component]
pub fn Reserve(cx: Scope, color: Color) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let state = move || game_state.get().state.get();
    let reserve = move || state().board.reserve(color, state().game_type);
    let stacked_pieces = move || {
        let mut seen = -1;
        Bug::all()
            .into_iter()
            .filter_map(|bug| {
                if let Some(piece_strings) = reserve().get(&bug) {
                    seen += 1;
                    piece_strings
                        .iter()
                        .rev()
                        .map(|piece_str| {
                            let piece = Piece::from_str(piece_str).unwrap();
                            let piecetype = if piece_active(&state(), &piece) {
                                PieceType::Reserve
                            } else {
                                PieceType::Inactive
                            };
                            Some((piece, Position::new(4 - seen / 2, seen), piecetype))
                        })
                        .collect::<Option<Vec<(Piece, Position, PieceType)>>>()
                } else {
                    None
                }
            })
            .collect::<Vec<Vec<(Piece, Position, PieceType)>>>()
    };

    let pieces_view = move || stacked_pieces().into_iter().map(|pieces| {
        view! { cx,
            <PieceStack pieces=pieces/>
        }
    }).collect_view(cx);

    view! { cx,
        <svg viewBox="180 110 100 100" style ="flex: 0 0 10%" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            { pieces_view }
        </svg>
    }
}
