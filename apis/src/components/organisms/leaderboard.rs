use crate::{components::molecules::user_row::UserRow, functions::users::get::get_top_users};
use leptos::*;
use shared_types::game_speed::GameSpeed;
use leptos::logging::log;

#[component]
pub fn Leaderboard(speed: GameSpeed) -> impl IntoView {
    let top_users = Resource::once(move || get_top_users(speed.clone(), 10));
    view! {
        <Transition>
            {move || {
                top_users()
                    .map(|data| match data {
                        Err(e) => {
                            log!("Error is: {:?}", e);
                            view! { <pre class="m-2 h-6">"Couldn't fetch top users"</pre> }
                                .into_view()
                        }
                        Ok(users) => {
                            let users = store_value(users);
                            view! {
                                <div class="m-2 flex flex-col w-fit">
                                    "Leaderboard:" <div>
                                        <For
                                            each=move || { users() }

                                            key=|users| (users.uid)
                                            let:user
                                        >
                                            <UserRow
                                                user=store_value(user)
                                            />
                                        </For>
                                    </div>
                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}
