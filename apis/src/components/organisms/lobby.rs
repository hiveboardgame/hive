use crate::components::molecules::display_challenge::DisplayChallenge;
use crate::functions::challenges::get_public::get_public_challenges;
use leptos::*;

#[component]
pub fn Lobby(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let challenges = create_resource(move || (), move |_| get_public_challenges());
    view! {
        <div class=format!("{extend_tw_classes}")>
            <button
                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                on:click=move |_| challenges.refetch()
            >
                Reload challenges
            </button>
            <Transition fallback=move || ()>
                {move || {
                    let challenges = move || match challenges.get() {
                        Some(Ok(challenge)) => Some(challenge),
                        _ => None,
                    };
                    view! {
                        <Show when=move || challenges().is_some() fallback=|| {  }>
                            {if !challenges().unwrap().is_empty() {
                                view! {
                                    <For
                                        each=move || {
                                            challenges().expect("There to be Some challenge")
                                        }
                                        key=|challenge| (challenge.id)
                                        let:challenge
                                    >
                                        <DisplayChallenge challenge=challenge/>
                                    </For>
                                }
                                    .into_view()
                            } else {
                                view! {
                                    <p>
                                        Go create your own challenge, there are none at the moment
                                    </p>
                                }
                                    .into_view()
                            }}

                        </Show>
                    }
                }}

            </Transition>
        </div>
    }
}
