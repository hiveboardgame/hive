use crate::{
    common::UserAction,
    components::molecules::user_row::UserRow,
    providers::{user_search::UserSearchSignal, ApiRequests},
};
use leptos::ev::Event;
use leptos::leptos_dom::helpers::debounce;
use leptos::*;
use std::time::Duration;
use uuid::Uuid;

#[component]
pub fn InviteUser(tournament: String) -> impl IntoView {
    let user_search = expect_context::<UserSearchSignal>();
    let pattern = RwSignal::new(String::new());
    let debounced_search = debounce(Duration::from_millis(100), move |ev: Event| {
        pattern.set(event_target_value(&ev));
        if pattern().is_empty() {
            user_search.signal.update(|s| s.clear());
        } else {
            let api = ApiRequests::new();
            api.search_user(pattern());
        }
    });
    let users = move || {
        if pattern().is_empty() {
            user_search.signal.update(|s| s.clear());
        }
        user_search.signal.get()
    };
    view! {
        <div class="flex flex-col m-2 w-fit">
            <input
                class="p-1 w-64"
                type="text"
                on:input=debounced_search
                placeholder="Invite player"
                prop:value=pattern
                attr:maxlength="20"
            />
            <div class="overflow-y-auto h-96">
                <For each=users key=move |(_, user)| user.uid let:user>
                    <UserRow actions=vec![UserAction::Invite(tournament.clone())] user=store_value(user.1)/>
                </For>

            </div>
        </div>
    }
}
