use crate::{
    common::UserAction, components::molecules::user_row::UserRow, functions::users::search_users,
    responses::TournamentResponse,
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::{collections::BTreeMap, time::Duration};

#[component]
pub fn InviteUser(tournament: TournamentResponse) -> impl IntoView {
    let pattern = RwSignal::new(String::new());
    let user_search = Resource::new(pattern, async move |pattern| {
        if pattern.is_empty() {
            BTreeMap::new()
        } else {
            let user_search = search_users(pattern).await;
            let mut btree = BTreeMap::new();
            for user in user_search.unwrap_or_default() {
                btree.insert(user.username.clone(), user);
            }
            btree
        }
    });
    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
    });
    let users = move || {
        let mut search_results = user_search.get().unwrap_or_default();
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
