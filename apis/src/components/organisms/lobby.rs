use crate::{
    components::molecules::display_challenge::DisplayChallenge,
    functions::challenges::challenge_response::ChallengeResponse,
};
use leptos::*;

#[component]
pub fn Lobby(challenges: StoredValue<Vec<ChallengeResponse>>) -> impl IntoView {
    let th_class = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh]";
    view! {
        <div class="flex col-span-full overflow-x-auto">
            <table class="grow md:grow-0 md:table-fixed">
                <thead>
                    <tr>
                        <th class=th_class></th>
                        <th class=th_class>Player</th>
                        <th class=th_class>Rating</th>
                        <th class=th_class>Expansions</th>
                        <th class=th_class>Time</th>
                        <th class=th_class>Mode</th>
                        <th class=th_class>Action</th>
                    </tr>
                </thead>
                <Show
                    when=move || challenges().is_empty()
                    fallback=move || {
                        view! {
                            <tbody>

                                <For
                                    each=move || { challenges() }

                                    key=|challenge| (challenge.id)
                                    let:challenge
                                >
                                    <DisplayChallenge
                                        challenge=store_value(challenge)
                                        single=false
                                    />
                                </For>

                            </tbody>
                        }
                    }
                >

                    ""
                </Show>
            </table>
        </div>
    }
}
