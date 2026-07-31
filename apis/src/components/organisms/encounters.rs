use crate::{
    components::molecules::{game_previews::GamePreviews, panel::Panel},
    responses::{GameResponse, TournamentResponse},
};
use leptos::prelude::*;
use shared_types::{ByeKind, Conclusion, GameId, Tiebreaker, TournamentGameResult, TournamentMode};
use std::collections::BTreeMap;
use uuid::Uuid;

/// One pairing in one round, however many games it took.
///
/// A Double Swiss or bracket match is two colour-swapped games scored as a single
/// contest, so showing them as two rows says the pair met twice when they met
/// once. Merged here into one row carrying the aggregate — and both game ids, so
/// each name can lead to the game that player had white in.
#[derive(Clone)]
struct Meeting {
    left: Uuid,
    right: Uuid,
    left_score: f32,
    right_score: f32,
    left_as_white: Option<GameId>,
    right_as_white: Option<GameId>,
    /// Any game of the pairing, so a side with no white game still links
    /// somewhere useful. In a single-game round only one player had white, and an
    /// empty `href` navigates to the current page — which looked like the name
    /// linking to the tournament.
    any_game: Option<GameId>,
    games: usize,
    decided: usize,
}

impl Meeting {
    fn played(&self) -> bool {
        self.decided > 0
    }

    /// The game to open for one side: the one they had white in, else whichever
    /// game of the pairing exists.
    fn game_for(&self, left: bool) -> Option<&GameId> {
        let side = if left {
            &self.left_as_white
        } else {
            &self.right_as_white
        };
        side.as_ref().or(self.any_game.as_ref())
    }

    fn tooltip(&self, left: bool) -> String {
        let side = if left {
            &self.left_as_white
        } else {
            &self.right_as_white
        };
        match (self.games > 1, side.is_some()) {
            (true, true) => String::from("Open the game this player had white in"),
            (true, false) => String::from("This player had black in both games"),
            _ => String::from("Open the game"),
        }
    }
}

fn unordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Groups a round's games into one entry per pairing.
fn meetings_of(games: &[GameResponse]) -> Vec<Meeting> {
    let mut meetings: Vec<Meeting> = Vec::new();

    for game in games {
        let white = game.white_player.uid;
        let black = game.black_player.uid;
        let (left, right) = unordered(white, black);

        let meeting = match meetings
            .iter_mut()
            .find(|meeting| meeting.left == left && meeting.right == right)
        {
            Some(meeting) => meeting,
            None => {
                meetings.push(Meeting {
                    left,
                    right,
                    left_score: 0.0,
                    right_score: 0.0,
                    left_as_white: None,
                    right_as_white: None,
                    any_game: None,
                    games: 0,
                    decided: 0,
                });
                meetings.last_mut().expect("just pushed")
            }
        };

        meeting.games += 1;
        meeting.any_game = Some(game.game_id.clone());
        if white == meeting.left {
            meeting.left_as_white = Some(game.game_id.clone());
        } else {
            meeting.right_as_white = Some(game.game_id.clone());
        }

        let (to_white, to_black) = match game.tournament_game_result {
            TournamentGameResult::Winner(hive_lib::Color::White) => (1.0, 0.0),
            TournamentGameResult::Winner(hive_lib::Color::Black) => (0.0, 1.0),
            TournamentGameResult::Draw => (0.5, 0.5),
            // A double forfeit is settled at nothing each, rather than unplayed.
            TournamentGameResult::DoubeForfeit => (0.0, 0.0),
            TournamentGameResult::Unknown => continue,
        };
        meeting.decided += 1;
        let (to_left, to_right) = if white == meeting.left {
            (to_white, to_black)
        } else {
            (to_black, to_white)
        };
        meeting.left_score += to_left;
        meeting.right_score += to_right;
    }

    meetings
}

/// An all-play-all grid: row against column, from the row player's point of view.
///
/// Round robin has rounds, but only as a scheduling device — the whole schedule is
/// fixed before a game is played, so nothing hinges on which round a result landed
/// in. What matters is who beat whom, which is what a cross-table shows. It also
/// puts a pair's two meetings in one cell, where a double round robin splits them
/// across rounds five apart.
#[component]
fn CrossTable(
    tournament: Signal<TournamentResponse>,
    highlighted: RwSignal<Option<Uuid>>,
    show_standings: RwSignal<bool>,
    name_of: impl Fn(Uuid) -> String + Clone + Send + Sync + 'static,
) -> impl IntoView {
    let order = move || {
        tournament.with(|t| {
            t.standings
                .groups
                .iter()
                .flatten()
                .map(|standing| (standing.position, standing.player))
                .collect::<Vec<_>>()
        })
    };
    // Keyed on the pair without a round, so both meetings aggregate into one cell.
    let meetings = move || tournament.with(|t| meetings_of(&t.games));
    // Which cell is open. A pair can have several games and picking one for the
    // reader was a guess — the cell now lists them instead.
    let opened = RwSignal::new(None::<(Uuid, Uuid)>);

    // The round a player stopped, derived from the earliest game the engine
    // forfeited for them. Without rounds on screen there is nowhere else for a
    // cross-table to say when somebody left.
    let withdrew_after = move |player: Uuid| {
        tournament.with(|t| {
            if !t.withdrawn.contains(&player) {
                return None;
            }
            t.games
                .iter()
                .filter(|game| {
                    game.conclusion == Conclusion::Withdrawal
                        && (game.white_player.uid == player || game.black_player.uid == player)
                })
                .filter_map(|game| game.round)
                .min()
                .map(|round| round.saturating_sub(1))
        })
    };

    let cell_of = move |row: Uuid, column: Uuid| {
        meetings().into_iter().find_map(|meeting| {
            if meeting.left == row && meeting.right == column {
                Some((meeting.left_score, meeting.left_as_white.clone(), meeting))
            } else if meeting.right == row && meeting.left == column {
                Some((meeting.right_score, meeting.right_as_white.clone(), meeting))
            } else {
                None
            }
        })
    };

    view! {
        <div class="space-y-3">
            // The opened cell's games go *below* the grid, not beside it: a
            // cross-table is as wide as the field, so anything sharing its row
            // squeezes it — far enough that the cell you clicked scrolled out of
            // reach and could not be closed again.
            <div class="flex flex-col gap-4">
                <div class="overflow-x-auto" data-testid="cross-table">
                    <table class="text-sm border-collapse">
                        <thead>
                            <tr>
                                <th class=CROSS_HEAD></th>
                                <th class=CROSS_HEAD>"Player"</th>
                                // Names rather than positions: a header row of digits
                                // gives no way to tell who a cell is against without
                                // counting rows off against the list on the left.
                                {{
                                    let name_of = name_of.clone();
                                    move || {
                                        let name_of = name_of.clone();
                                        order()
                                            .into_iter()
                                            .map(|(_, player)| {
                                                view! {
                                                    <th class=CROSS_HEAD_NAME>
                                                        <span class="whitespace-nowrap">{name_of(player)}</span>
                                                    </th>
                                                }
                                            })
                                            .collect_view()
                                    }
                                }}
                                <th class=CROSS_HEAD>"Total"</th>
                            </tr>
                        </thead>
                        <tbody on:mouseleave=move |_| {
                            highlighted.set(None)
                        }>
                            {{
                                let rows_name_of = name_of.clone();
                                move || {
                                    let order = order();
                                    let name_of = rows_name_of.clone();
                                    order
                                        .iter()
                                        .map(|(position, row)| {
                                            let (position, row) = (*position, *row);
                                            let name_of = name_of.clone();
                                            let total: f32 = order
                                                .iter()
                                                .filter_map(|(_, column)| {
                                                    (*column != row)
                                                        .then(|| cell_of(row, *column).map(|(score, _, _)| score))
                                                        .flatten()
                                                })
                                                .sum();
                                            view! {
                                                <tr
                                                    class=move || {
                                                        let base = "ui-dense-table-row";
                                                        if highlighted.get() == Some(row) {
                                                            format!("{base} !bg-pillbug-teal/25")
                                                        } else {
                                                            base.to_owned()
                                                        }
                                                    }
                                                    on:mouseenter=move |_| highlighted.set(Some(row))
                                                >
                                                    <td class=CROSS_CELL>{position}</td>
                                                    <td class="py-1 px-2 text-left whitespace-nowrap">
                                                        {
                                                            let left = withdrew_after(row);
                                                            view! {
                                                                <span class=if left.is_some() {
                                                                    "line-through decoration-2 opacity-60"
                                                                } else {
                                                                    ""
                                                                }>{name_of(row)}</span>
                                                                {left
                                                                    .map(|round| {
                                                                        view! {
                                                                            <span
                                                                                class="ml-1 tracking-tight text-gray-500 uppercase text-[10px]"
                                                                                title="Left the tournament; their remaining games were forfeited"
                                                                            >
                                                                                {format!("withdrew after r{round}")}
                                                                            </span>
                                                                        }
                                                                    })}
                                                            }
                                                        }
                                                    </td>
                                                    {order
                                                        .iter()
                                                        .map(|(_, column)| {
                                                            let column = *column;
                                                            if column == row {
                                                                return view! {
                                                                    <td class=CROSS_CELL>
                                                                        <span class="text-gray-400 dark:text-gray-600">"—"</span>
                                                                    </td>
                                                                }
                                                                    .into_any();
                                                            }
                                                            match cell_of(row, column) {
                                                                Some((score, _, meeting)) => {
                                                                    let played = meeting.played();
                                                                    let pair = unordered(row, column);
                                                                    let is_open = move || opened.get() == Some(pair);
                                                                    view! {
                                                                        <td class=CROSS_CELL>
                                                                            // A cell can hold two games, so it opens the
                                                                            // list below rather than guessing which one
                                                                            // the reader meant.
                                                                            <button
                                                                                type="button"
                                                                                class=move || {
                                                                                    if is_open() {
                                                                                        "w-8 rounded bg-pillbug-teal font-bold tabular-nums text-white ring-2 ring-pillbug-teal/40"
                                                                                    } else {
                                                                                        "w-8 rounded tabular-nums hover:bg-pillbug-teal/20"
                                                                                    }
                                                                                }
                                                                                attr:aria-expanded=move || is_open().to_string()
                                                                                on:click=move |_| {
                                                                                    opened
                                                                                        .update(|open| {
                                                                                            *open = if *open == Some(pair) { None } else { Some(pair) };
                                                                                        })
                                                                                }
                                                                                title=format!(
                                                                                    "{} of {} against {} — click for the games",
                                                                                    format_score(score),
                                                                                    meeting.games,
                                                                                    name_of(column),
                                                                                )
                                                                            >
                                                                                {if played {
                                                                                    format_score(score)
                                                                                } else {
                                                                                    String::from("–")
                                                                                }}
                                                                            </button>
                                                                        </td>
                                                                    }
                                                                        .into_any()
                                                                }
                                                                None => view! { <td class=CROSS_CELL></td> }.into_any(),
                                                            }
                                                        })
                                                        .collect_view()}
                                                    <td class=CROSS_CELL>
                                                        <span class="font-bold tabular-nums">
                                                            {format_score(total)}
                                                        </span>
                                                    </td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()
                                }
                            }}
                        </tbody>
                    </table>
                </div>
                // Every game behind the opened cell, rather than one chosen for the
                // reader — a double round robin has two, played rounds apart.
                {{
                    let name_of = name_of.clone();
                    move || {
                        let (row, column) = opened.get()?;
                        let name_of = name_of.clone();
                        let games = tournament
                            .with(|t| {
                                let mut games: Vec<GameResponse> = t
                                    .games
                                    .iter()
                                    .filter(|game| {
                                        let pair = unordered(
                                            game.white_player.uid,
                                            game.black_player.uid,
                                        );
                                        pair == (row, column)
                                    })
                                    .cloned()
                                    .collect();
                                games.sort_by_key(|game| game.round);
                                games
                            });
                        Some(
                            view! {
                                <div class="p-2 space-y-2 w-full min-w-0 ui-setting-group">
                                    <div class="flex gap-2 justify-between items-start">
                                        <p class="text-sm font-bold">
                                            {format!("{} vs {}", name_of(row), name_of(column))}
                                        </p>
                                        // Its own control rather than relying on
                                        // clicking the cell again, which a wide
                                        // grid can scroll out of reach.
                                        <button
                                            type="button"
                                            class="shrink-0 ui-button ui-button-secondary ui-button-sm"
                                            title="Close"
                                            on:click=move |_| opened.set(None)
                                        >
                                            "Close"
                                        </button>
                                    </div>
                                    // The real preview rather than a line of text — the
                                    // board and the clocks are what makes a result worth
                                    // opening.
                                    <GamePreviews games=games show_time=true />
                                </div>
                            },
                        )
                    }
                }}
            </div>
            <button
                type="button"
                class="ui-button ui-button-secondary ui-button-sm"
                attr:aria-expanded=move || show_standings.get().to_string()
                on:click=move |_| show_standings.update(|open| *open = !*open)
            >
                {move || if show_standings.get() { "Hide standings" } else { "Standings" }}
            </button>
        </div>
    }
}

const CROSS_HEAD: &str =
    "px-2 py-1 text-[10px] font-bold uppercase tracking-tight text-gray-700 dark:text-gray-300";
/// A player's name is their own — upper-casing it makes `tt-04` into `TT-04`,
/// which is not what they are called.
const CROSS_HEAD_NAME: &str =
    "px-2 py-1 text-xs font-bold tracking-tight text-gray-700 dark:text-gray-300";
const CROSS_CELL: &str = "px-2 py-1 text-center";

/// Who played whom, in which round, and how it went.
///
/// The standings answer "where did I finish" but not "who did I actually play",
/// which in a Swiss is the more interesting question — the table alone cannot
/// show that two players on equal points had completely different opposition.
/// Hovering a player on the left picks their games out of the grid on the right.
#[component]
pub fn Encounters(
    tournament: Signal<TournamentResponse>,
    /// Shared with the standings panel below, which this component's own button
    /// unfolds.
    show_standings: RwSignal<bool>,
) -> impl IntoView {
    let highlighted = RwSignal::new(None::<Uuid>);

    let name_of = move |player: Uuid| {
        tournament.with(|t| {
            t.players
                .get(&player)
                .map(|user| user.username.clone())
                .unwrap_or_else(|| String::from("—"))
        })
    };

    // Best-first, flattened out of the tie groups, so the order matches the
    // standings table above it.
    let ranked = move || {
        tournament.with(|t| {
            t.standings
                .groups
                .iter()
                .flatten()
                .map(|standing| {
                    let points = standing
                        .scores
                        .get(&Tiebreaker::RawPoints)
                        .copied()
                        .unwrap_or(0.0);
                    (standing.position, standing.player, points)
                })
                .collect::<Vec<_>>()
        })
    };

    let rounds = move || {
        tournament.with(|t| {
            let mut rounds: BTreeMap<i32, Vec<GameResponse>> = BTreeMap::new();
            for game in &t.games {
                if let Some(round) = game.round {
                    rounds.entry(round).or_default().push(game.clone());
                }
            }
            rounds
                .into_iter()
                .map(|(round, games)| (round, meetings_of(&games)))
                .collect::<Vec<_>>()
        })
    };

    // A bye produces no game, so it would otherwise be a silent gap in the round
    // it belongs to — the round is exactly where it makes sense to show it.
    let byes_in = move |round: i32| {
        tournament.with(|t| {
            t.byes
                .iter()
                .filter(|bye| bye.round == round)
                .map(|bye| (bye.player, bye.kind))
                .collect::<Vec<_>>()
        })
    };

    // Derived from the games rather than carried: `withdrawn_at` is a timestamp,
    // and what a reader wants is the round the player stopped — which is the
    // earliest round the engine forfeited on their behalf.
    let withdrawals_in = move |round: i32| {
        tournament.with(|t| {
            let mut left = Vec::new();
            for player in &t.withdrawn {
                let first_forfeit = t
                    .games
                    .iter()
                    .filter(|game| {
                        game.conclusion == Conclusion::Withdrawal
                            && (game.white_player.uid == *player
                                || game.black_player.uid == *player)
                    })
                    .filter_map(|game| game.round)
                    .min();
                if first_forfeit == Some(round) {
                    left.push(*player);
                }
            }
            left
        })
    };

    // Round robin gets the grid; Swiss keeps the rounds, where the round is the
    // competitive structure rather than a timetable.
    let all_play_all = tournament.with_untracked(|t| {
        t.mode
            .parse::<TournamentMode>()
            .is_ok_and(|mode| mode.is_round_robin())
    });

    view! {
        <Show when=move || !rounds().is_empty()>
            <Panel title="Encounters" class="min-w-0">
                <Show
                    when=move || !all_play_all
                    fallback=move || {
                        view! {
                            <CrossTable
                                tournament
                                highlighted
                                show_standings
                                name_of=name_of.clone()
                            />
                        }
                    }
                >
                    <div class="flex flex-col gap-4 lg:flex-row lg:items-start">
                        <div class="flex-shrink-0 w-full lg:w-56">
                            <ul on:mouseleave=move |_| {
                                highlighted.set(None)
                            }>
                                {move || {
                                    ranked()
                                        .into_iter()
                                        .map(|(position, player, points)| {
                                            let name = name_of(player);
                                            let is_on = move || highlighted.get() == Some(player);
                                            view! {
                                                <li
                                                    // The base class always applies: it carries the
                                                    // border and striping, so swapping it out for the
                                                    // highlight shifted the row's contents. Only the
                                                    // background changes, and both overrides need `!`
                                                    // to beat the `odd:`/`even:` and `text-center`
                                                    // the shared row class brings with it.
                                                    class=move || {
                                                        let base = "flex gap-2 items-center px-2 py-1 text-sm cursor-default ui-dense-table-row !text-left";
                                                        if is_on() {
                                                            format!("{base} !bg-pillbug-teal/25")
                                                        } else {
                                                            base.to_owned()
                                                        }
                                                    }
                                                    on:mouseenter=move |_| highlighted.set(Some(player))
                                                >
                                                    <span class="w-6 text-xs tabular-nums text-gray-500">
                                                        {position}
                                                    </span>
                                                    <span class="flex-1 truncate">{name}</span>
                                                    <span class="text-xs font-bold tabular-nums">{points}</span>
                                                </li>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </ul>
                            // The full table lives below and starts folded away: this
                            // list already answers "who is winning", and the tiebreaker
                            // columns are only wanted when somebody asks why.
                            <button
                                type="button"
                                class="mt-2 w-full ui-button ui-button-secondary ui-button-sm"
                                attr:aria-expanded=move || show_standings.get().to_string()
                                on:click=move |_| show_standings.update(|open| *open = !*open)
                            >
                                {move || {
                                    if show_standings.get() {
                                        "Hide standings"
                                    } else {
                                        "Standings"
                                    }
                                }}
                            </button>
                        </div>
                        <div class="overflow-x-auto min-w-0 grow" data-testid="encounters-grid">
                            // Columns share the width rather than bunching up on the
                            // left, and fall back to scrolling once there are more
                            // rounds than fit.
                            <div class="flex gap-3 items-stretch w-full">
                                {move || {
                                    rounds()
                                        .into_iter()
                                        .map(|(round, meetings)| {
                                            view! {
                                                <div class="flex flex-col flex-1 gap-2 min-w-48">
                                                    <h3 class="text-xs font-bold tracking-tight text-gray-700 uppercase dark:text-gray-300">
                                                        {format!("Round {round}")}
                                                    </h3>
                                                    {meetings
                                                        .into_iter()
                                                        .map(|meeting| {
                                                            view! {
                                                                <EncounterRow meeting highlighted name_of=name_of.clone() />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {byes_in(round)
                                                        .into_iter()
                                                        .map(|(player, kind)| {
                                                            let (note, explanation) = match kind {
                                                                ByeKind::PairingAllocated => {
                                                                    (
                                                                        "bye",
                                                                        "Odd number of players, so the pairing gave this player the round off. Worth a full point.",
                                                                    )
                                                                }
                                                                ByeKind::ZeroPoint => {
                                                                    (
                                                                        "sat out",
                                                                        "This player asked to skip the round. Scores whatever the tournament set for a requested bye — nothing, by default.",
                                                                    )
                                                                }
                                                            };
                                                            view! {
                                                                <SatOut
                                                                    player
                                                                    highlighted
                                                                    name=name_of(player)
                                                                    note
                                                                    explanation
                                                                    emphatic=false
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                    {withdrawals_in(round)
                                                        .into_iter()
                                                        .map(|player| {
                                                            view! {
                                                                <SatOut
                                                                    player
                                                                    highlighted
                                                                    name=name_of(player)
                                                                    note="withdrew"
                                                                    explanation="Left the tournament from this round on. Their remaining games were forfeited."
                                                                    emphatic=true
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </div>
                    </div>
                </Show>
            </Panel>
        </Show>
    }
}

/// A round entry for somebody who did not play: a bye, or the point they left.
///
/// Sized and highlighted like a pairing so the round reads as a complete account
/// of everyone in it, rather than a list of games with unexplained gaps.
#[component]
fn SatOut(
    player: Uuid,
    highlighted: RwSignal<Option<Uuid>>,
    name: String,
    note: &'static str,
    /// What the note means, since "bye" and "sat out" score differently and the
    /// difference is not guessable from the word.
    explanation: &'static str,
    /// A withdrawal is a departure rather than a rest, so it is not dimmed away.
    emphatic: bool,
) -> impl IntoView {
    let outer = move || {
        let dimmed = highlighted
            .get()
            .is_some_and(|highlighted| highlighted != player);
        let tone = if emphatic {
            "border-dashed border-gray-500 text-gray-500 dark:border-gray-500 dark:text-gray-400"
        } else {
            "border-dashed border-pillbug-teal/60 text-pillbug-teal"
        };
        let shade = if highlighted.get() == Some(player) {
            "bg-pillbug-teal/25"
        } else {
            ""
        };
        format!(
            "flex justify-between gap-2 rounded border px-2 py-0.5 text-xs transition-opacity {tone} {shade} {}",
            if dimmed { "opacity-30" } else { "" },
        )
    };

    view! {
        <div class=outer title=explanation on:mouseenter=move |_| highlighted.set(Some(player))>
            <span class="truncate">{name}</span>
            <span class="tracking-tight uppercase text-[10px]">{note}</span>
        </div>
    }
}

#[component]
fn EncounterRow(
    meeting: Meeting,
    highlighted: RwSignal<Option<Uuid>>,
    name_of: impl Fn(Uuid) -> String + Clone + 'static,
) -> impl IntoView {
    let (left, right) = (meeting.left, meeting.right);
    let involved = move || {
        highlighted
            .get()
            .is_some_and(|player| player == left || player == right)
    };
    // Dimmed rather than hidden while something else is hovered: the grid keeps
    // its shape, so picking out one player's path does not reflow the panel.
    let outer = move || {
        if highlighted.get().is_none() || involved() {
            "overflow-hidden rounded border border-gray-300 dark:border-gray-700 text-xs transition-opacity"
        } else {
            "overflow-hidden rounded border border-gray-300 dark:border-gray-700 text-xs transition-opacity opacity-30"
        }
    };

    // The two sides get different backgrounds so a pairing reads as two players
    // rather than one block of four values, following the same odd/even pattern
    // the dense tables use. The leader is emphasised on top of that.
    let side = move |player: Uuid, ahead: bool, odd: bool| {
        move || {
            let shade = if highlighted.get() == Some(player) {
                "bg-pillbug-teal/25"
            } else if odd {
                "bg-odd-light dark:bg-surface-row-odd"
            } else {
                "bg-even-light dark:bg-surface-row-even"
            };
            let weight = if ahead {
                "font-bold text-gray-900 dark:text-gray-100"
            } else {
                "text-gray-600 dark:text-gray-400"
            };
            format!("flex justify-between gap-2 px-2 py-0.5 no-link-style {shade} {weight}")
        }
    };

    let played = meeting.played();
    let left_ahead = played && meeting.left_score > meeting.right_score;
    let right_ahead = played && meeting.right_score > meeting.left_score;
    let (left_score, right_score) = if played {
        (
            format_score(meeting.left_score),
            format_score(meeting.right_score),
        )
    } else {
        (String::from("–"), String::from("–"))
    };

    // Each side links to the game *that* player had white in, which is the whole
    // point of merging a two-game match into one row: the pairing is one contest,
    // but there are two games behind it and a name is how you pick one.
    let link = |game: Option<&GameId>| {
        game.map(|game| format!("/game/{game}"))
            .unwrap_or_else(|| String::from("#"))
    };

    view! {
        <div class=outer>
            // Hoverable from here too, so a player's path can be picked up from a
            // pairing rather than only from the table on the left.
            <a
                href=link(meeting.game_for(true))
                class=side(left, left_ahead, true)
                title=meeting.tooltip(true)
                on:mouseenter=move |_| highlighted.set(Some(left))
            >
                <span class="truncate">{name_of(left)}</span>
                <span class="tabular-nums">{left_score}</span>
            </a>
            <a
                href=link(meeting.game_for(false))
                class=side(right, right_ahead, false)
                title=meeting.tooltip(false)
                on:mouseenter=move |_| highlighted.set(Some(right))
            >
                <span class="truncate">{name_of(right)}</span>
                <span class="tabular-nums">{right_score}</span>
            </a>
        </div>
    }
}

/// Halves rendered the way a scoretable does, so 1.5 reads as `1½`.
fn format_score(score: f32) -> String {
    let whole = score.floor() as i32;
    let has_half = score - whole as f32 >= 0.25;
    match (whole, has_half) {
        (0, true) => String::from("½"),
        (whole, true) => format!("{whole}½"),
        (whole, false) => whole.to_string(),
    }
}
