use crate::common::UserStatus;
use crate::i18n::*;
use crate::providers::online_users::{OnlineUsersSignal, OnlineUsersState};
use crate::{
    components::molecules::challenge_row::ChallengeRow,
    providers::{challenges::ChallengeStateSignal, AuthContext},
    responses::ChallengeResponse,
};
use leptos::prelude::*;

fn challenge_order(
    a: &ChallengeResponse,
    b: &ChallengeResponse,
    online_users: &OnlineUsersState,
) -> std::cmp::Ordering {
    let online_a = online_users.username_status.get(&a.challenger.username);
    let online_a = matches!(online_a, Some(UserStatus::Online));
    let online_b = online_users.username_status.get(&b.challenger.username);
    let online_b = matches!(online_b, Some(UserStatus::Online));
    if a.challenge_id.0.cmp(&b.challenge_id.0) == std::cmp::Ordering::Equal {
        std::cmp::Ordering::Equal
    } else if online_a && !online_b {
        std::cmp::Ordering::Less
    } else if !online_a && online_b {
        std::cmp::Ordering::Greater
    } else if a.time_base == b.time_base {
        let a_incr = a.time_increment.unwrap_or(i32::MAX);
        let b_incr = b.time_increment.unwrap_or(i32::MAX);
        a_incr.cmp(&b_incr)
    } else {
        let a_base = a.time_base.unwrap_or(i32::MAX);
        let b_base = b.time_base.unwrap_or(i32::MAX);
        a_base.cmp(&b_base)
    }
}

#[component]
pub fn Challenges() -> impl IntoView {
    let i18n = use_i18n();
    let th_class =
        "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh] max-w-screen";
    let challenges = expect_context::<ChallengeStateSignal>();
    let online_users = expect_context::<OnlineUsersSignal>().signal;
    let auth_context = expect_context::<AuthContext>();
    let user = move || {
        if let Some(Ok(Some(user))) = auth_context.user.get() {
            Some(user)
        } else {
            None
        }
    };
    let direct = Signal::derive(move || {
        let mut ret = if let Some(user) = user() {
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
        };
        ret.sort_by(|a, b| challenge_order(a, b, &online_users.get()));
        ret
    });

    let own = Signal::derive(move || {
        let mut ret = if let Some(user) = user() {
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
        };
        ret.sort_by(|a, b| challenge_order(a, b, &online_users.get()));
        ret
    });

    let public = Signal::derive(move || {
        let mut ret = if let Some(user) = user() {
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
        };
        ret.sort_by(|a, b| challenge_order(a, b, &online_users.get()));
        ret
    });
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
                <For each=direct key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=StoredValue::new(challenge.to_owned()) single=false />
                </For>
                <tr class="h-2"></tr>
                <For each=own key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=StoredValue::new(challenge.to_owned()) single=false />
                </For>
                <tr class="h-2"></tr>
                <For each=public key=|c| c.challenge_id.to_owned() let:challenge>
                    <ChallengeRow challenge=StoredValue::new(challenge.to_owned()) single=false />
                </For>
            </tbody>
        </table>
    }
}
