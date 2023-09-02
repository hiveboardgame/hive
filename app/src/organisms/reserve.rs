use std::str::FromStr;

use crate::atoms::svgs::Svgs;
use crate::common::piece_type::PieceType;
use crate::molecules::piece::Piece;
use crate::molecules::piece_stack::PieceStack;
use hive_lib::{
    board::Board, bug::Bug, color::Color, game_type::GameType, piece::Piece, position::Position,
    state::State,
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
    let state = State::new(GameType::MLP, true);
    let reserve = state.board.reserve(color, state.game_type);
    // let len = reserve.iter().fold(0, |acc, (_, bugs)| acc + bugs.len());
    let mut seen = -1;
    let pieces = Bug::all()
        .map(|bug| {
            if let Some(piece_strings) = reserve.get(&bug) {
                seen += 1;
                piece_strings
                    .iter()
                    .map(|piece_str| {
                        let piece = Piece::from_str(piece_str).unwrap();
                        let piecetype = if piece_active(&state, &piece) {
                            PieceType::Reserve
                        } else {
                            PieceType::Inactive
                        };
                        (piece, Position::new(1, 1 * seen), piecetype)
                    })
                    .collect::<Vec<(Piece, Position, PieceType)>>()
            } else {
                Vec::new()
            }
        })
        .collect::<Vec<Vec<(Piece, Position, PieceType)>>>();

    let pieces_view = pieces
        .into_iter()
        .map(|v| {
            view! {cx, <PieceStack pieces=v/>}
        })
        .collect_view(cx);

    view! { cx,
    <svg viewBox="0 0 1072 900" >
    //<svg viewBox="0 0 100 100" width="100vw" height="90vh" xmlns="http://www.w3.org/2000/svg">
            <Svgs/>
            { pieces_view }
            //<LastMove/>
        </svg>
    }
}

//      html! { <svg viewBox={vb}>
//          <Bugs />
//          {
//              for pos_pieces.iter().map(|(pos, piecetype, pieces)| {
//                  html_nested! {
//                      <StackedPieces pieces={pieces.clone()} position={pos.clone()} piecetype={piecetype.clone()} />
//                  }
//              })
//          }
//          </svg>
//      }
