use crate::components::molecules::score_row::ScoreRow;
use crate::responses::TournamentResponse;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::ev;
use leptos::html::Div;
use leptos_icons::*;
use leptos_use::on_click_outside;
use shared_types::Tiebreaker;

const TH_CLASS: &str = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase";



#[component]
fn TiebreakerHeader(tiebreaker: Tiebreaker, open_tooltip: RwSignal<Option<Tiebreaker>>) -> impl IntoView {
    let i18n = use_i18n();
    let stored_tiebreaker = StoredValue::new(tiebreaker.clone());
    let tooltip_ref = NodeRef::<Div>::new();
    
    let is_open = move || {
        stored_tiebreaker.with_value(|tb| open_tooltip.get() == Some(tb.clone()))
    };
    
    let toggle_tooltip = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        stored_tiebreaker.with_value(|tb| {
            let current_tb = tb.clone();
            open_tooltip.update(|current| {
                *current = if *current == Some(current_tb.clone()) {
                    None
                } else {
                    Some(current_tb)
                };
            });
        });
    };
    
    let tiebreaker_clone = tiebreaker.clone();
    let explanation = move || match &tiebreaker_clone {
        Tiebreaker::RawPoints => t_string!(i18n, tournaments.tiebreakers.raw_points),
        Tiebreaker::HeadToHead => t_string!(i18n, tournaments.tiebreakers.head_to_head),
        Tiebreaker::WinsAsBlack => t_string!(i18n, tournaments.tiebreakers.wins_as_black),
        Tiebreaker::SonnebornBerger => t_string!(i18n, tournaments.tiebreakers.sonneborn_berger),
    };
    
    let _ = on_click_outside(tooltip_ref, move |_| {
        stored_tiebreaker.with_value(|tb| {
            open_tooltip.update(|current| {
                if *current == Some(tb.clone()) {
                    *current = None;
                }
            });
        });
    });
    
                view! {
                <th class=TH_CLASS>
                    <div 
                        class="relative flex items-center justify-center gap-1 cursor-pointer"
                        on:click=toggle_tooltip
                        title="Click for explanation"
                    >
                        <span class="hover:cursor-help" title=explanation.clone()>
                            {tiebreaker.pretty_str().to_owned()}
                        </span>
                        <div class="inline-flex items-center justify-center w-6 h-6 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                            <Icon icon=icondata_bi::BiInfoCircleRegular attr:class="w-5 h-5" />
                        </div>
                <Show when=is_open>
                    <div 
                        node_ref=tooltip_ref
                        class="absolute top-full left-1/2 transform -translate-x-1/2 mt-1 z-50 w-64 p-2 text-xs font-normal normal-case text-left bg-white text-gray-900 border border-gray-200 rounded-lg shadow-lg dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600"
                    >
                        <div class="relative">
                            {explanation()}
                            <div class="absolute -top-1 left-1/2 transform -translate-x-1/2 w-2 h-2 bg-white border-l border-t border-gray-200 dark:bg-gray-700 dark:border-gray-600 rotate-45"></div>
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
    let open_tooltip = RwSignal::new(None::<Tiebreaker>);
    
    let tiebreakers_view = tiebreakers
        .iter()
        .map(|tiebreaker| {
            view! { 
                <TiebreakerHeader tiebreaker=tiebreaker.clone() open_tooltip />
            }
        })
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
            <table class="m-2 table-fixed max-w-fit h-fit">
                <thead>
                    <tr>
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
