use crate::{
    components::organisms::display_profile::DisplayProfile,
    functions::users::get::get_user_by_username,
};
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[component]
pub fn ProfileView() -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let username = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.username.clone())
                .unwrap_or_default()
        })
    };

    let user = Resource::new(username, move |_| get_user_by_username(username()));

    view! {
        <div class="h-full w-full bg-white dark:bg-gray-900 mt-6">
            <Transition>
                {move || {
                    user()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                            Ok(user) => {
                                view! { <DisplayProfile user=store_value(user)/> }
                            }
                        })
                }}

            </Transition>

        </div>
    }
}

