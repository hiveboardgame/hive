use crate::components::molecules::score_row::ScoreRow;
use crate::responses::TournamentResponse;
use leptos::*;
use uuid::Uuid;

#[component]
pub fn Standings(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let th_class = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase";
    view! {
        <table class="m-2 table-fixed max-w-fit h-fit">
            <thead>
                <tr>
                    <th class=th_class>Pos</th>
                    <th class=th_class>Player</th>
                    {tournament()
                        .tiebreakers
                        .iter()
                        .map(|tiebreaker| {
                            view! { <th class=th_class>{tiebreaker.pretty_str().to_owned()}</th> }
                        })
                        .collect_view()}
                </tr>
            </thead>
            <tbody>
                <For
                    each=move || { tournament().standings.results().into_iter() }
                    key=|players_at_position| {
                        players_at_position.iter().map(|(uuid, _, _)| *uuid).collect::<Vec<Uuid>>()
                    }

                    let:players_at_position
                >

                    {
                        let players_at_position = store_value(players_at_position);
                        view! {
                            <For
                                each=players_at_position

                                key=|(uuid, _position, _hash)| (*uuid)
                                let:player
                            >

                                {
                                    let (uuid, position, hash) = player;
                                    let uuid = store_value(uuid);
                                    let user = store_value(
                                        tournament()
                                            .players
                                            .get(&uuid())
                                            .expect("User in tournament")
                                            .clone(),
                                    );
                                    view! {
                                        <ScoreRow
                                            user=user
                                            standing=position
                                            tiebreakers=tournament().tiebreakers
                                            scores=hash
                                        />
                                    }
                                }

                            </For>
                        }
                    }

                </For>
            </tbody>
        </table>
    }
}
