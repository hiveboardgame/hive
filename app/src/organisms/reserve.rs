use std::str::FromStr;

use crate::{atoms::svgs::Svgs, common::game_state::GameState};
use crate::common::piece_type::PieceType;
use crate::molecules::piece_stack::PieceStack;
use hive_lib::{
    bug::Bug, color::Color, piece::Piece, position::Position, state::State,
};
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
    let game_state = use_context::<RwSignal<GameState>>(cx)
        .expect("there to be a `GameState` signal provided");
    let a_store = move || game_state.get();
    let state = move || a_store().state.get();
    let reserve = state().board.reserve(color, state().game_type);
    // let len = reserve.iter().fold(0, |acc, (_, bugs)| acc + bugs.len());
    let mut seen = -1;
    let pieces = Bug::all().filter_map(|bug| {
        if let Some(piece_strings) = reserve.get(&bug) {
            seen += 1;
            piece_strings
                .iter()
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
    });

    let pieces_view = pieces
        .map(|v| {
            view! {cx, <PieceStack pieces=v/>}
        })
        .collect_view(cx);

    view! { cx,
        <svg viewBox="180 110 100 100" style ="flex: 0 0 10%" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            { pieces_view }
            //<LastMove/>
        </svg>
    }
}
