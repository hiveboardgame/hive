use crate::components::molecules::display_challenge::DisplayChallenge;
use crate::functions::challenges::get_public::get_public_challenges;
use leptos::*;

#[component]
pub fn Lobby(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let challenges = Resource::once(move || get_public_challenges());
    view! {
        <div class=format!("{extend_tw_classes}")>
            <Transition>
                {move || {
                    let challenges = move || match challenges() {
                        Some(Ok(challenges)) => Some(challenges),
                        _ => None,
                    };
                    view! {
                        <Show
                            when=move || {
                                challenges().is_some() && !challenges().unwrap().is_empty()
                            }

                            fallback=move || {
                                view! {
                                    <p>
                                        Go create your own challenge, there are none at the moment
                                    </p>
                                }
                            }
                        >

                            <For
                                each=move || { challenges().expect("There to be Some challenge") }

                                key=|challenge| (challenge.id)
                                let:challenge
                            >
                                <DisplayChallenge challenge=challenge/>
                            </For>

                        </Show>
                    }
                }}

            </Transition>
        </div>
    }
}

