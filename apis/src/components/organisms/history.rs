use crate::components::atoms::history_button::set_timer_from_response;
use crate::components::molecules::history_controls::HistoryControls;
use crate::components::organisms::side_board::move_query_signal;
use crate::providers::game_state::{self, GameStateSignal};
use crate::providers::timer::TimerSignal;
use hive_lib::GameStatus;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{PrettyString, TimeMode};

static NANOS_IN_SECOND: u64 = 1000000000_u64;

#[component]
pub fn HistoryMove(
    turn: usize,
    piece: String,
    position: String,
    repetition: bool,
    parent_div: NodeRef<html::Div>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let div_ref = NodeRef::<html::Div>::new();
    let timer = expect_context::<TimerSignal>();
    let (_move, set_move) = move_query_signal();
    let onclick = move |_| {
        game_state.show_history_turn(turn);
        set_move.set(
            game_state
                .signal
                .with_untracked(|gs| gs.history_turn.map(|v| v + 1)),
        );
        set_timer_from_response(game_state, timer);
    };
    let history_turn = create_read_slice(game_state.signal, |gs| gs.history_turn);
    let is_realtime = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .is_some_and(|gr| gr.time_mode == TimeMode::RealTime)
    });
    let get_class = move || {
        let base_class = "col-span-2 p-1 h-auto max-h-6 leading-6 transition-transform duration-300 transform odd:ml-1 odd:justify-self-start even:mr-1 even:justify-self-end hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";
        if let Some(history_turn) = history_turn() {
            if turn == history_turn {
                return format!("{base_class} bg-orange-twilight");
            }
        }
        base_class.to_string()
    };
    div_ref.on_load(move |elem| {
        if let Some(parent_div) = parent_div.get_untracked() {
            parent_div.set_scroll_top(parent_div.scroll_height())
        }
        if history_turn().is_some_and(|t| t == turn) {
            elem.scroll_into_view_with_bool(false);
        }
    });
    let rep = if repetition {
        String::from(" ↺")
    } else {
        String::new()
    };
    let time_took = move || {
        if turn < 2 {
            return None;
        }
        let response = game_state.signal.with(|gs| gs.game_response.clone())?;
        let increment = response.time_increment? as i64 * NANOS_IN_SECOND as i64;
        let time_left = (*response.move_times.get(turn)?)?;
        let prev_time = (*response.move_times.get(turn - 2)?)?;
        let seconds = ((prev_time + increment - time_left) as f64) / (NANOS_IN_SECOND as f64);
        if seconds > 60.0 {
            Some(format!(" ({:.1} m)", seconds / 60.0))
        } else {
            Some(format!(" ({seconds:.2} s)"))
        }
    };
    view! {
        <div node_ref=div_ref class=get_class on:click=onclick>
            {format!("{}. {piece} {position}{}", turn + 1, rep)}
            <Show when=is_realtime>{time_took}</Show>
        </div>
    }
}

#[component]
pub fn History(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state = expect_context::<game_state::GameStateSignal>();
    let params = use_params_map();
    let queries = use_query_map();
    let state = create_read_slice(game_state.signal, |gs| gs.state.clone());
    let repetitions = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().map(|gr| gr.repetitions.clone())
    });
    let history_moves = move || {
        state()
            .history
            .moves
            .into_iter()
            .enumerate()
            .map(|(i, (piece, pos))| (i, piece, pos))
            .collect::<Vec<(usize, String, String)>>()
    };

    let parent = NodeRef::<html::Div>::new();
    let game_result = move || match state().game_status {
        GameStatus::Finished(result) => result.to_string(),
        _ => "".to_string(),
    };

    let conclusion = create_read_slice(game_state.signal, |gs| {
        if let Some(game) = &gs.game_response {
            game.conclusion.pretty_string()
        } else {
            String::from("No data")
        }
    });

    let is_repetition = move |turn: usize| {
        if let Some(repetitions) = repetitions() {
            repetitions.contains(&turn)
        } else {
            false
        }
    };

    let analysis_url = move || {
        if let Some(nanoid) = params.get().get("nanoid") {
            let mut url = format!("/analysis/{nanoid}");

            if let Some(move_param) = queries.get().get("move") {
                url = format!("{url}?move={move_param}");
            }

            url
        } else {
            "/analysis".to_string()
        }
    };
    view! {
        <div class=format!("h-full flex flex-col pb-4 {extend_tw_classes}")>

            <HistoryControls parent=parent.into() />
            <div node_ref=parent class="grid overflow-auto grid-cols-4 gap-1 mb-8 max-h-full h-fit">
                <For each=history_moves key=|history_move| history_move.0 let:history_move>

                    <HistoryMove
                        turn=history_move.0
                        piece=history_move.1
                        position=history_move.2
                        parent_div=parent
                        repetition=is_repetition(history_move.0)
                    />
                </For>

                <Show when=game_state.is_finished()>
                    <div class="col-span-4 text-center">{game_result}</div>
                    <div class="col-span-4 text-center">{conclusion}</div>
                    <a
                        href=analysis_url
                        class="col-span-4 place-self-center w-4/5 text-white rounded duration-300 no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
                    >
                        <div class="flex gap-1 justify-center items-center">
                            <Icon icon=icondata::TbMicroscope attr:class="py-1 w-7 h-7" />
                            "Analyze here"
                        </div>
                    </a>
                </Show>
            </div>
        </div>
    }
}
