use crate::components::molecules::display_challenge::DisplayChallenge;
use crate::functions::challenges::get::get_challenge_by_nanoid;
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

    let challenge = create_resource(nanoid, get_challenge_by_nanoid);

    view! {
        <div>
            <Transition fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || {
                    challenge
                        .get()
                        .map(|data| match data {
                            Err(_) => view! { <pre>"Error"</pre> }.into_view(),
                            Ok(challenge) => {
                                view! {
                                    // if challenge.challenger.uuid == {}
                                    // TODO make this a delete action if it's the challenge owner
                                    <DisplayChallenge challenge=challenge/>
                                }
                            }
                        })
                }}

            </Transition>
        </div>
    }
}
