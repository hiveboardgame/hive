use crate::common::UserStatus;
use crate::i18n::*;
use crate::providers::online_users::{OnlineUsersSignal, OnlineUsersState};
use crate::{
    components::molecules::challenge_row::ChallengeRow,
    providers::{challenges::ChallengeStateSignal, AuthContext},
    responses::ChallengeResponse,
};
use leptos::prelude::*;
use shared_types::ChallengeId;
use std::collections::HashMap;

/// Represents a group of challenges with identical parameters from the same user
#[derive(Clone, Debug)]
pub struct GroupedChallenge {
    /// The representative challenge to display
    pub challenge: ChallengeResponse,
    /// All challenge IDs in this group (for bulk actions like cancel)
    pub challenge_ids: Vec<ChallengeId>,
    /// Number of challenges in this group
    pub count: usize,
}

impl GroupedChallenge {
    /// Creates a unique key for grouping challenges based on their attributes.
    /// Includes opponent uid so direct challenges to different players are not grouped.
    pub fn group_key(challenge: &ChallengeResponse) -> String {
        let opponent_uid = challenge
            .opponent
            .as_ref()
            .map(|o| o.uid.to_string())
            .unwrap_or_default();
        format!(
            "{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}",
            challenge.challenger.uid,
            challenge.game_type,
            challenge.rated,
            challenge.time_mode,
            challenge.time_base,
            challenge.time_increment,
            challenge.color_choice,
            challenge.visibility,
            challenge.band_lower,
            challenge.band_upper,
            opponent_uid,
        )
    }

    /// Returns a unique key for this group that includes the count for reactivity
    pub fn reactive_key(&self) -> String {
        format!("{}|{}", Self::group_key(&self.challenge), self.count)
    }

    /// Groups a list of challenges by their common attributes
    pub fn group_challenges(challenges: Vec<ChallengeResponse>) -> Vec<GroupedChallenge> {
        let mut groups: HashMap<String, GroupedChallenge> = HashMap::new();

        for challenge in challenges {
            let key = Self::group_key(&challenge);

            groups
                .entry(key)
                .and_modify(|group| {
                    group.challenge_ids.push(challenge.challenge_id.clone());
                    group.count += 1;
                })
                .or_insert_with(|| GroupedChallenge {
                    challenge_ids: vec![challenge.challenge_id.clone()],
                    challenge: challenge,
                    count: 1,
                });
        }

        groups.into_values().collect()
    }
}

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

fn grouped_challenge_order(
    a: &GroupedChallenge,
    b: &GroupedChallenge,
    online_users: &OnlineUsersState,
) -> std::cmp::Ordering {
    challenge_order(&a.challenge, &b.challenge, online_users)
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
        let challenges_list = if user.with(|u| u.is_some()) {
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
        let mut grouped = GroupedChallenge::group_challenges(challenges_list);
        online_users.with(|ou| grouped.sort_by(|a, b| grouped_challenge_order(a, b, ou)));
        grouped
    });

    let own = Signal::derive(move || {
        let challenges_list = if user.with(|u| u.is_some()) {
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
        let mut grouped = GroupedChallenge::group_challenges(challenges_list);
        online_users.with(|ou| grouped.sort_by(|a, b| grouped_challenge_order(a, b, ou)));
        grouped
    });

    let public = Signal::derive(move || {
        let challenges_list = if user.with(|u| u.is_some()) {
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
        let mut grouped = GroupedChallenge::group_challenges(challenges_list);
        online_users.with(|ou| grouped.sort_by(|a, b| grouped_challenge_order(a, b, ou)));
        grouped
    });
    let has_games = |list: &Vec<GroupedChallenge>| !list.is_empty();
    let not_hidden =
        Memo::new(move |_| has_games(&direct()) || has_games(&own()) || has_games(&public()));
    view! {
        <div class=move || {
            format!(
                "w-full m-2 overflow-hidden flex justify-center lg:justify-end 2xl:justify-center {}",
                if not_hidden() { "" } else { "hidden" },
            )
        }>
            <div class="overflow-y-auto w-full max-w-screen-md max-h-96 rounded-lg border border-gray-200 dark:border-gray-700">
                <table class="w-full min-w-0 table-fixed">
                    <thead class="sticky top-0 z-10 bg-white border-b border-gray-200 dark:bg-gray-800 dark:border-gray-700">
                        <tr>
                            <th class=format!("{} w-6 min-w-0", th_class)></th>
                            <th class=format!(
                                "{} w-16 xs:w-20 sm:w-24 md:w-32 lg:w-40 min-w-0 text-xs sm:text-sm",
                                th_class,
                            )>{t!(i18n, home.challenge_details.player)}</th>
                            <th class=format!(
                                "{} w-12 xs:w-14 sm:w-16 md:w-16 lg:w-20 min-w-0 text-xs sm:text-sm",
                                th_class,
                            )>Elo</th>
                            <th class=format!(
                                "{} w-8 xs:w-10 sm:w-12 md:w-14 lg:w-16 min-w-0 text-xs sm:text-sm",
                                th_class,
                            )>Plm</th>
                            <th class=format!(
                                "{} w-12 xs:w-14 sm:w-16 md:w-20 lg:w-24 min-w-0 text-xs sm:text-sm",
                                th_class,
                            )>{t!(i18n, home.challenge_details.time)}</th>
                            <th class=format!(
                                "{} w-8 xs:w-10 sm:w-12 md:w-14 lg:w-16 min-w-0 text-xs sm:text-sm",
                                th_class,
                            )>{t!(i18n, home.challenge_details.rated.title)}</th>
                            <th class=format!(
                                "{} w-12 xs:w-14 sm:w-16 md:w-18 lg:w-20 min-w-0",
                                th_class,
                            )></th>
                        </tr>
                    </thead>
                    <tbody>
                        <For each=direct key=|g| g.reactive_key() let(grouped)>
                            <ChallengeRow
                                challenge=grouped.challenge
                                single=false
                                uid=uid()
                                count=grouped.count
                                challenge_ids=grouped.challenge_ids
                            />
                        </For>
                        <For each=own key=|g| g.reactive_key() let(grouped)>
                            <ChallengeRow
                                challenge=grouped.challenge
                                single=false
                                uid=uid()
                                count=grouped.count
                                challenge_ids=grouped.challenge_ids
                            />
                        </For>
                        <For each=public key=|g| g.reactive_key() let(grouped)>
                            <ChallengeRow
                                challenge=grouped.challenge
                                single=false
                                uid=uid()
                                count=grouped.count
                                challenge_ids=grouped.challenge_ids
                            />
                        </For>
                    </tbody>
                </table>
            </div>
        </div>
    }
}
