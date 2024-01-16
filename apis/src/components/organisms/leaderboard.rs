use crate::{components::molecules::user_row::UserRow, functions::users::get::get_top_users};
use leptos::*;

#[component]
pub fn Leaderboard() -> impl IntoView {
    let top_users = Resource::once(move || get_top_users(10));
    view! {
        <Transition>
            {move || {
                top_users()
                    .map(|data| match data {
                        Err(_) => view! { <pre>"No top users"</pre> }.into_view(),
                        Ok(users) => {
                            let users = store_value(users);
                            view! {
                                <div class="m-2 flex flex-col w-fit">
                                    "Leaderboard:" <ol>
                                        <For
                                            each=move || { users() }

                                            key=|users| (users.uid)
                                            let:user
                                        >
                                            <UserRow
                                                username=store_value(user.username)
                                                rating=user.rating
                                            />
                                        </For>
                                    </ol>
                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}
