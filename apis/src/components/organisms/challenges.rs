use crate::i18n::*;
use crate::{
    components::molecules::challenge_row::ChallengeRow,
    providers::{challenges::ChallengeStateSignal, AuthContext},
    responses::ChallengeResponse,
};
use leptos::*;

#[component]
pub fn Challenges() -> impl IntoView {
    let i18n = use_i18n();
    let th_class =
        "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh] max-w-screen";
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
    let has_games = move |list: Vec<ChallengeResponse>| !list.is_empty();
    let not_hidden =
        Memo::new(move |_| has_games(direct()) || has_games(own()) || has_games(public()));
    view! {
        <table class=move || {
            format!("table-fixed max-w-fit m-2 {}", if not_hidden() { "" } else { "hidden" })
        }>
            <thead>
                <tr>
                    <th class=th_class></th>
                    <th class=th_class>{t!(i18n, home.challenge_details.player)}</th>
                    <th class=th_class>Elo</th>
                    <th class=th_class>Plm</th>
                    <th class=th_class>{t!(i18n, home.challenge_details.time)}</th>
                    <th class=th_class>{t!(i18n, home.challenge_details.rated.title)}</th>
                    <th class=th_class></th>
                </tr>
            </thead>
            <tbody>
                <For each=move || { direct() } key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                </For>
                <tr class="h-2"></tr>
                <For each=move || { own() } key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                </For>
                <tr class="h-2"></tr>
                <For each=move || { public() } key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=store_value(challenge.to_owned()) single=false/>
                </For>
            </tbody>
        </table>
    }
}
