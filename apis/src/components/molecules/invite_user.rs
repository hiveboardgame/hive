use crate::{
    common::UserAction,
    components::molecules::user_row::UserRow,
    providers::{user_search::UserSearchSignal, ApiRequestsProvider},
    responses::TournamentResponse,
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::time::Duration;

#[component]
pub fn InviteUser(tournament: TournamentResponse) -> impl IntoView {
    let user_search = expect_context::<UserSearchSignal>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let pattern = RwSignal::new(String::new());
    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
        if pattern().is_empty() {
            user_search.signal.update(|s| s.clear());
        } else {
            let api = api.get();
            api.search_user(pattern());
        }
    });
    let users = move || {
        if pattern().is_empty() {
            user_search.signal.update(|s| s.clear());
        }
        let mut search_results = user_search.signal.get();
        for (_, user) in tournament.players.iter() {
            search_results.remove(&user.username);
        }
        for user in tournament.invitees.iter() {
            search_results.remove(&user.username);
        }
        search_results
    };
    view! {
        <div class="flex flex-col justify-center items-center m-2 w-fit">
            <input
                class="p-1 w-64"
                type="text"
                on:input=debounced_search
                placeholder="Invite player"
                prop:value=pattern
                maxlength="20"
            />
            <div class="overflow-y-auto max-h-96">
                <For each=users key=move |(_, user)| user.uid let:user>
                    <UserRow
                        actions=vec![UserAction::Invite(tournament.tournament_id.clone())]
                        user=StoredValue::new(user.1)
                    />
                </For>

            </div>
        </div>
    }
}
