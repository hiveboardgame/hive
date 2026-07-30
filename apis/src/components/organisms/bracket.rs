use crate::{
    components::molecules::panel::Panel,
    responses::{GameResponse, TournamentResponse},
};
use leptos::prelude::*;
use shared_types::TournamentGameResult;
use std::collections::BTreeMap;
use uuid::Uuid;

/// One matchup: two players and how many games each won.
///
/// A bracket matchup is a two-game match, replayed until it is decisive, so a
/// score of 1-1 means "still being settled" rather than a drawn result.
#[derive(Clone)]
struct Matchup {
    left: Uuid,
    right: Uuid,
    left_wins: u32,
    right_wins: u32,
    decided: bool,
}

fn unordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Groups a tournament's games into rounds of matchups.
///
/// Derived from the games rather than from `Standings`, which only carries
/// finishing tiers — the pairing structure a bracket diagram needs lives in
/// `round` plus the two player ids.
fn rounds_of(games: &[GameResponse]) -> Vec<(i32, Vec<Matchup>)> {
    let mut rounds: BTreeMap<i32, BTreeMap<(Uuid, Uuid), Matchup>> = BTreeMap::new();

    for game in games {
        let Some(round) = game.round else {
            continue;
        };
        let left = game.white_player.uid;
        let right = game.black_player.uid;
        let pair = unordered(left, right);
        let matchup = rounds
            .entry(round)
            .or_default()
            .entry(pair)
            .or_insert_with(|| Matchup {
                left: pair.0,
                right: pair.1,
                left_wins: 0,
                right_wins: 0,
                decided: false,
            });

        let winner = match game.tournament_game_result {
            TournamentGameResult::Winner(hive_lib::Color::White) => Some(left),
            TournamentGameResult::Winner(hive_lib::Color::Black) => Some(right),
            _ => None,
        };
        match winner {
            Some(winner) if winner == matchup.left => matchup.left_wins += 1,
            Some(_) => matchup.right_wins += 1,
            None => {}
        }
        if game.finished {
            matchup.decided = matchup.left_wins != matchup.right_wins;
        }
    }

    rounds
        .into_iter()
        .map(|(round, matchups)| (round, matchups.into_values().collect()))
        .collect()
}

/// Rounds counted from the end, the way brackets are actually named.
fn round_name(index: usize, total: usize) -> String {
    match total.saturating_sub(index) {
        1 => String::from("Final"),
        2 => String::from("Semifinals"),
        3 => String::from("Quarterfinals"),
        _ => format!("Round {}", index + 1),
    }
}

#[component]
pub fn Bracket(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let rounds = move || tournament.with(|t| rounds_of(&t.games));
    let name_of = move |player: Uuid| {
        tournament.with(|t| {
            t.players
                .get(&player)
                .map(|user| user.username.clone())
                .unwrap_or_else(|| String::from("—"))
        })
    };

    view! {
        <Panel title="Results" class="min-w-0">
            <div class="overflow-x-auto">
                <div class="flex gap-4 items-start min-w-fit">
                    {move || {
                        let rounds = rounds();
                        let total = rounds.len();
                        rounds
                            .into_iter()
                            .enumerate()
                            .map(|(index, (_, matchups))| {
                                view! {
                                    <div class="flex flex-col gap-2 min-w-44">
                                        <h3 class="text-xs font-bold tracking-tight text-gray-700 uppercase dark:text-gray-300">
                                            {round_name(index, total)}
                                        </h3>
                                        {matchups
                                            .into_iter()
                                            .map(|matchup| {
                                                view! { <MatchupBox matchup name_of=name_of.clone() /> }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </div>
        </Panel>
    }
}

#[component]
fn MatchupBox(
    matchup: Matchup,
    name_of: impl Fn(Uuid) -> String + Clone + 'static,
) -> impl IntoView {
    // The winner is only emphasised once the match is actually settled — a 1-1
    // bracket match is replayed, not drawn.
    let left_won = matchup.decided && matchup.left_wins > matchup.right_wins;
    let right_won = matchup.decided && matchup.right_wins > matchup.left_wins;
    let side = |won: bool| {
        if won {
            "flex justify-between gap-2 px-2 py-1 font-bold text-gray-900 dark:text-gray-100"
        } else {
            "flex justify-between gap-2 px-2 py-1 text-gray-600 dark:text-gray-400"
        }
    };

    view! {
        <div class="text-sm rounded border border-gray-300 divide-y divide-gray-300 dark:border-gray-700 dark:divide-gray-700">
            <div class=side(left_won)>
                <span class="truncate">{name_of(matchup.left)}</span>
                <span>{matchup.left_wins}</span>
            </div>
            <div class=side(right_won)>
                <span class="truncate">{name_of(matchup.right)}</span>
                <span>{matchup.right_wins}</span>
            </div>
        </div>
    }
}
