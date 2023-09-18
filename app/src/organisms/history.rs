use crate::common::game_state::GameStateSignal;
use leptos::*;

#[component]
pub fn HistoryMove(cx: Scope, turn: usize, piece: String, position: String) -> impl IntoView {
    let game_state_signal = use_context::<RwSignal<GameStateSignal>>(cx)
        .expect("there to be a `GameState` signal provided");

    let onclick = move |_| {
        let mut game_state = game_state_signal.get();
        game_state.show_history_turn(turn);
    };

    view! { cx,
        <div class="ml-3" on:click=onclick>
            {format!("{turn}. {piece} {position}")}
        </div>
    }
}

#[component]
pub fn History(cx: Scope) -> impl IntoView {
    let game_state_signal = use_context::<RwSignal<GameStateSignal>>(cx)
        .expect("there to be a `GameState` signal provided");

    let history_moves = move || {
        let mut his = Vec::new();
        for (i, (piece, pos)) in game_state_signal
            .get()
            .signal
            .get()
            .state
            .history
            .moves
            .into_iter()
            .enumerate()
        {
            his.push((i, piece, pos));
        }
        his
    };

    let next = move |_| {
        log!("Next");
        game_state_signal.get().next_history_turn();
    };

    let previous = move |_| {
        log!("prev");
        game_state_signal.get().previous_history_turn();
    };

    view! { cx,
        <div class="grid grid-cols-2 gap-1">

            <button on:click=previous>
                Previous
            </button>

            <button on:click=next>
                Next
            </button>

            <div class="ml-3 mt-6 mb-3">
                White
            </div>

            <div class="ml-3 mt-6 mb-3">
                Black
            </div>
            <For
                each=history_moves
                key=|a_move| (a_move.0)
                view=move |cx, a_move| {
                    view! { cx, <HistoryMove turn=a_move.0 piece=a_move.1 position=a_move.2/> }
                }
            />

        </div>
    }
}
