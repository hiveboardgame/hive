use crate::components::molecules::score_row::ScoreRow;
use crate::responses::TournamentResponse;
use leptos::prelude::*;

const TH_CLASS: &str = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase";

#[component]
pub fn Standings(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let tiebreakers = tournament.with_untracked(|t| t.tiebreakers.clone());
    let tiebreakers_view = tiebreakers
        .iter()
        .map(|tiebreaker| {
            view! { <th class=TH_CLASS>{tiebreaker.pretty_str().to_owned()}</th> }
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
    }
}
