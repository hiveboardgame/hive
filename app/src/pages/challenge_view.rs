use crate::functions::challenges::{accept::AcceptChallenge, get::get_challenge_by_url};
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
    let url = move || {
        move || {
            params.with(|params| {
                params
                    .as_ref()
                    .map(|params| params.nanoid.clone())
                    .unwrap_or_default()
            })
        }
    };

    let challenge = create_resource(move || url()(), get_challenge_by_url);
    let accept_challenge = create_server_action::<AcceptChallenge>();
    view! {
        <div>
            <p> here: { url() } </p>
         </div>
        <div>
            <Transition
                fallback=move || view! { <p>"Loading..."</p> }
            >
                {move || {
                    challenge.get().map(|data| match data {
                        Err(_) => view! { <pre>"Error"</pre> }.into_view(),
                        Ok(challenge) =>
                            view! {
                                <p> { challenge.challenger.username } rating:{ challenge.challenger.rating } wants to play a {challenge.game_type } game with you! Do you accept? </p>
                                <div>
                                    <ActionForm action=accept_challenge>
                                        <input
                                            type="hidden"
                                            name="url"
                                            value={url()}
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
    }
}
