use crate::common::UserAction;
use crate::components::atoms::rating::icon_for_speed;
use crate::{components::molecules::user_row::UserRow, functions::users::get::get_top_users};
use leptos::logging::log;
use leptos::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn Leaderboard(speed: GameSpeed) -> impl IntoView {
    let speed = store_value(speed);
    let top_users = Resource::once(move || get_top_users(speed(), 10));
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
                            let is_empty = move || users().is_empty();
                            view! {
                                <div class="flex flex-col m-2 w-fit">
                                    <div class="flex gap-1 items-center">
                                        <Icon icon=icon_for_speed(&speed())/>
                                        {speed().to_string()}
                                        :
                                    </div>
                                    <div class=move || {
                                        format!(
                                            "p-1 h-6 {}",
                                            if !is_empty() { "hidden" } else { "flex" },
                                        )
                                    }>{move || if is_empty() { "No one yet" } else { "" }}</div>
                                    <div>
                                        <For
                                            each=move || { users() }

                                            key=|users| (users.uid)
                                            let:user
                                        >
                                            <UserRow
                                                actions=vec![UserAction::Challenge]
                                                user=store_value(user)
                                                game_speed=speed
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
