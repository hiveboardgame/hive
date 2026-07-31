use crate::{
    common::tiebreaker_explanation,
    components::molecules::{dropdown_panel::DropdownPanel, panel::Panel, score_row::ScoreRow},
    i18n::*,
    responses::TournamentResponse,
};
use leptos::{ev, html::Div, prelude::*};
use leptos_icons::*;
use leptos_use::on_click_outside;
use shared_types::{GameSpeed, Tiebreaker};
use std::collections::HashMap;
use uuid::Uuid;

const TH_CLASS: &str = "py-1 px-1 md:py-2 md:px-2 font-bold uppercase leading-tight tracking-tight text-[10px] xs:text-xs text-gray-700 dark:text-gray-300";

#[component]
fn TiebreakerHeader(tiebreaker: Tiebreaker) -> impl IntoView {
    let i18n = use_i18n();
    let container_ref = NodeRef::<Div>::new();

    let is_open = RwSignal::new(false);

    let toggle_tooltip = move |_: ev::MouseEvent| {
        is_open.update(|o| *o = !*o);
    };

    let explanation = Signal::derive(move || tiebreaker_explanation(i18n, tiebreaker));

    let _ = on_click_outside(container_ref, move |_| {
        is_open.set(false);
    });

    view! {
        <th class=TH_CLASS>
            <div node_ref=container_ref class="relative">
                <button
                    type="button"
                    // `min-h-6` keeps the header a 24px touch target — WCAG
                    // 2.5.8's floor — which 10px header text alone falls under.
                    class="flex flex-wrap gap-1 justify-center items-center w-full text-center cursor-pointer min-h-6 xs:flex-nowrap"
                    on:click=toggle_tooltip
                    title="Click for explanation"
                    attr:aria-expanded=move || is_open.get().to_string()
                >
                    <span
                        class="whitespace-normal xs:whitespace-nowrap hover:cursor-help"
                        title=explanation
                    >
                        {tiebreaker.pretty_str().to_owned()}
                    </span>
                    <div class="hidden justify-center items-center w-4 h-4 text-gray-500 sm:w-5 sm:h-5 dark:text-gray-400 hover:text-gray-700 xs:inline-flex dark:hover:text-gray-200">
                        <Icon icon=icondata_bi::BiInfoCircleRegular attr:class="w-5 h-5" />
                    </div>
                </button>
                <Show when=is_open>
                    // `w-max` rather than `w-fit`: an absolutely positioned box
                    // fits to its containing block, which here is a narrow table
                    // header, so `w-fit` wrapped a short phrase onto three lines.
                    // Max-content instead, still capped so a long explanation wraps.
                    <DropdownPanel class="absolute left-1/2 top-full z-50 p-2 mt-1 w-max text-xs font-normal text-left text-gray-900 normal-case -translate-x-1/2 dark:text-gray-200 max-w-[18rem] text-wrap">
                        <div class="relative">{explanation}</div>
                    </DropdownPanel>
                </Show>
            </div>
        </th>
    }
}

#[component]
pub fn Standings(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let tiebreakers = tournament.with_untracked(|t| t.tiebreakers.clone());

    let tiebreakers_view = tiebreakers
        .iter()
        .map(|tiebreaker| view! { <TiebreakerHeader tiebreaker=tiebreaker.clone() /> })
        .collect_view();

    // `groups` is best-first, each inner group a tie the tiebreakers could not
    // split. Flattened for rendering; the shared `position` is what shows the
    // tie.
    let standings_data = move || {
        tournament.with(|t| {
            t.standings
                .groups
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let players_map = tournament.with_untracked(|t| t.players.clone());
    // Ratings are per speed, and the tournament's own speed is the only one that
    // says anything about the field these players are in.
    let speed = tournament
        .with_untracked(|t| GameSpeed::from_base_increment(t.time_base, t.time_increment));
    let withdrawn = tournament.with_untracked(|t| t.withdrawn.clone());

    // The rating each player carried in this tournament, not the one they have
    // today: a finished event read months later would otherwise show ratings that
    // have nothing to do with the field that played it. Taken from the last game
    // they played here — `white_rating` is the rating going in, so the change is
    // added back to give where they stood when it ended.
    let ratings_here = tournament.with_untracked(|t| {
        let mut latest: HashMap<Uuid, (i32, f64)> = HashMap::new();
        for game in &t.games {
            let round = game.round.unwrap_or(0);
            for (player, rating, change) in [
                (
                    game.white_player.uid,
                    game.white_rating,
                    game.white_rating_change,
                ),
                (
                    game.black_player.uid,
                    game.black_rating,
                    game.black_rating_change,
                ),
            ] {
                let Some(rating) = rating else { continue };
                let after = rating + change.unwrap_or(0.0);
                latest
                    .entry(player)
                    .and_modify(|held| {
                        if round >= held.0 {
                            *held = (round, after);
                        }
                    })
                    .or_insert((round, after));
            }
        }
        latest
            .into_iter()
            .map(|(player, (_, rating))| (player, rating.round() as u64))
            .collect::<HashMap<Uuid, u64>>()
    });

    view! {
        <div class="w-full min-w-0">
            <Panel title="Standings" class="min-w-0" body_class="overflow-auto">
                <table class="w-full table-auto h-fit">
                    <thead>
                        <tr class="[&>th:nth-child(4)]:pl-2 sm:[&>th:nth-child(4)]:pl-3">
                            <th class=TH_CLASS>Pos</th>
                            <th class=TH_CLASS>Player</th>
                            <th class=TH_CLASS>Elo</th>
                            {tiebreakers_view}
                            <th class=TH_CLASS>Finished</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=standings_data
                            key=|standing| (
                                standing.player,
                                standing.position,
                                standing.games_played,
                                standing.scores.values().sum::<f32>() as i64,
                            )
                            let:standing
                        >

                            {players_map
                                .get(&standing.player)
                                .cloned()
                                .map(|user| {
                                    let rating = ratings_here
                                        .get(&standing.player)
                                        .copied()
                                        .or_else(|| {
                                            user.ratings.get(&speed).map(|rating| rating.rating)
                                        });
                                    // Falls back to the live rating only for a
                                    // player with no game here yet.
                                    view! {
                                        <ScoreRow
                                            user
                                            standing=standing.position.to_string()
                                            finished=standing.games_played
                                            tiebreakers=tiebreakers.clone()
                                            scores=standing.scores.clone()
                                            rating
                                            withdrawn=withdrawn.contains(&standing.player)
                                        />
                                    }
                                })}

                        </For>
                    </tbody>
                </table>
            </Panel>
        </div>
    }
}
