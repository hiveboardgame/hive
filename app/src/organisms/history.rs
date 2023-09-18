use crate::common::game_state::GameStateSignal;
use leptos::*;

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
            .iter()
            .enumerate()
        {
            if i == 0 {
                his.push(format!("{}. {piece} ", i + 1));
            } else {
                his.push(format!("{}. {piece} {pos} ", i + 1));
            }
        }
        his
    };

    let onclick = move |_| log!("Show move");

    view! { cx,
        <div class="grid grid-cols-2 gap-1">
            <div class="ml-3 mt-6 mb-3">
                White
            </div>
            <div class="ml-3 mt-6 mb-3">
                Black
            </div>
            <For
                each=history_moves
                key=|a_move| (a_move.to_string())
                view=move |cx, a_move| {
                    view! { cx,
                        <div class="ml-3" on:click=onclick>
                            {a_move}
                        </div>
                    }
                }
            />

        </div>
    }
}
