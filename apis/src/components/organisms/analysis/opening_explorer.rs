use crate::{
    common::PieceType,
    components::atoms::{bug_tile::BugTile, rating::icon_for_speed},
    functions::opening_explorer::opening_explorer,
    providers::{analysis::AnalysisSignal, game_state::GameStateSignal, ApiRequestsProvider},
    responses::ExplorerResponse,
};
use hive_lib::{GameStatus, GameType, Piece, Position, State};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{
    Conclusion,
    ExplorerFilters,
    ExplorerMove,
    GameProgress,
    GameSpeed,
    GamesQueryOptions,
    TimeMode,
};
use std::{collections::HashMap, str::FromStr};

/// Map every legal move from `state` to the canonical hash of the resulting position. This lets
/// the client translate a server suggestion (keyed by the resulting hash) back into a concrete
/// local move, independent of the rotational frame the suggestion's notation was authored in.
fn local_moves(state: &State) -> HashMap<u64, (Piece, Position)> {
    let mut map = HashMap::new();
    let base_len = state.hashes.len();
    let color = state.turn_color;
    for ((piece, _from), targets) in state.board.moves(color) {
        for target in targets {
            let mut s = state.clone();
            if s.play_turn_from_position(piece, target).is_ok() {
                // Use the move hash, not the auto-pass hash that may follow it.
                if let Some(&h) = s.hashes.get(base_len) {
                    map.entry(h).or_insert((piece, target));
                }
            }
        }
    }
    let reserve = state.board.reserve(color, state.game_type);
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    for pieces in reserve.values() {
        if let Some(piece) = pieces.first().and_then(|p| Piece::from_str(p).ok()) {
            for &target in &spawns {
                let mut s = state.clone();
                if s.play_turn_from_position(piece, target).is_ok() {
                    if let Some(&h) = s.hashes.get(base_len) {
                        map.entry(h).or_insert((piece, target));
                    }
                }
            }
        }
    }
    map
}

fn pct(part: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        (part as f64 / total as f64 * 100.0).round()
    }
}

/// The play / preview / reset handlers shared by every suggestion row, plus the (fixed for this
/// position) side-to-move used to compute the mover's score. All `Copy`, so rows capture freely.
#[derive(Clone, Copy)]
struct RowHandlers {
    play: Callback<(Piece, Position)>,
    preview: Callback<(Piece, Position)>,
    reset: Callback<()>,
    white_to_move: bool,
}

#[derive(Clone)]
pub struct AnalysisPreviewSnapshot {
    node_id: Option<i32>,
    state: State,
    history_turn: Option<usize>,
}

pub fn reset_analysis_preview(
    preview_snapshot: RwSignal<Option<AnalysisPreviewSnapshot>>,
    analysis: AnalysisSignal,
    game_state: GameStateSignal,
) {
    let Some(snapshot) = preview_snapshot.get_untracked() else {
        return;
    };

    if analysis
        .0
        .with_untracked(|analysis| analysis.current_node_id())
        == snapshot.node_id
    {
        game_state.signal.update(|game_state| {
            game_state.state = snapshot.state;
            game_state.history_turn = snapshot.history_turn;
            game_state.move_info.reset();
        });
    }
    preview_snapshot.set(None);
}

/// White/draw/black result bar for an aggregated position.
#[component]
fn ResultBar(white: i64, draws: i64, black: i64, total: i64) -> impl IntoView {
    let w = pct(white, total);
    let d = pct(draws, total);
    let b = pct(black, total);
    view! {
        <div class="flex overflow-hidden w-full h-4 leading-4 text-center rounded select-none text-[10px]">
            <div class="text-black bg-white" style:width=move || format!("{w}%")>
                {move || if w >= 12.0 { format!("{w:.0}%") } else { String::new() }}
            </div>
            <div class="text-white bg-gray-500" style:width=move || format!("{d}%")>
                {move || if d >= 12.0 { format!("{d:.0}%") } else { String::new() }}
            </div>
            <div class="text-white bg-black" style:width=move || format!("{b}%")>
                {move || if b >= 12.0 { format!("{b:.0}%") } else { String::new() }}
            </div>
        </div>
    }
}

#[component]
pub fn OpeningExplorer(
    preview_snapshot: RwSignal<Option<AnalysisPreviewSnapshot>>,
) -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let api = expect_context::<ApiRequestsProvider>().0;

    let game_type = analysis.with_untracked(|a| a.game_type);
    let filters = RwSignal::new(ExplorerFilters::new(game_type));

    // Current position hash from the analysis tree (0 = empty-board start node).
    let current_hash = Signal::derive(move || analysis.with(|a| a.current_hash()));

    let resource = Resource::new(
        move || (current_hash.get(), filters.get()),
        |(hash, filters)| async move { opening_explorer(hash as i64, filters).await },
    );

    let reset_preview = Callback::new(move |_: ()| {
        reset_analysis_preview(preview_snapshot, AnalysisSignal(analysis), game_state);
    });

    // Preview a suggested move: apply it to the real position and show it on the board/reserve.
    let preview_move = Callback::new(move |(piece, position): (Piece, Position)| {
        let base = match preview_snapshot.get_untracked() {
            Some(snapshot) => snapshot.state,
            None => {
                let snap = game_state
                    .signal
                    .with_untracked(|game_state| AnalysisPreviewSnapshot {
                        node_id: analysis.with_untracked(|analysis| analysis.current_node_id()),
                        state: game_state.state.clone(),
                        history_turn: game_state.history_turn,
                    });
                preview_snapshot.set(Some(snap.clone()));
                snap.state
            }
        };
        let mut previewed = base;
        if previewed.play_turn_from_position(piece, position).is_ok() {
            game_state.signal.update(move |gs| {
                gs.history_turn = Some(previewed.history.moves.len().saturating_sub(1));
                gs.state = previewed;
                gs.move_info.reset();
            });
        }
    });

    // Play a suggested move. First undo any active preview so we commit from the real position
    // (the board is showing the previewed state while hovered), then play it the normal way.
    let play_move = Callback::new(move |(piece, position): (Piece, Position)| {
        reset_analysis_preview(preview_snapshot, AnalysisSignal(analysis), game_state);
        game_state.signal.update(|gs| {
            gs.move_info.active = Some((piece, PieceType::Move));
            gs.move_info.target_position = Some(position);
        });
        let mut g = game_state;
        g.move_active(Some(AnalysisSignal(analysis)), api.get_untracked());
    });

    // The archive URL for the current position ("Search this position"), or None at the empty
    // board. Recomputed when the position/filters change (the suggestions closure reruns then).
    let search_href = move || {
        let hash = current_hash.get_untracked();
        if hash == 0 {
            return None;
        }
        let expansions = match filters.with_untracked(|f| f.game_type) {
            GameType::Base => Some(false),
            GameType::MLP => Some(true),
            _ => None,
        };
        let opts = GamesQueryOptions {
            position_hash: Some(hash as i64),
            expansions,
            game_progress: GameProgress::Finished,
            ..GamesQueryOptions::default()
        };
        Some(format!("/archive{opts}"))
    };

    let suggestions = move || {
        resource.get().map(|result| match result {
            Err(_) => view! { <div class="p-2">"Failed to load opening data."</div> }.into_any(),
            Ok(response) => {
                let local = game_state
                    .signal
                    .with_untracked(|gs| local_moves(&gs.state));
                let white_to_move = game_state
                    .signal
                    .with_untracked(|gs| gs.state.turn % 2 == 0);
                let handlers = RowHandlers {
                    play: play_move,
                    preview: preview_move,
                    reset: reset_preview,
                    white_to_move,
                };
                render_response(response, handlers, &local, search_href()).into_any()
            }
        })
    };

    view! {
        <div class="flex flex-col p-1 text-sm select-none">
            <div class="font-bold">"Opening explorer"</div>
            <FilterControls filters />
            // Transition (not Suspense) keeps the prior results on screen while a new query loads,
            // so toggling filters doesn't collapse the panel to "Loading…" and jump.
            <div class="min-h-64">
                <Transition fallback=move || {
                    view! { <div class="p-2">"Loading…"</div> }
                }>{suggestions}</Transition>
            </div>
        </div>
    }
}

fn render_response(
    response: ExplorerResponse,
    handlers: RowHandlers,
    local: &HashMap<u64, (Piece, Position)>,
    search_href: Option<String>,
) -> impl IntoView {
    let header = response.position_total;
    let total = header.total;
    let games_label = if total == 1 {
        "1 game".to_string()
    } else {
        format!("{total} games")
    };
    let has_top = !response.top_games.is_empty();
    let has_recent = !response.recent_games.is_empty();
    let rows = response
        .moves
        .into_iter()
        .filter(|m| m.total > 0)
        .map(|m| {
            let local_move = local.get(&(m.next_hash as u64)).copied();
            move_row(m, local_move, handlers)
        })
        .collect_view();
    let top_games = response
        .top_games
        .into_iter()
        .map(top_game_row)
        .collect_view();
    let recent_games = response
        .recent_games
        .into_iter()
        .map(top_game_row)
        .collect_view();

    let table = if total > 0 {
        view! {
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="text-left text-gray-500 dark:text-gray-400">
                        <th class="py-1 px-2 font-normal">"Move"</th>
                        <th class="py-1 px-2 font-normal text-right">"Games"</th>
                        <th class="py-1 px-2 font-normal text-right">"Score"</th>
                        <th class="py-1 px-2 w-2/5 font-normal">"W / D / B"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
        }
        .into_any()
    } else {
        view! { <div class="py-2 px-2 italic">"No games for this position."</div> }.into_any()
    };
    let search_link = search_href.map(|href| {
        view! {
            <div class="flex justify-center mt-2">
                <a
                    href=href
                    target="_blank"
                    rel="noopener"
                    class="flex justify-center items-center px-4 h-10 text-white rounded-sm transition-transform duration-300 active:scale-95 no-link-style bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
                >
                    "Search this position ↗"
                </a>
            </div>
        }
    });
    let top_header =
        has_top.then(|| view! {
            <div class="mt-3 mb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                "Top games"
            </div>
        });
    let recent_header =
        has_recent.then(|| view! {
            <div class="mt-3 mb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                "Recent games"
            </div>
        });

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex gap-2 items-center py-1 px-2">
                // Fixed width so the bar's remaining space stays constant across game counts.
                <span class="w-24 tabular-nums whitespace-nowrap shrink-0">{games_label}</span>
                <ResultBar
                    white=header.white_wins
                    draws=header.draws
                    black=header.black_wins
                    total=header.total
                />
            </div>
            {table}
            {search_link}
            {top_header}
            <div class="flex flex-col">{top_games}</div>
            {recent_header}
            <div class="flex flex-col">{recent_games}</div>
        </div>
    }
}

/// One entry in the "Top games" list: players + ratings, result and time control, opens the game
/// in a new tab.
fn top_game_row(g: crate::responses::GameResponse) -> impl IntoView {
    let href = format!("/game/{}", g.game_id.0);
    let rating = |r: Option<f64>| {
        r.map(|r| format!(" ({})", r.round() as i64))
            .unwrap_or_default()
    };
    let white = format!("{}{}", g.white_player.username, rating(g.white_rating));
    let black = format!("{}{}", g.black_player.username, rating(g.black_rating));
    let result = match &g.game_status {
        GameStatus::Finished(res) => res.to_string(),
        _ => String::new(),
    };
    let (conclusion_icon, is_rep) = match g.conclusion {
        Conclusion::Timeout => (Some(icondata_bs::BsHourglassSplit), false),
        Conclusion::Resigned => (Some(icondata_ai::AiFlagOutlined), false),
        Conclusion::Board => (Some(icondata_mdi::MdiHexagonMultiple), false),
        Conclusion::Draw => (Some(icondata_fa::FaHandshakeSimpleSolid), false),
        Conclusion::Repetition => (None, true),
        _ => (None, false),
    };
    let speed_icon = icon_for_speed(g.speed);
    let time_text = match g.time_mode {
        TimeMode::Untimed => "∞".to_string(),
        TimeMode::RealTime => format!(
            "{} + {}",
            g.time_base.unwrap_or(0) / 60,
            g.time_increment.unwrap_or(0),
        ),
        TimeMode::Correspondence => {
            if let Some(base) = g.time_base {
                format!("{} d/side", base / 86400)
            } else if let Some(inc) = g.time_increment {
                format!("{} d/move", inc / 86400)
            } else {
                String::new()
            }
        }
    };
    view! {
        <a
            href=href
            target="_blank"
            rel="noopener"
            class="block py-1 px-2 rounded transition-colors hover:bg-blue-100 no-link-style dark:hover:bg-slate-700"
        >
            <div class="flex gap-2 justify-between items-center">
                <span class="truncate">{white} " – " {black}</span>
                <span class="flex gap-1 items-center font-mono shrink-0">
                    {conclusion_icon
                        .map(|icon| view! { <Icon icon attr:class="size-4 shrink-0" /> })}
                    {is_rep.then_some("↺")} {result}
                </span>
            </div>
            <div class="flex gap-1 items-center text-xs text-gray-500 dark:text-gray-400">
                <Icon icon=speed_icon attr:class="size-4 shrink-0" />
                {time_text}
                " · "
                {g.created_at.format("%Y-%m-%d").to_string()}
            </div>
        </a>
    }
}

fn move_row(
    m: ExplorerMove,
    local_move: Option<(Piece, Position)>,
    handlers: RowHandlers,
) -> impl IntoView {
    let label = if m.position.is_empty() {
        m.piece.clone()
    } else {
        format!("{} {}", m.piece, m.position)
    };
    let total = m.total;
    let (white, draws, black) = (m.white_wins, m.draws, m.black_wins);
    let mover_wins = if handlers.white_to_move { white } else { black };
    let score = pct(mover_wins, total); // win share; draws shown separately in the bar
                                        // Prefer the locally-derived piece (matches the move that gets played); fall back to the
                                        // server's representative label when we can't map the suggestion onto a local move.
    let icon_piece = local_move
        .map(|(piece, _)| piece)
        .or_else(|| Piece::from_str(&m.piece).ok());
    view! {
        <tr
            class="border-t border-gray-200 transition-colors cursor-pointer dark:border-gray-700 hover:bg-blue-100 dark:hover:bg-slate-700"
            on:click=move |_| {
                if let Some(mv) = local_move {
                    handlers.play.run(mv);
                }
            }
            on:mouseenter=move |_| {
                if let Some(mv) = local_move {
                    handlers.preview.run(mv);
                }
            }
            on:mouseleave=move |_| handlers.reset.run(())
        >
            <td class="py-1.5 px-2 whitespace-nowrap">
                <div class="flex gap-1.5 items-center">
                    {icon_piece.map(|piece| view! { <BugTile piece /> })}
                    <span class="font-mono">{label}</span>
                </div>
            </td>
            <td class="py-1.5 px-2 tabular-nums text-right">{total.to_string()}</td>
            <td class="py-1.5 px-2 tabular-nums text-right">{format!("{score:.0}%")}</td>
            <td class="py-1.5 px-2">
                <ResultBar white=white draws=draws black=black total=total />
            </td>
        </tr>
    }
}

#[component]
fn FilterControls(filters: RwSignal<ExplorerFilters>) -> impl IntoView {
    const SELECT_CLASS: &str = "w-full min-h-10 px-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-white text-gray-900 dark:bg-gray-800 dark:text-gray-100 shadow-sm focus:outline-none focus:ring-2 focus:ring-pillbug-teal/50 focus:border-pillbug-teal";
    const LABEL_CLASS: &str = "block text-sm font-semibold text-gray-700 dark:text-gray-200";

    // Speed pill: highlighted when its speed is in the selected set.
    let speed_pill_class = move |speed: GameSpeed| {
        move || {
            let base = "flex justify-center items-center text-sm rounded-lg border-2 shadow-sm transition-colors duration-150 size-10 cursor-pointer";
            if filters.with(|f| f.speeds.contains(&speed)) {
                format!("{base} border-pillbug-teal bg-pillbug-teal/10 text-pillbug-teal")
            } else {
                format!("{base} border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-900 hover:bg-gray-50 dark:hover:bg-gray-800 hover:border-pillbug-teal/40")
            }
        }
    };
    // Multi-select toggle: add/remove the speed, keeping at least one selected.
    let toggle_speed = move |speed: GameSpeed| {
        filters.update(|f| {
            if let Some(pos) = f.speeds.iter().position(|v| v == &speed) {
                if f.speeds.len() > 1 {
                    f.speeds.remove(pos);
                }
            } else {
                f.speeds.push(speed);
                f.speeds.sort();
                f.speeds.dedup();
            }
        });
    };
    // Show Untimed only when not restricted to rated games (rated games can't be untimed).
    let show_untimed = Signal::derive(move || filters.with(|f| f.rated != Some(true)));
    let untimed_pill_class = speed_pill_class(GameSpeed::Untimed);

    view! {
        <div class="py-1 space-y-2">
            <div class="grid grid-cols-2 gap-2">
                <div class="space-y-1">
                    <label class=LABEL_CLASS>"Type"</label>
                    <select
                        class=SELECT_CLASS
                        prop:value=Signal::derive(move || filters.with(|f| f.game_type.to_string()))
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            if let Ok(gt) = GameType::from_str(&v) {
                                filters.update(|f| f.game_type = gt);
                            }
                        }
                    >
                        <option value="Base">"Base"</option>
                        <option value="Base+MLP">"Base+MLP"</option>
                    </select>
                </div>
                <div class="space-y-1">
                    <label class=LABEL_CLASS>"Rated"</label>
                    <select
                        class=SELECT_CLASS
                        prop:value=Signal::derive(move || match filters.with(|f| f.rated) {
                            Some(true) => "rated".to_string(),
                            Some(false) => "casual".to_string(),
                            None => "any".to_string(),
                        })
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            filters
                                .update(|f| {
                                    f.rated = match v.as_str() {
                                        "rated" => Some(true),
                                        "casual" => Some(false),
                                        _ => None,
                                    };
                                    if f.rated == Some(true) {
                                        f.speeds.retain(|s| *s != GameSpeed::Untimed);
                                        if f.speeds.is_empty() {
                                            f.speeds = GameSpeed::all_rated_games();
                                        }
                                    }
                                });
                        }
                    >
                        <option value="rated">"Rated"</option>
                        <option value="any">"Any"</option>
                        <option value="casual">"Casual"</option>
                    </select>
                </div>
            </div>
            <div class="flex flex-wrap gap-2">
                {[
                    GameSpeed::Bullet,
                    GameSpeed::Blitz,
                    GameSpeed::Rapid,
                    GameSpeed::Classic,
                    GameSpeed::Correspondence,
                ]
                    .into_iter()
                    .map(|speed| {
                        let icon = icon_for_speed(speed);
                        view! {
                            <label class=speed_pill_class(speed) title=speed.to_string()>
                                <input
                                    type="checkbox"
                                    class="sr-only"
                                    prop:checked=move || filters.with(|f| f.speeds.contains(&speed))
                                    on:change=move |_| toggle_speed(speed)
                                />
                                <Icon icon attr:class="size-5" />
                                <span class="sr-only">{speed.to_string()}</span>
                            </label>
                        }
                    })
                    .collect_view()} // Always rendered (slot kept) and hidden via Tailwind when
                // rated-only, so the pill grid doesn't reflow when the Rated filter changes.
                <label
                    class=move || {
                        let base = untimed_pill_class();
                        if show_untimed.get() {
                            base
                        } else {
                            format!("{base} invisible pointer-events-none")
                        }
                    }
                    title="Untimed"
                >
                    <input
                        type="checkbox"
                        class="sr-only"
                        prop:checked=move || {
                            filters.with(|f| f.speeds.contains(&GameSpeed::Untimed))
                        }
                        on:change=move |_| toggle_speed(GameSpeed::Untimed)
                    />
                    <Icon icon=icon_for_speed(GameSpeed::Untimed) attr:class="size-5" />
                    <span class="sr-only">"Untimed"</span>
                </label>
            </div>
        </div>
    }
}
