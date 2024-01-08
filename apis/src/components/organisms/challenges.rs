use crate::{
    components::molecules::challenge_row::ChallengeRow, responses::challenge::ChallengeResponse,
};
use leptos::*;
use std::collections::HashMap;

#[component]
pub fn Challenges(challenges: HashMap<String, ChallengeResponse>) -> impl IntoView {
    let th_class = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh]";
    let challenges = store_value(challenges);
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
                                    key=|(key, _)| key.to_owned()
                                    let:one_challenge
                                >
                                    <ChallengeRow
                                        challenge=store_value(one_challenge.1)
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
