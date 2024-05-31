use crate::components::molecules::user_row::UserRow;
use crate::providers::{
    navigation_controller::NavigationControllerSignal, tournaments::TournamentStateSignal,
};
use leptos::*;

#[component]
pub fn Tournament() -> impl IntoView {
    let navi = expect_context::<NavigationControllerSignal>();
    let tournaments = expect_context::<TournamentStateSignal>();
    let current_tournament = move || {
        navi.tournament_signal
            .get()
            .nanoid
            .and_then(|tournament_id| {
                tournaments
                    .signal
                    .get()
                    .tournaments
                    .get(&tournament_id)
                    .cloned()
            })
    };
    let display_tournament = move || {
        current_tournament().and_then(|tournament| {
            view! {
                <div>{tournament.name}</div>
                <div>{tournament.description}</div>
                <div>{tournament.scoring}</div>
                <div>
                    <For
                        each=move || { tournament.organizers.clone() }

                        key=|users| (users.uid)
                        let:user
                    >
                        <UserRow user=store_value(user)/>
                    </For>
                </div>
                <div>{tournament.seats}</div>
                <div>{tournament.rounds}</div>
                <div>{tournament.time_mode}</div>
            }
            .into()
        })
    };
    view! { <div class="flex flex-col pt-10">{display_tournament}</div> }
}
