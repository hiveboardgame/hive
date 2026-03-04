use crate::{components::molecules::score_row::ScoreRow, i18n::*, responses::TournamentResponse};
use leptos::{ev, html::Div, prelude::*};
use leptos_icons::*;
use leptos_use::on_click_outside;
use shared_types::Tiebreaker;

const TH_CLASS: &str = "py-1 px-1 md:py-2 md:px-2 font-bold uppercase leading-tight tracking-tight text-[10px] xs:text-xs";

#[component]
fn TiebreakerHeader(tiebreaker: Tiebreaker) -> impl IntoView {
    let i18n = use_i18n();
    let container_ref = NodeRef::<Div>::new();

    let is_open = RwSignal::new(false);

    let toggle_tooltip = move |_: ev::MouseEvent| {
        is_open.update(|o| *o = !*o);
    };

    let tiebreaker_clone = tiebreaker.clone();
    let explanation = move || match &tiebreaker_clone {
        Tiebreaker::RawPoints => t_string!(i18n, tournaments.tiebreakers.raw_points),
        Tiebreaker::HeadToHead => t_string!(i18n, tournaments.tiebreakers.head_to_head),
        Tiebreaker::WinsAsBlack => t_string!(i18n, tournaments.tiebreakers.wins_as_black),
        Tiebreaker::SonnebornBerger => t_string!(i18n, tournaments.tiebreakers.sonneborn_berger),
    };

    let _ = on_click_outside(container_ref, move |_| {
        is_open.set(false);
    });

    view! {
        <th class=TH_CLASS>
            <div node_ref=container_ref class="relative">
                <button
                    type="button"
                    class="flex flex-wrap gap-1 justify-center items-center w-full text-center cursor-pointer xs:flex-nowrap"
                    on:click=toggle_tooltip
                    title="Click for explanation"
                    attr:aria-expanded=move || is_open.get().to_string()
                >
                    <span
                        class="whitespace-normal xs:whitespace-nowrap hover:cursor-help"
                        title=explanation.clone()
                    >
                        {tiebreaker.pretty_str().to_owned()}
                    </span>
                    <div class="hidden justify-center items-center w-4 h-4 text-gray-500 sm:w-5 sm:h-5 dark:text-gray-400 hover:text-gray-700 xs:inline-flex dark:hover:text-gray-200">
                        <Icon icon=icondata_bi::BiInfoCircleRegular attr:class="w-5 h-5" />
                    </div>
                </button>
                <Show when=move || is_open.get()>
                    <div class="absolute left-1/2 top-full z-50 p-2 mt-1 text-xs font-normal text-left text-gray-900 normal-case whitespace-normal break-words bg-white rounded-lg border border-gray-200 shadow-lg -translate-x-1/2 dark:text-gray-200 dark:bg-gray-700 dark:border-gray-600 w-fit max-w-[18rem] text-wrap">
                        <div class="relative">
                            {explanation()}
                            <div class="absolute -top-1 left-1/2 w-2 h-2 bg-white border-t border-l border-gray-200 rotate-45 -translate-x-1/2 dark:bg-gray-700 dark:border-gray-600"></div>
                        </div>
                    </div>
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

    let standings_data = move || {
        tournament.with(|t| {
            t.standings
                .results()
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
    };
    let players_map = tournament.with_untracked(|t| t.players.clone());

    view! {
        <div class="relative">
            <table class="m-2 w-full table-auto sm:w-auto h-fit">
                <thead>
                    <tr class="[&>th:nth-child(3)]:pl-2 sm:[&>th:nth-child(3)]:pl-3">
                        <th class=TH_CLASS>Pos</th>
                        <th class=TH_CLASS>Player</th>
                        {tiebreakers_view}
                        <th class=TH_CLASS>Finished</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=standings_data
                        key=|(uuid, position, finished, hash)| (
                            *uuid,
                            position.clone(),
                            *finished,
                            hash.values().sum::<f32>() as i64,
                        )
                        let:player_at_position
                    >

                        {
                            let (uuid, position, finished, hash) = player_at_position;
                            let user = players_map.get(&uuid).expect("User in tournament").clone();

                            view! {
                                <ScoreRow
                                    user
                                    standing=position
                                    finished
                                    tiebreakers=tiebreakers.clone()
                                    scores=hash
                                />
                            }
                        }

                    </For>
                </tbody>
            </table>
        </div>
    }
}
