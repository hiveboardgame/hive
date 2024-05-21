use leptos::*;

use crate::{components::molecules::user_row::UserRow, providers::online_users::OnlineUsersSignal};

#[component]
pub fn OnlineUsers() -> impl IntoView {
    let online_users = expect_context::<OnlineUsersSignal>();
    let online_players = move || (online_users.signal)().username_user;
    let total_online = move || online_players().len();
    let search = create_rw_signal("".to_string());
    view! {
        <div class="flex flex-col pt-10 m-2 w-fit">
            <input
                type="text"
                on:input=move |ev| {
                    search.set(event_target_value(&ev));
                }

                placeholder="Search for a player"
                prop:value=search
            />

            {total_online}
            Online players:
            <div class="overflow-y-auto max-h-96 h-fit">
                <div class=move || {
                    format!("p-1 h-6 {}", if total_online() > 0 { "hidden" } else { "flex" })
                }>{move || if total_online() == 0 { "Only you" } else { "" }}</div>

                <For
                    each=online_players
                    key=move |(key, _)| (key.to_owned(), search())
                    let:a_user
                    children=move |a_user| {
                        if search().is_empty()
                            || a_user.1.username.to_lowercase().contains(&search().to_lowercase())
                        {
                            view! { <UserRow user=store_value(a_user.1)/> }
                        } else {
                            "".into_view()
                        }
                    }
                />

            </div>
        </div>
    }
}
