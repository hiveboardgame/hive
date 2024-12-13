use crate::components::molecules::score_row::ScoreRow;
use crate::responses::TournamentResponse;
use leptos::*;

const TH_CLASS: &str = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase";

#[component]
pub fn Standings(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let tiebreakers_view = tournament
        .get_untracked()
        .tiebreakers
        .iter()
        .map(|tiebreaker| {
            view! { <th class=TH_CLASS>{tiebreaker.pretty_str().to_owned()}</th> }
        })
        .collect_view();
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
                    each=move || tournament().standings.results().into_iter().flatten()
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
                        let uuid = store_value(uuid);
                        let user = store_value(
                            tournament().players.get(&uuid()).expect("User in tournament").clone(),
                        );
                        view! {
                            <ScoreRow
                                user=user
                                standing=position
                                finished
                                tiebreakers=tournament().tiebreakers
                                scores=hash
                            />
                        }
                    }

                </For>
            </tbody>
        </table>
    }
}
