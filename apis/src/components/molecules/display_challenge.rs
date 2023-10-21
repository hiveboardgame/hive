use crate::{
    functions::challenges::{
        accept::AcceptChallenge, challenge_response::ChallengeResponse, delete::DeleteChallenge,
    },
    providers::{auth_context::AuthContext, queries::use_challenge_query},
};
use leptos::*;
use leptos_query::use_query_client;
use leptos_router::*;

#[component]
pub fn DisplayChallenge(challenge: ChallengeResponse) -> impl IntoView {
    let client = store_value(use_query_client());
    let onsubmit = move |_| {
        client.with_value(|client| {
            client.invalidate_query::<(), Result<Vec<ChallengeResponse>, ServerFnError>>(())
        });
    };
    let accept_challenge = create_server_action::<AcceptChallenge>();
    let delete_challenge = create_server_action::<DeleteChallenge>();
    let auth_context = expect_context::<AuthContext>();
    let stored_challenge = store_value(challenge.clone());
    let challenge_string = format!(
        "{} rated:{} is looking for a {} game!",
        challenge.challenger.username, challenge.challenger.rating, challenge.game_type
    );
    let own_challenge_string = format!("You are looking for a {} game!", challenge.game_type);

    view! {
        <Show
            when=move || {
                let user = move || match auth_context.user.get() {
                    Some(Ok(Some(user))) => Some(user),
                    _ => None,
                };
                if user().is_some() {
                    user().expect("there to be a user").id != challenge.challenger.uid
                } else {
                    true
                }
            }

            fallback=move || {
                view! {
                    <div class="flex items-center">
                        <p>{&own_challenge_string}</p>
                        <ActionForm action=delete_challenge on:submit=onsubmit>
                            <input type="hidden" name="id" value=stored_challenge().id.to_string()/>
                            <input
                                type="submit"
                                value="Delete"
                                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                            />
                        </ActionForm>
                    </div>
                }
            }
        >

            <div class="flex items-center">
                <p>{&challenge_string}</p>
                <ActionForm action=accept_challenge>
                    <input type="hidden" name="nanoid" value=stored_challenge().nanoid/>
                    <input
                        type="submit"
                        value="Join"
                        class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                    />
                </ActionForm>
            </div>
        </Show>
    }
}

