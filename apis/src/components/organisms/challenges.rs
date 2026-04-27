use crate::{
    common::UserStatus,
    components::molecules::challenge_row::ChallengeRow,
    i18n::*,
    providers::{
        challenges::ChallengeStateSignal,
        online_users::{OnlineUsersSignal, OnlineUsersState},
        AuthContext,
    },
    responses::ChallengeResponse,
};
use leptos::prelude::*;
use shared_types::ChallengeId;
use std::collections::HashMap;
use uuid::Uuid;

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

    /// Returns a unique key for this group based on the exact grouped membership.
    pub fn reactive_key(&self) -> String {
        self.challenge_ids
            .iter()
            .map(|id| id.0.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Groups a list of challenges by their common attributes
    pub fn group_challenges(challenges: Vec<ChallengeResponse>) -> Vec<GroupedChallenge> {
        let mut groups: HashMap<String, Vec<ChallengeResponse>> = HashMap::new();

        for challenge in challenges {
            groups
                .entry(Self::group_key(&challenge))
                .or_default()
                .push(challenge);
        }

        groups
            .into_values()
            .map(|mut group| {
                group.sort_by(|a, b| a.challenge_id.0.cmp(&b.challenge_id.0));
                let challenge_ids = group
                    .iter()
                    .map(|challenge| challenge.challenge_id.clone())
                    .collect::<Vec<_>>();
                let count = challenge_ids.len();
                let challenge = group
                    .into_iter()
                    .next()
                    .expect("challenge group to contain at least one challenge");

                GroupedChallenge {
                    challenge,
                    challenge_ids,
                    count,
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChallengeTab {
    Humans,
    Bots,
}

fn tab_button_state_classes(tab: ChallengeTab, active_tab: ChallengeTab) -> &'static str {
    match (tab, active_tab) {
        (ChallengeTab::Humans, ChallengeTab::Humans) | (ChallengeTab::Bots, ChallengeTab::Bots) => {
            "text-white bg-button-dawn dark:bg-button-twilight"
        }
        (ChallengeTab::Humans, ChallengeTab::Bots) | (ChallengeTab::Bots, ChallengeTab::Humans) => {
            "hover:bg-pillbug-teal/10 dark:hover:bg-pillbug-teal/10"
        }
    }
}

fn tab_badge_state_classes(tab: ChallengeTab, active_tab: ChallengeTab) -> &'static str {
    match (tab, active_tab) {
        (ChallengeTab::Humans, ChallengeTab::Humans) | (ChallengeTab::Bots, ChallengeTab::Bots) => {
            "bg-white/20 text-white"
        }
        (ChallengeTab::Humans, ChallengeTab::Bots) | (ChallengeTab::Bots, ChallengeTab::Humans) => {
            "bg-pillbug-teal/10 text-pillbug-teal"
        }
    }
}

struct ChallengeTabState {
    rows: Vec<GroupedChallenge>,
    signature: Vec<Vec<ChallengeId>>,
}

fn challenge_tab_state_changed(
    prev: Option<&ChallengeTabState>,
    next: Option<&ChallengeTabState>,
) -> bool {
    prev.map(|state| &state.signature) != next.map(|state| &state.signature)
}

fn build_tab_state(
    tab: ChallengeTab,
    challenges: &HashMap<ChallengeId, ChallengeResponse>,
    online_users: &OnlineUsersState,
    viewer_id: Option<Uuid>,
) -> ChallengeTabState {
    let mut direct = Vec::new();
    let mut own = Vec::new();
    let mut public = Vec::new();

    for challenge in challenges.values() {
        if let Some(viewer_id) = viewer_id {
            if challenge
                .opponent
                .as_ref()
                .is_some_and(|opponent| opponent.uid == viewer_id)
            {
                direct.push(challenge.clone());
            } else if challenge.challenger.uid == viewer_id {
                own.push(challenge.clone());
            } else if challenge.opponent.is_none() {
                public.push(challenge.clone());
            }
        } else {
            direct.push(challenge.clone());
        }
    }

    let mut rows = Vec::new();
    let mut signature = Vec::new();
    let mut append_matching_rows = |challenges_list: Vec<ChallengeResponse>| {
        if challenges_list.is_empty() {
            return;
        }

        let mut grouped = GroupedChallenge::group_challenges(challenges_list);
        grouped.sort_by(|a, b| {
            let online_a = matches!(
                online_users
                    .username_status
                    .get(&a.challenge.challenger.username),
                Some(UserStatus::Online)
            );
            let online_b = matches!(
                online_users
                    .username_status
                    .get(&b.challenge.challenger.username),
                Some(UserStatus::Online)
            );

            if a.challenge.challenge_id.0 == b.challenge.challenge_id.0 {
                std::cmp::Ordering::Equal
            } else if online_a && !online_b {
                std::cmp::Ordering::Less
            } else if !online_a && online_b {
                std::cmp::Ordering::Greater
            } else if a.challenge.time_base == b.challenge.time_base {
                a.challenge
                    .time_increment
                    .unwrap_or(i32::MAX)
                    .cmp(&b.challenge.time_increment.unwrap_or(i32::MAX))
            } else {
                a.challenge
                    .time_base
                    .unwrap_or(i32::MAX)
                    .cmp(&b.challenge.time_base.unwrap_or(i32::MAX))
            }
        });

        for group in grouped {
            let displayed_player_is_bot = match group.challenge.opponent.as_ref() {
                Some(opponent) if Some(group.challenge.challenger.uid) == viewer_id => opponent.bot,
                Some(_) | None => group.challenge.challenger.bot,
            };
            let group_matches_tab = displayed_player_is_bot
                == match tab {
                    ChallengeTab::Humans => false,
                    ChallengeTab::Bots => true,
                };

            if group_matches_tab {
                signature.push(group.challenge_ids.clone());
                rows.push(group);
            }
        }
    };

    append_matching_rows(direct);
    append_matching_rows(own);
    append_matching_rows(public);

    ChallengeTabState { rows, signature }
}

#[component]
pub fn Challenges(#[prop(optional)] realtime_disabled: Signal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    let th_class =
        "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh] max-w-screen";
    let challenges = expect_context::<ChallengeStateSignal>().signal;
    let online_users = expect_context::<OnlineUsersSignal>().signal;
    let auth_context = expect_context::<AuthContext>();
    let active_tab = RwSignal::new(ChallengeTab::Humans);
    let user = auth_context.user;
    let humans = Memo::new_with_compare(
        move |_| {
            let viewer_id = user.with(|account| account.as_ref().map(|account| account.id));
            challenges.with(|state| {
                online_users.with(|users| {
                    build_tab_state(ChallengeTab::Humans, &state.challenges, users, viewer_id)
                })
            })
        },
        challenge_tab_state_changed,
    );
    let bots = Memo::new_with_compare(
        move |_| {
            let viewer_id = user.with(|account| account.as_ref().map(|account| account.id));
            challenges.with(|state| {
                online_users.with(|users| {
                    build_tab_state(ChallengeTab::Bots, &state.challenges, users, viewer_id)
                })
            })
        },
        challenge_tab_state_changed,
    );
    let empty_state_message = move || match active_tab() {
        ChallengeTab::Humans => t_string!(i18n, home.challenge_tabs.empty_state.humans),
        ChallengeTab::Bots => t_string!(i18n, home.challenge_tabs.empty_state.bots),
    };

    view! {
        <div class="flex overflow-hidden justify-center m-2 w-full lg:justify-end 2xl:justify-center">
            <div class="overflow-hidden w-full max-w-screen-md rounded-lg border border-gray-200 dark:border-gray-700">
                <div class="flex border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700 bg-stone-100">
                    <button
                        class=move || {
                            format!(
                                "flex gap-2 items-center justify-center px-4 py-2 font-bold transition-colors duration-300 grow {}",
                                tab_button_state_classes(ChallengeTab::Humans, active_tab()),
                            )
                        }
                        on:click=move |_| active_tab.set(ChallengeTab::Humans)
                    >
                        <span>{move || t_string!(i18n, home.challenge_tabs.humans)}</span>
                        <span class=move || {
                            format!(
                                "py-0.5 px-2 text-xs rounded-full {}",
                                tab_badge_state_classes(ChallengeTab::Humans, active_tab()),
                            )
                        }>{move || humans.with(|tab| tab.rows.len().to_string())}</span>
                    </button>
                    <button
                        class=move || {
                            format!(
                                "flex gap-2 items-center justify-center px-4 py-2 font-bold transition-colors duration-300 grow {}",
                                tab_button_state_classes(ChallengeTab::Bots, active_tab()),
                            )
                        }
                        on:click=move |_| active_tab.set(ChallengeTab::Bots)
                    >
                        <span>{move || t_string!(i18n, home.challenge_tabs.bots)}</span>
                        <span class=move || {
                            format!(
                                "py-0.5 px-2 text-xs rounded-full {}",
                                tab_badge_state_classes(ChallengeTab::Bots, active_tab()),
                            )
                        }>{move || bots.with(|tab| tab.rows.len().to_string())}</span>
                    </button>
                </div>
                <div class="overflow-y-auto max-h-96">
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
                            <Show when=move || {
                                match active_tab() {
                                    ChallengeTab::Humans => humans.with(|tab| tab.rows.is_empty()),
                                    ChallengeTab::Bots => bots.with(|tab| tab.rows.is_empty()),
                                }
                            }>
                                <tr>
                                    <td
                                        colspan="7"
                                        class="py-6 px-4 text-sm text-center text-gray-500 dark:text-gray-400"
                                    >
                                        {empty_state_message}
                                    </td>
                                </tr>
                            </Show>
                            <For
                                each=move || {
                                    match active_tab() {
                                        ChallengeTab::Humans => humans.with(|tab| tab.rows.clone()),
                                        ChallengeTab::Bots => bots.with(|tab| tab.rows.clone()),
                                    }
                                }
                                key=|g| g.reactive_key()
                                let(grouped)
                            >
                                <ChallengeRow
                                    challenge=grouped.challenge
                                    single=false
                                    count=grouped.count
                                    challenge_ids=grouped.challenge_ids
                                    realtime_disabled
                                />
                            </For>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}
