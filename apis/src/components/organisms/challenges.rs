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
    let challenges = expect_context::<ChallengeStateSignal>().signal;
    let online_users = expect_context::<OnlineUsersSignal>().signal;
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let uid = move || auth_context.user.with(|a| a.as_ref().map(|user| user.id));
    let direct = Signal::derive(move || {
        let mut ret = if user.with(|u| u.is_some()) {
            // Get the challenges direct at the current user
            challenges.with(|c| {
                c.challenges
                    .values()
                    .filter(|&challenge| {
                        challenge
                            .opponent
                            .as_ref()
                            .is_some_and(|o| o.uid == uid().unwrap())
                    })
                    .cloned()
                    .collect::<Vec<ChallengeResponse>>()
            })
        } else {
            challenges.with(|c| {
                c.challenges
                    .values()
                    .cloned()
                    .collect::<Vec<ChallengeResponse>>()
            })
        };
        online_users.with(|ou| ret.sort_by(|a, b| challenge_order(a, b, ou)));
        ret
    });

    let own = Signal::derive(move || {
        let mut ret = if user.with(|u| u.is_some()) {
            challenges.with(|c| {
                c.challenges
                    .values()
                    .filter(|&challenge| challenge.challenger.uid == uid().unwrap())
                    .cloned()
                    .collect::<Vec<ChallengeResponse>>()
            })
        } else {
            Vec::new()
        };
        online_users.with(|ou| ret.sort_by(|a, b| challenge_order(a, b, ou)));
        ret
    });

    let public = Signal::derive(move || {
        let mut ret = if user.with(|u| u.is_some()) {
            challenges.with(|c| {
                c.challenges
                    .values()
                    .filter(|&challenge| {
                        challenge.opponent.is_none() && challenge.challenger.uid != uid().unwrap()
                    })
                    .cloned()
                    .collect::<Vec<ChallengeResponse>>()
            })
        } else {
            Vec::new()
        };
        online_users.with(|ou| ret.sort_by(|a, b| challenge_order(a, b, ou)));
        ret
    });
    let has_games = |list: &Vec<ChallengeResponse>| !list.is_empty();
    let not_hidden =
        Memo::new(move |_| has_games(&direct()) || has_games(&own()) || has_games(&public()));
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
                <For each=direct key=|c| c.challenge_id.clone() let(challenge)>
                    <ChallengeRow challenge=challenge single=false uid=uid() />
                </For>
                <tr class="h-2"></tr>
                <For each=own key=|c| c.challenge_id.clone() let(challenge)>
                    <ChallengeRow challenge=challenge single=false uid=uid() />
                </For>
                <tr class="h-2"></tr>
                <For each=public key=|c| c.challenge_id.clone() let(challenge)>
                    <ChallengeRow challenge=challenge single=false uid=uid() />
                </For>
            </tbody>
        </table>
    }
}
