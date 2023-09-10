use crate::common::game_state::GameState;
use leptos::*;
use web_sys::MouseEvent;

#[component]
pub fn History(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let history_moves = move || {
        let mut his = Vec::new();
        for (i, (piece, pos)) in game_state().state.get().history.moves.iter().enumerate() {
            if i == 0 {
                his.push(format!("{}. {piece} ", i + 1));
            } else {
                his.push(format!("{}. {piece} {pos} ", i + 1));
            }
        }
        his
    };
    let onclick = move |_:MouseEvent| log!("This will change the display to that move");
    view! { cx,
        <div style="display: flex; flex-direction: column; height: 300px; overflow-y: auto;">
        History:
            <For
                each=history_moves
                key=|a_move| (a_move.to_string())
                view=move |cx, a_move| {
                    view! { cx, <p on:click=onclick>{a_move}</p> }
                }
            />

        </div>
    }
}
