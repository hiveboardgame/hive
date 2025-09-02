use crate::components::molecules::score_row::ScoreRow;
use crate::i18n::*;
use crate::responses::TournamentResponse;
use leptos::ev;
use leptos::html::Div;
use leptos::prelude::*;
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
                    class="flex items-center justify-center gap-1 cursor-pointer flex-wrap xs:flex-nowrap text-center w-full"
                    on:click=toggle_tooltip
                    title="Click for explanation"
                    attr:aria-expanded=move || is_open.get().to_string()
                >
                    <span class="hover:cursor-help whitespace-normal xs:whitespace-nowrap" title=explanation.clone()>
                        {tiebreaker.pretty_str().to_owned()}
                    </span>
                    <div class="hidden xs:inline-flex items-center justify-center w-4 h-4 sm:w-5 sm:h-5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                        <Icon icon=icondata_bi::BiInfoCircleRegular attr:class="w-5 h-5" />
                    </div>
                </button>
                <Show when=move || is_open.get()>
                    <div
                        class="absolute left-1/2 top-full -translate-x-1/2 z-50 mt-1 p-2 text-xs font-normal normal-case text-left bg-white text-gray-900 border border-gray-200 rounded-lg shadow-lg dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 whitespace-normal break-words w-fit max-w-[18rem] text-wrap"
                    >
                        <div class="relative">
                            {explanation()}
                            <div class="absolute left-1/2 -top-1 -translate-x-1/2 w-2 h-2 bg-white border-l border-t border-gray-200 dark:bg-gray-700 dark:border-gray-600 rotate-45"></div>
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
            <table class="m-2 table-auto w-full sm:w-auto h-fit">
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
