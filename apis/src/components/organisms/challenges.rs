use crate::{
    components::molecules::challenge_row::ChallengeRow,
    providers::{auth_context::AuthContext, challenges::ChallengeStateSignal},
    responses::challenge::ChallengeResponse,
};
use leptos::*;

#[component]
pub fn Challenges() -> impl IntoView {
    let th_class = "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh]";
    let challenges = expect_context::<ChallengeStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user = move || {
        if let Some(Ok(Some(user))) = (auth_context.user)() {
            Some(user)
        } else {
            None
        }
    };
    let direct = move || {
        if let Some(user) = user() {
            // Get the challenges direct at the current user
            challenges
                .signal
                .get()
                .challenges
                .values()
                .filter(|&challenge| challenge.clone().opponent.is_some_and(|o| o.uid == user.id))
                .cloned()
                .collect::<Vec<ChallengeResponse>>()
        } else {
            challenges
                .signal
                .get()
                .challenges
                .values()
                .cloned()
                .collect::<Vec<ChallengeResponse>>()
        }
    };

    let own = move || {
        if let Some(user) = user() {
            challenges
                .signal
                .get()
                .challenges
                .values()
                .filter(|&challenge| challenge.challenger.uid == user.id)
                .cloned()
                .collect::<Vec<ChallengeResponse>>()
        } else {
            Vec::new()
        }
    };

    let public = move || {
        if let Some(user) = user() {
            challenges
                .signal
                .get()
                .challenges
                .values()
                .filter(|&challenge| {
                    challenge.clone().opponent.is_none() && challenge.challenger.uid != user.id
                })
                .cloned()
                .collect::<Vec<ChallengeResponse>>()
        } else {
            Vec::new()
        }
    };
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
                <tbody>
                    <For each=move || { direct() } key=|c| c.nanoid.to_owned() let:challenge>
                        <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                    </For>
                    <tr class="h-2"></tr>
                    <For each=move || { own() } key=|c| c.nanoid.to_owned() let:challenge>
                        <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                    </For>
                    <tr class="h-2"></tr>
                    <For each=move || { public() } key=|c| c.nanoid.to_owned() let:challenge>
                        <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                    </For>
                </tbody>
            </table>
        </div>
    }
}
