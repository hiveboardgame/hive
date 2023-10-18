use crate::components::molecules::display_challenge::DisplayChallenge;
use crate::functions::{challenges::get::get_challenge_by_nanoid, hostname::hostname_and_port};
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
    let auth_context = expect_context::<AuthContext>();
    // id: || -> usize
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };

    let challenge = Resource::once(move || get_challenge_by_nanoid(nanoid()));

    view! {
        <div>
            <Transition>
                {move || {
                    challenge
                        .get()
                        .map(|data| match data {
                            Err(_) => {
                                view! { <pre>"Challenge doesn't seem to exist"</pre> }.into_view()
                            }
                            Ok(challenge) => {
                                let user = move || match auth_context.user.get() {
                                    Some(Ok(Some(user))) => Some(user),
                                    _ => None,
                                };
                                view! {
                                    <Show when=move || {
                                        if user().is_some() {
                                            user().expect("there to be a user").id
                                                == challenge.challenger.uid
                                        } else {
                                            false
                                        }
                                    }>

                                        <p>"To invite someone to play, give this URL:"</p>
                                        <a href=format!(
                                            "/challenge/{}",
                                            nanoid(),
                                        )>
                                            {move || {
                                                format!("{}/challenge/{}", hostname_and_port(), nanoid())
                                            }}

                                        </a>
                                    </Show>
                                    <DisplayChallenge challenge=challenge/>
                                }
                                    .into_view()
                            }
                        })
                }}

            </Transition>
        </div>
    }
}

