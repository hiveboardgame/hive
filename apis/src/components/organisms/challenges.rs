use crate::{
    common::{challenge_displayed_player, challenge_viewer_role, with_class, UserStatus},
    components::molecules::challenge_row::ChallengeRow,
    hooks::tap_feedback::use_tap_feedback,
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

const CHALLENGE_SEGMENTED_CLASS: &str =
    "flex overflow-hidden rounded-none border-b border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-muted";
const CHALLENGE_TAB_CLASS: &str =
    "flex grow items-center justify-center gap-2 px-4 py-2 font-bold transition-colors duration-200";
const CHALLENGE_TAB_ACTIVE_CLASS: &str = "bg-button-dawn text-white hover:bg-button-dawn data-[ui-pressed=true]:bg-button-dawn dark:bg-button-twilight dark:hover:bg-button-twilight dark:data-[ui-pressed=true]:bg-button-twilight";
const CHALLENGE_TAB_INACTIVE_CLASS: &str = "text-gray-700 hover:bg-pillbug-teal/10 data-[ui-pressed=true]:bg-pillbug-teal/10 dark:text-gray-200 dark:hover:bg-pillbug-teal/10 dark:data-[ui-pressed=true]:bg-pillbug-teal/10";
const CHALLENGE_TAB_BADGE_CLASS: &str = "rounded-full px-2 py-0.5 text-xs";
const CHALLENGE_TAB_BADGE_ACTIVE_CLASS: &str = "bg-white/20 text-white";
const CHALLENGE_TAB_BADGE_INACTIVE_CLASS: &str =
    "bg-pillbug-teal/10 text-pillbug-teal dark:bg-pillbug-teal/15";
const CHALLENGE_TABLE_SCROLL_CLASS: &str = "max-h-96 overflow-x-auto overflow-y-auto";
const CHALLENGE_TABLE_CLASS: &str =
    "w-full min-w-[21.5rem] table-auto sm:min-w-full sm:table-fixed";

struct ChallengeTabState {
    rows: Vec<GroupedChallenge>,
    signature: Vec<Vec<ChallengeId>>,
}

struct ChallengeTabsState {
    humans: ChallengeTabState,
    bots: ChallengeTabState,
}

fn challenge_tabs_state_changed(
    prev: Option<&ChallengeTabsState>,
    next: Option<&ChallengeTabsState>,
) -> bool {
    match (prev, next) {
        (Some(prev), Some(next)) => {
            prev.humans.signature != next.humans.signature
                || prev.bots.signature != next.bots.signature
        }
        (None, None) => false,
        _ => true,
    }
}

fn challenge_time_sort_key(challenge: &ChallengeResponse) -> (i32, i32) {
    (
        challenge.time_base.unwrap_or(i32::MAX),
        challenge.time_increment.unwrap_or(i32::MAX),
    )
}

fn displayed_player_rating(challenge: &ChallengeResponse, viewer_id: Option<Uuid>) -> u64 {
    let role = challenge_viewer_role(challenge, viewer_id);
    let (_, rating) = challenge_displayed_player(challenge, role);
    rating
}

fn displayed_player_is_online(
    challenge: &ChallengeResponse,
    online_users: &OnlineUsersState,
    viewer_id: Option<Uuid>,
) -> bool {
    let role = challenge_viewer_role(challenge, viewer_id);
    let (displayed_player, _) = challenge_displayed_player(challenge, role);
    matches!(
        online_users.username_status.get(&displayed_player.username),
        Some(UserStatus::Online)
    )
}

fn append_grouped_challenge_rows(
    tabs: &mut ChallengeTabsState,
    challenges_list: Vec<ChallengeResponse>,
    online_users: &OnlineUsersState,
    viewer_id: Option<Uuid>,
) {
    if challenges_list.is_empty() {
        return;
    }

    let mut grouped = GroupedChallenge::group_challenges(challenges_list);
    grouped.sort_by(|a, b| {
        b.challenge
            .rated
            .cmp(&a.challenge.rated)
            .then_with(|| {
                challenge_time_sort_key(&a.challenge).cmp(&challenge_time_sort_key(&b.challenge))
            })
            .then_with(|| {
                displayed_player_is_online(&b.challenge, online_users, viewer_id).cmp(
                    &displayed_player_is_online(&a.challenge, online_users, viewer_id),
                )
            })
            .then_with(|| {
                displayed_player_rating(&b.challenge, viewer_id)
                    .cmp(&displayed_player_rating(&a.challenge, viewer_id))
            })
            .then_with(|| a.challenge.challenge_id.0.cmp(&b.challenge.challenge_id.0))
    });

    for group in grouped {
        let role = challenge_viewer_role(&group.challenge, viewer_id);
        let (displayed_player, _) = challenge_displayed_player(&group.challenge, role);
        let tab = if displayed_player.bot {
            &mut tabs.bots
        } else {
            &mut tabs.humans
        };

        tab.signature.push(group.challenge_ids.clone());
        tab.rows.push(group);
    }
}

fn build_tabs_state(
    challenges: &HashMap<ChallengeId, ChallengeResponse>,
    online_users: &OnlineUsersState,
    viewer_id: Option<Uuid>,
) -> ChallengeTabsState {
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

    let mut tabs = ChallengeTabsState {
        humans: ChallengeTabState {
            rows: Vec::new(),
            signature: Vec::new(),
        },
        bots: ChallengeTabState {
            rows: Vec::new(),
            signature: Vec::new(),
        },
    };

    append_grouped_challenge_rows(&mut tabs, direct, online_users, viewer_id);
    append_grouped_challenge_rows(&mut tabs, own, online_users, viewer_id);
    append_grouped_challenge_rows(&mut tabs, public, online_users, viewer_id);

    tabs
}

#[component]
pub fn Challenges() -> impl IntoView {
    let i18n = use_i18n();
    let th_class =
        "py-1 px-1 md:py-2 md:px-2 lg:px-3 font-bold uppercase max-h-[80vh] max-w-screen";
    let challenges = expect_context::<ChallengeStateSignal>().signal;
    let online_users = expect_context::<OnlineUsersSignal>().signal;
    let auth_context = expect_context::<AuthContext>();
    let active_tab = RwSignal::new(ChallengeTab::Humans);
    let mark_tab_press = use_tap_feedback("[data-challenge-tab]");
    let user = auth_context.user;
    let tabs = Memo::new_with_compare(
        move |_| {
            let viewer_id = user.with(|account| account.as_ref().map(|account| account.id));
            challenges.with(|state| {
                online_users.with(|users| build_tabs_state(&state.challenges, users, viewer_id))
            })
        },
        challenge_tabs_state_changed,
    );
    let empty_state_message = move || match active_tab() {
        ChallengeTab::Humans => t_string!(i18n, home.challenge_tabs.empty_state.humans),
        ChallengeTab::Bots => t_string!(i18n, home.challenge_tabs.empty_state.bots),
    };

    view! {
        <div class="overflow-hidden mx-auto w-full max-w-screen-md ui-panel">
            <div
                class=CHALLENGE_SEGMENTED_CLASS
                on:pointerdown=move |event| mark_tab_press.run(event)
            >
                <button
                    type="button"
                    data-challenge-tab="true"
                    class=move || {
                        if active_tab() == ChallengeTab::Humans {
                            with_class(CHALLENGE_TAB_CLASS, CHALLENGE_TAB_ACTIVE_CLASS)
                        } else {
                            with_class(CHALLENGE_TAB_CLASS, CHALLENGE_TAB_INACTIVE_CLASS)
                        }
                    }
                    on:click=move |_| active_tab.set(ChallengeTab::Humans)
                >
                    <span>{move || t_string!(i18n, home.challenge_tabs.humans)}</span>
                    <span class=move || {
                        if active_tab() == ChallengeTab::Humans {
                            with_class(CHALLENGE_TAB_BADGE_CLASS, CHALLENGE_TAB_BADGE_ACTIVE_CLASS)
                        } else {
                            with_class(
                                CHALLENGE_TAB_BADGE_CLASS,
                                CHALLENGE_TAB_BADGE_INACTIVE_CLASS,
                            )
                        }
                    }>{move || tabs.with(|tabs| tabs.humans.rows.len().to_string())}</span>
                </button>
                <button
                    type="button"
                    data-challenge-tab="true"
                    class=move || {
                        if active_tab() == ChallengeTab::Bots {
                            with_class(CHALLENGE_TAB_CLASS, CHALLENGE_TAB_ACTIVE_CLASS)
                        } else {
                            with_class(CHALLENGE_TAB_CLASS, CHALLENGE_TAB_INACTIVE_CLASS)
                        }
                    }
                    on:click=move |_| active_tab.set(ChallengeTab::Bots)
                >
                    <span>{move || t_string!(i18n, home.challenge_tabs.bots)}</span>
                    <span class=move || {
                        if active_tab() == ChallengeTab::Bots {
                            with_class(CHALLENGE_TAB_BADGE_CLASS, CHALLENGE_TAB_BADGE_ACTIVE_CLASS)
                        } else {
                            with_class(
                                CHALLENGE_TAB_BADGE_CLASS,
                                CHALLENGE_TAB_BADGE_INACTIVE_CLASS,
                            )
                        }
                    }>{move || tabs.with(|tabs| tabs.bots.rows.len().to_string())}</span>
                </button>
            </div>
            <div class=CHALLENGE_TABLE_SCROLL_CLASS>
                <table class=CHALLENGE_TABLE_CLASS>
                    <thead class="sticky top-0 z-10 border-b border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel">
                        <tr>
                            <th class=format!("{} w-24 sm:w-16 min-w-0", th_class)></th>
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
                                "{} hidden sm:table-cell w-20 md:w-24 min-w-0",
                                th_class,
                            )></th>
                        </tr>
                    </thead>
                    <tbody>
                        <Show when=move || {
                            match active_tab() {
                                ChallengeTab::Humans => {
                                    tabs.with(|tabs| tabs.humans.rows.is_empty())
                                }
                                ChallengeTab::Bots => tabs.with(|tabs| tabs.bots.rows.is_empty()),
                            }
                        }>
                            <tr>
                                <td colspan="7" class="p-3">
                                    <div class=with_class(
                                        "flex flex-col items-center justify-center rounded-lg border border-dashed border-black/15 bg-odd-light/80 px-4 py-8 text-center text-gray-600 dark:border-white/15 dark:bg-surface-field dark:text-gray-300",
                                        "border-0 py-6",
                                    )>{empty_state_message}</div>
                                </td>
                            </tr>
                        </Show>
                        <For
                            each=move || {
                                match active_tab() {
                                    ChallengeTab::Humans => {
                                        tabs.with(|tabs| tabs.humans.rows.clone())
                                    }
                                    ChallengeTab::Bots => tabs.with(|tabs| tabs.bots.rows.clone()),
                                }
                            }
                            key=|g| g.reactive_key()
                            let(grouped)
                        >
                            <ChallengeRow
                                challenge=grouped.challenge
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
