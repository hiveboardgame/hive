use crate::common::{Hex, HexStack, HexType, PieceType};

use crate::components::{atoms::svgs::Svgs, molecules::hex_stack::HexStack};
use crate::providers::game_state::{GameStateSignal, View};
use hive_lib::History;
use hive_lib::{Bug, BugStack, Color, GameStatus, Piece, Position, State};
use leptos::*;
use std::str::FromStr;

fn piece_active(state: &State, viewing: &View, piece: &Piece, is_last_turn: bool) -> bool {
    //viewing history
    if viewing == &View::History && !is_last_turn {
        return false;
    }
    // game is over
    if let GameStatus::Finished(_) = state.game_status {
        return false;
    }
    // #TODO make this come from global state
    if !piece.is_color(state.turn_color) {
        return false;
    };
    // first and second turn
    // -> disable queen
    if state.tournament && piece.bug() == Bug::Queen && state.turn < 2 {
        return false;
    };
    // if queen_required
    // -> disable all but queen
    if state.board.queen_required(state.turn, state.turn_color) && piece.bug() != Bug::Queen {
        return false;
    };
    true
}

#[derive(PartialEq, Eq, Debug)]
pub enum Alignment {
    SingleRow,
    DoubleRow,
}

#[component]
pub fn Reserve(
    #[prop(into)] color: MaybeSignal<Color>,
    alignment: Alignment,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();

    let (viewbox_str, viewbox_styles) = match alignment {
        Alignment::SingleRow => ("-40 -55 450 100", "inline max-h-[inherit] h-full w-fit"),
        Alignment::DoubleRow => ("-32 -55 250 180", "p-1"),
    };

    let stacked_pieces = move || {
        let game_state = (game_state_signal.signal)();
        let reserve = match game_state.view {
            View::Game => game_state
                .state
                .board
                .reserve(color(), game_state.state.game_type),
            View::History => {
                let mut history = History::new();
                if let Some(turn) = game_state.history_turn {
                    history.moves = game_state.state.history.moves[0..=turn].into();
                }
                let state = State::new_from_history(&history).expect("Got state from history");
                state.board.reserve(color(), game_state.state.game_type)
            }
        };
        let mut clicked_position = None;
        if color() == game_state.state.turn_color {
            clicked_position = game_state.reserve_position;
        }
        let mut seen = -1;
        let mut res = Vec::new();
        for bug in Bug::all().into_iter() {
            if let Some(piece_strings) = reserve.get(&bug) {
                seen += 1;
                let position = if alignment == Alignment::SingleRow {
                    Position::new(seen, 0)
                } else {
                    Position::new(seen % 4, seen / 4)
                };
                let bs = BugStack::new();
                let mut hs = HexStack::new(&bs, position);
                let stack_height = piece_strings.len() - 1;
                for (i, piece_str) in piece_strings.iter().rev().enumerate() {
                    let piece = Piece::from_str(piece_str).expect("Parsed piece");
                    let piece_type = if piece_active(
                        &game_state.state,
                        &game_state.view,
                        &piece,
                        game_state.is_last_turn(),
                    ) {
                        if i == stack_height {
                            PieceType::Reserve
                        } else {
                            PieceType::Nope
                        }
                    } else {
                        PieceType::Inactive
                    };
                    hs.hexes.push(Hex {
                        kind: HexType::Tile(piece, piece_type),
                        position,
                        level: i,
                    });
                }
                if let Some(click) = clicked_position {
                    if click == position {
                        if game_state.target_position.is_some() {
                            hs.add_active(true);
                        } else {
                            hs.add_active(false);
                        }
                    }
                }
                res.push(hs);
            } else if alignment == Alignment::DoubleRow {
                seen += 1;
            }
        }
        res
    };

    let pieces_view = move || {
        stacked_pieces()
            .into_iter()
            .map(|hex_stack| {
                view! { <HexStack hex_stack=hex_stack/> }
            })
            .collect_view()
    };

    view! {
        <svg
            width="100%"
            height="100%"
            class=format!("duration-300 {viewbox_styles} {extend_tw_classes}")
            viewBox=viewbox_str
            xmlns="http://www.w3.org/2000/svg"
        >
            <Svgs/>
            {pieces_view}
        </svg>
    }
}
