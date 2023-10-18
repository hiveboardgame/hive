use crate::{
    components::molecules::display_profile::DisplayProfile,
    functions::users::get::{get_user_by_username, get_user_games},
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

    let user = Resource::once(move || get_user_by_username(username()));
    let games = Resource::once(move || get_user_games(username()));

    view! {
        <div>
            <Transition>
                {move || {
                    user.get()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                            Ok(user) => {
                                view! { <DisplayProfile user=user/> }
                            }
                        })
                }}

            </Transition>
            <Transition>
                {move || {
                    let games = move || match games.get() {
                        Some(Ok(games)) => Some(games),
                        _ => None,
                    };
                    view! {
                        <Show when=move || {
                            games().is_some()
                        }>

                            {
                                view! {
                                    " "
                                    {games().unwrap().len()}
                                    <li>
                                        <For
                                            each=move || {
                                                games().expect("There to be Some challenge")
                                            }

                                            key=|game| (game.game_id)
                                            let:game
                                        >

                                            <ul>
                                                <a href=format!("/play/{}", game.nanoid)>{game.nanoid}</a>
                                            </ul>
                                        </For>
                                    </li>
                                }
                            }

                        </Show>
                    }
                }}

            </Transition>
        </div>
    }
}

