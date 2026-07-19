use crate::{
    components::{
        molecules::history_controls::HistoryControls,
        organisms::side_board::move_query_signal,
    },
    hiveground::HivegroundInteraction,
    providers::game_state::{BoardView, GameStateStore, GameStateStoreFields},
};
use hive_lib::{GameStatus, State};
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use shared_types::{PrettyString, TimeMode};
use std::time::Duration;

#[component]
pub fn HistoryMove(
    turn: usize,
    piece: String,
    position: String,
    repetition: bool,
    parent_div: NodeRef<html::Div>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let div_ref = NodeRef::<html::Div>::new();
    let (_move, set_move) = move_query_signal();
    let onclick = move |_| {
        game_state.show_history_turn(turn);
        set_move.set(Some(turn + 1));
    };
    let board_view = game_state.board_view();
    let state = game_state.state();
    let game_response = game_state.game_response();
    let is_realtime = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|game| game.time_mode == TimeMode::RealTime)
        })
    });
    let is_current = Signal::derive(move || match board_view.get() {
        BoardView::Live => state.with(|state| state.turn.checked_sub(1) == Some(turn)),
        BoardView::History {
            turn: Some(history_turn),
        } => turn == history_turn,
        BoardView::History { turn: None } => false,
    });
    let get_class = move || {
        let base_class = "col-span-2 p-1 h-auto max-h-6 leading-6 transition-transform duration-300 transform odd:ml-1 odd:justify-self-start even:mr-1 even:justify-self-end hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95";
        if is_current.get() {
            return format!("{base_class} bg-orange-twilight");
        }
        base_class.to_string()
    };
    div_ref.on_load(move |elem| {
        if let Some(parent_div) = parent_div.get_untracked() {
            parent_div.set_scroll_top(parent_div.scroll_height())
        }
        if is_current.get_untracked() {
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
        game_response.with(|game_response| {
            let response = game_response.as_ref()?;
            let increment = Duration::from_secs(u64::try_from(response.time_increment?).ok()?);
            let time_left = response.recorded_time_left(turn)?;
            let prev_time = response.recorded_time_left(turn - 2)?;
            let seconds = prev_time.checked_add(increment)?.as_secs_f64() - time_left.as_secs_f64();
            if seconds > 60.0 {
                Some(format!(" ({:.1} m)", seconds / 60.0))
            } else {
                Some(format!(" ({seconds:.2} s)"))
            }
        })
    };
    view! {
        <div
            node_ref=div_ref
            class=get_class
            data-history-current=move || is_current.get().to_string()
            on:click=onclick
        >
            {format!("{}. {piece} {position}{}", turn + 1, rep)}
            <Show when=is_realtime>{time_took}</Show>
        </div>
    }
}

#[component]
pub fn History(interaction: HivegroundInteraction, history_state: Memo<State>) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let params = use_params_map();
    let queries = use_query_map();
    let state = game_state.state();
    let is_finished = game_state.is_finished();
    let game_response = game_state.game_response();
    let repetitions = Memo::new(move |_| {
        game_response
            .with(|game_response| game_response.as_ref().map(|game| game.repetitions.clone()))
    });
    let history_moves = move || {
        state.with(|state| {
            state
                .history
                .moves
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, (piece, pos))| (i, piece, pos))
                .collect::<Vec<(usize, String, String)>>()
        })
    };

    let parent = NodeRef::<html::Div>::new();
    let game_result = Memo::new(move |_| {
        state.with(|state| match &state.game_status {
            GameStatus::Finished(result) => result.to_string(),
            _ => "".to_string(),
        })
    });

    let conclusion = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response.as_ref().map_or_else(
                || String::from("No data"),
                |game| game.conclusion.pretty_string(),
            )
        })
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
        <div class="flex flex-col pb-4 h-full">

            <HistoryControls parent=parent.into() interaction history_state />
            <Show when=is_finished>
                <div class="flex flex-col gap-2 px-2 pb-2">
                    <div class="flex flex-wrap gap-2 justify-center text-sm font-semibold text-center">
                        <span>{game_result}</span>
                        <span>{conclusion}</span>
                    </div>
                    <a
                        href=analysis_url
                        class="w-full ui-button ui-button-primary ui-button-md no-link-style"
                    >
                        <div class="flex gap-1 justify-center items-center">
                            <Icon icon=icondata_tb::TbMicroscopeOutline attr:class="py-1 size-7" />
                            "Analyze here"
                        </div>
                    </a>
                </div>
            </Show>
            <div
                node_ref=parent
                class="grid overflow-auto flex-1 grid-cols-4 gap-1 content-start min-h-0"
            >
                <For each=history_moves key=|history_move| history_move.0 let:history_move>

                    <HistoryMove
                        turn=history_move.0
                        piece=history_move.1
                        position=history_move.2
                        parent_div=parent
                        repetition=is_repetition(history_move.0)
                    />
                </For>
            </div>
        </div>
    }
}
