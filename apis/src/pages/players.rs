use crate::{components::atoms::profile_link::ProfileLink,functions::users::get::get_top_users};
use leptos::*;

#[component]
pub fn PlayersView() -> impl IntoView {
    let top_users = Resource::once(move || get_top_users(10));
    let list_class =
        "flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light";
    view! {
        <Transition>
            {move || {
                top_users()
                    .map(|data| match data {
                        Err(_) => view! { <pre>"No top users"</pre> }.into_any(),
                        Ok(users) => {
                            let users = store_value(users);
                            view! {
                                <div class="m-2 flex flex-col w-fit">
                                    "Leaderboard" <ol>
                                        <For
                                            each=move || { users() }

                                            key=|users| (users.uid)
                                            let:user
                                        >
                                            <li class=list_class>
                                                <ProfileLink username=user.username />
                                                <p class="ml-2">{user.rating}</p>
                                            </li>
                                        </For>
                                    </ol>
                                </div>
                            }
                                .into_any()
                        }
                    })
            }}

        </Transition>
    }
}
