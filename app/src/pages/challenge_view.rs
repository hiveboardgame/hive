use crate::functions::challenges::{accept::AcceptChallenge, get::get_challenge_by_nanoid};
use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq)]
struct ChallengeParams {
    nanoid: String,
}

#[component]
pub fn ChallengeView() -> impl IntoView {
    let params = use_params::<ChallengeParams>();

    // id: || -> usize
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };

    let challenge = create_resource(move || nanoid(), get_challenge_by_nanoid);
    let accept_challenge = create_server_action::<AcceptChallenge>();

    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");

    // TODO this needs a transition
    if let Some(Ok(user)) = auth_context.user.get() {
        view! {
            <div>
                <p> nanoid: { nanoid() } </p>
                <p> user: { user.username } </p>
             </div>
            <div>
                <Transition
                    fallback=move || view! { <p>"Loading..."</p> }
                >
                    {move || {
                        challenge.get().map(|data| match data {
                            Err(_) => view! { <pre>"Error"</pre> }.into_view(),
                            Ok(challenge) =>
                                // if challenge.challenger.uuid == {}
                                // TODO make this a delete action if it's the challenge owner
                                view! {
                                    <p> { challenge.challenger.username } rating:{ challenge.challenger.rating } wants to play a {challenge.game_type } game with you! Do you accept? </p>
                                    <div>
                                        <ActionForm action=accept_challenge>
                                            <input
                                                type="hidden"
                                                name="nanoid"
                                                value={nanoid()}
                                            />

                                            <input
                                                type="submit"
                                                value="Accept challenge"
                                                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                                            />
                                        </ActionForm>
                                    </div>
                                }.into_view(),
                        })
                    }}
                </Transition>
            </div>
        }.into_view()
    } else {
        if let Some(_) = auth_context.user.get() {
            view! { Some }.into_view()
        } else  {
            view! { None }.into_view()
        }
    }
}
