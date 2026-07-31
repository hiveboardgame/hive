use crate::{components::molecules::game_previews::GamePreviews, responses::GameResponse};
use leptos::{either::Either, prelude::*};
use std::collections::BTreeMap;

/// A tournament's games split into rounds, most recent first.
///
/// One long list is unreadable past a couple of rounds: a 16-player Swiss ends up
/// with 40 games in no meaningful order, and the question somebody actually has
/// is "what happened in round 3". Games with no round — an arena pairs on a clock
/// rather than in rounds — fall back to a single flat list.
#[component]
pub fn GamesByRound(#[prop(into)] games: Signal<Vec<GameResponse>>) -> impl IntoView {
    let rounds = move || {
        let mut rounds: BTreeMap<i32, Vec<GameResponse>> = BTreeMap::new();
        let mut roundless = Vec::new();
        for game in games.get() {
            match game.round {
                Some(round) => rounds.entry(round).or_default().push(game),
                None => roundless.push(game),
            }
        }
        // Descending: the round in progress is the one being looked for.
        let ordered: Vec<(i32, Vec<GameResponse>)> = rounds.into_iter().rev().collect();
        (ordered, roundless)
    };

    view! {
        {move || {
            let (ordered, roundless) = rounds();
            if ordered.is_empty() {
                return Either::Left(view! { <GamePreviews games=roundless /> });
            }
            Either::Right(
                view! {
                    <div class="space-y-4">
                        {ordered
                            .into_iter()
                            .map(|(round, games)| {
                                view! {
                                    <div class="space-y-2">
                                        <h3 class="text-xs font-bold tracking-tight text-gray-700 uppercase dark:text-gray-300">
                                            {format!("Round {round}")}
                                        </h3>
                                        <GamePreviews games />
                                    </div>
                                }
                            })
                            .collect_view()}
                        {(!roundless.is_empty())
                            .then(|| {
                                view! {
                                    <div class="space-y-2">
                                        <h3 class="text-xs font-bold tracking-tight text-gray-700 uppercase dark:text-gray-300">
                                            "Other games"
                                        </h3>
                                        <GamePreviews games=roundless />
                                    </div>
                                }
                            })}
                    </div>
                },
            )
        }}
    }
}
