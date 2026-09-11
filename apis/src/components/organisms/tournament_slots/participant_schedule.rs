use super::{
    fixed_slot_fields,
    offers_by_slot,
    schedule_is_available,
    slot_card_label,
    slot_ordering,
    SlotOffers,
};
use crate::{
    common::format_local_datetime as format_schedule_time,
    components::{
        atoms::message_button::MessageButton,
        molecules::user_identity::UserIdentity,
        organisms::my_schedules::ScheduleEditorDialog,
    },
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        RoundRobinStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatStore,
        TournamentScheduleState,
        TournamentState,
    },
    responses::{ScheduleResponse, UserResponse},
};
use chrono::{DateTime, Duration, Local, Utc};
use leptos::{html::Dialog, prelude::*};
use leptos_i18n::I18nContext;
use leptos_router::{
    components::{Outlet, A},
    hooks::{use_location, use_params_map},
};
use reactive_stores::ArcField;
use shared_types::{
    tournament::{SlotKey, SwissLeg},
    tournament_view::SlotResponse,
    TournamentId,
};
use std::{cmp::Ordering, collections::HashMap};
use uuid::Uuid;

#[cfg(feature = "hydrate")]
use crate::common::parse_schedule_slot_fragment;
#[cfg(feature = "hydrate")]
use leptos_router::hooks::use_url;
#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

#[cfg(feature = "hydrate")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotFragment {
    Focus(Uuid),
    OpenSchedule(Uuid),
}

#[cfg(feature = "hydrate")]
fn parse_slot_fragment(hash: &str) -> Option<SlotFragment> {
    let fragment = hash.strip_prefix('#').unwrap_or(hash);
    fragment
        .strip_prefix("tournament-slot-")
        .and_then(|slot_id| slot_id.parse().ok())
        .map(SlotFragment::Focus)
        .or_else(|| parse_schedule_slot_fragment(fragment).map(SlotFragment::OpenSchedule))
}

#[cfg(feature = "hydrate")]
fn reveal_slot_fragment() {
    use leptos::leptos_dom::helpers::{document, request_animation_frame};

    let url = use_url();
    Effect::new(move |_| {
        let hash = url.get().hash().to_string();
        let Some(fragment) = parse_slot_fragment(&hash) else {
            return;
        };
        let slot_id = match fragment {
            SlotFragment::Focus(slot_id) | SlotFragment::OpenSchedule(slot_id) => slot_id,
        };
        let id = format!("tournament-slot-{slot_id}");
        request_animation_frame(move || {
            request_animation_frame(move || {
                let Some(element) = document().get_element_by_id(&id) else {
                    return;
                };
                let mut ancestor = Some(element.clone());
                while let Some(current) = ancestor {
                    if current.tag_name() == "DETAILS" {
                        let _ = current.set_attribute("open", "");
                    }
                    ancestor = current.parent_element();
                }
                element.scroll_into_view_with_bool(false);
                if let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() {
                    let _ = element.focus();
                }
            });
        });
    });
}

#[cfg(not(feature = "hydrate"))]
fn reveal_slot_fragment() {}

#[cfg(feature = "hydrate")]
fn requested_schedule_fragment() -> Signal<Option<Uuid>> {
    let url = use_url();
    Signal::derive(move || match parse_slot_fragment(url.get().hash()) {
        Some(SlotFragment::OpenSchedule(slot_id)) => Some(slot_id),
        Some(SlotFragment::Focus(_)) | None => None,
    })
}

#[cfg(not(feature = "hydrate"))]
fn requested_schedule_fragment() -> Signal<Option<Uuid>> {
    Signal::derive(|| None)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ScheduleRoute {
    Index,
    Opponent(String),
    Invalid,
}

fn schedule_route(pathname: &str, tournament_id: &TournamentId) -> Option<ScheduleRoute> {
    let root = format!("/tournament/{}/schedule", tournament_id.0);
    let suffix = pathname.strip_prefix(&root)?;
    if suffix.is_empty() || suffix == "/" {
        return Some(ScheduleRoute::Index);
    }
    let Some(username) = suffix.strip_prefix('/') else {
        return Some(ScheduleRoute::Invalid);
    };
    let username = username.trim_end_matches('/');
    Some(if !username.is_empty() && !username.contains('/') {
        ScheduleRoute::Opponent(username.to_string())
    } else {
        ScheduleRoute::Invalid
    })
}

fn current_schedule_route(context: &ScheduleContext) -> Option<ScheduleRoute> {
    let pathname = context.pathname.try_get()?;
    let tournament_id = context.tournament_id.try_get_value()?;
    let route = schedule_route(&pathname, &tournament_id)?;
    context.route_active.try_get()?.then_some(route)
}

fn short_game_label(tournament: TournamentState, slot: &SlotResponse) -> String {
    let number = match slot.key {
        SlotKey::RoundRobin { .. } => match tournament.format {
            TournamentFormatStore::RoundRobin(format) => format
                .rounds()
                .get()
                .iter()
                .find(|round| {
                    round
                        .slots
                        .iter()
                        .any(|round_slot| round_slot.slot_id == slot.id)
                })
                .map_or(1, |round| round.pass_index + 1),
            _ => 1,
        },
        SlotKey::Swiss { slot: native_slot } => match native_slot.leg {
            SwissLeg::Single | SwissLeg::First => 1,
            SwissLeg::Second => 2,
        },
        SlotKey::Elimination {
            slot: native_slot, ..
        } => native_slot.value() as usize + 1,
    };
    // TODO: i18n once copy is approved.
    format!("Game {number}")
}

// Variant order is display priority.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ScheduleState {
    Starting,
    Late,
    ReplyNeeded,
    Overdue,
    Unscheduled,
    ProposalSent,
    Scheduled,
}

fn scheduled_timing_state(scheduled_at: DateTime<Utc>, now: DateTime<Utc>) -> ScheduleState {
    if now >= scheduled_at + Duration::minutes(30) {
        ScheduleState::Late
    } else if now >= scheduled_at - Duration::minutes(15) {
        ScheduleState::Starting
    } else {
        ScheduleState::Scheduled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScheduleSummary {
    state: ScheduleState,
    relevant_at: Option<DateTime<Utc>>,
    label: String,
    detail: Option<String>,
    predecessor_game_href: Option<String>,
}

#[derive(Clone, PartialEq)]
struct OpponentGroup {
    user: UserResponse,
    slot_ids: Vec<Uuid>,
    summary: Option<ScheduleSummary>,
}

#[derive(Clone)]
struct OpponentSlots {
    user: UserResponse,
    slot_ids: Vec<Uuid>,
    slots: Vec<ArcField<SlotResponse>>,
}

impl PartialEq for OpponentSlots {
    fn eq(&self, other: &Self) -> bool {
        self.user == other.user && self.slot_ids == other.slot_ids
    }
}

#[derive(Clone, Copy)]
struct ScheduleContext {
    tournament_id: StoredValue<TournamentId>,
    tournament: TournamentState,
    slots: Signal<Vec<ArcField<SlotResponse>>>,
    slot_offers: Memo<HashMap<Uuid, SlotOffers>>,
    user_id: Signal<Option<Uuid>>,
    now: Signal<DateTime<Utc>>,
    opponents: Memo<Vec<OpponentGroup>>,
    selected_slot: RwSignal<Option<Uuid>>,
    dialog_el: NodeRef<Dialog>,
    schedule_root: StoredValue<String>,
    pathname: Memo<String>,
    route_active: Signal<bool>,
}

fn editor_button_label(
    pending: Option<&ScheduleResponse>,
    user_id: Uuid,
    scheduled: bool,
    now: DateTime<Utc>,
) -> &'static str {
    // TODO: i18n once copy is approved.
    if pending.is_some_and(|offer| !offer.has_future_candidate(now)) {
        "Suggest new times"
    } else if pending.is_some_and(|offer| offer.opponent_id == user_id) {
        "Reply"
    } else if pending.is_some() {
        "View proposal"
    } else if scheduled {
        "Reschedule"
    } else {
        "Propose times"
    }
}

fn pending_summary(
    i18n: I18nContext<Locale, I18nKeys>,
    offer: &ScheduleResponse,
    user_id: Uuid,
    scheduled_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> ScheduleSummary {
    let incoming = offer.opponent_id == user_id;
    let detail = scheduled_at.map(|scheduled_at| {
        // TODO: i18n once copy is approved.
        format!(
            "Currently scheduled · {}",
            format_schedule_time(i18n.get_locale(), scheduled_at),
        )
    });
    let expired = !offer.has_future_candidate(now);
    ScheduleSummary {
        state: if expired {
            ScheduleState::Unscheduled
        } else if incoming {
            ScheduleState::ReplyNeeded
        } else {
            ScheduleState::ProposalSent
        },
        relevant_at: None,
        label: if expired {
            // TODO: i18n once copy is approved.
            String::from("Proposed times expired")
        } else if incoming {
            // TODO: i18n once copy is approved.
            String::from(if scheduled_at.is_some() {
                "Reschedule request"
            } else {
                "Reply needed"
            })
        } else {
            // TODO: i18n once copy is approved.
            String::from("Proposed")
        },
        detail: detail.or_else(|| {
            incoming.then(|| {
                // TODO: i18n once copy is approved.
                format!(
                    "{} proposed time{}",
                    offer.candidate_times.len(),
                    if offer.candidate_times.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                )
            })
        }),
        predecessor_game_href: None,
    }
}

fn predecessor_link(tournament: TournamentState, slot: &SlotResponse) -> Option<(String, String)> {
    let predecessor_id = slot.waits_for?;
    let predecessor = fixed_slot_fields(tournament)
        .into_iter()
        .find(|candidate| candidate.with_untracked(|slot| slot.id == predecessor_id))?
        .get();
    let game_id = &predecessor.game.as_ref()?.game_id;
    let game = short_game_label(tournament, &predecessor);
    // TODO: i18n once copy is approved.
    Some((
        format!("Finish {game} first"),
        format!("/game/{}", game_id.0),
    ))
}

fn schedule_summary(
    i18n: I18nContext<Locale, I18nKeys>,
    tournament: TournamentState,
    slot: &SlotResponse,
    pending: Option<&ScheduleResponse>,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> ScheduleSummary {
    if let Some(scheduled_at) = slot.scheduled_at {
        let state = scheduled_timing_state(scheduled_at, now);
        if matches!(state, ScheduleState::Starting | ScheduleState::Late) {
            let predecessor = slot
                .game
                .is_none()
                .then(|| predecessor_link(tournament, slot))
                .flatten();
            let (label, predecessor_game_href) = if let Some((label, href)) = predecessor {
                (label, Some(href))
            } else if slot.game.is_none() {
                // TODO: i18n once copy is approved.
                (String::from("Not available yet"), None)
            } else if state == ScheduleState::Starting {
                // TODO: i18n once copy is approved.
                (String::from("Starting"), None)
            } else {
                // TODO: i18n once copy is approved.
                (String::from("Late"), None)
            };
            return ScheduleSummary {
                state,
                relevant_at: Some(scheduled_at),
                label,
                detail: Some({
                    // TODO: i18n once copy is approved.
                    let request = pending
                        .map(|offer| {
                            if !offer.has_future_candidate(now) {
                                " · Proposed times expired"
                            } else if offer.opponent_id == user_id {
                                " · Reschedule request"
                            } else {
                                " · Reschedule proposed"
                            }
                        })
                        .unwrap_or_default();
                    format!(
                        "Agreed · {}{request}",
                        format_schedule_time(i18n.get_locale(), scheduled_at)
                    )
                }),
                predecessor_game_href,
            };
        }
        if let Some(pending) = pending {
            return pending_summary(i18n, pending, user_id, Some(scheduled_at), now);
        }
        return ScheduleSummary {
            state: ScheduleState::Scheduled,
            relevant_at: Some(scheduled_at),
            // TODO: i18n once copy is approved.
            label: String::from("Scheduled"),
            detail: Some(format_schedule_time(i18n.get_locale(), scheduled_at)),
            predecessor_game_href: None,
        };
    }
    if let Some(offer) = pending {
        return pending_summary(i18n, offer, user_id, None, now);
    }
    let overdue = slot.deadline_at.is_some_and(|deadline| deadline <= now);
    ScheduleSummary {
        state: if overdue {
            ScheduleState::Overdue
        } else {
            ScheduleState::Unscheduled
        },
        relevant_at: slot.deadline_at,
        // TODO: i18n once copy is approved.
        label: String::from("Needs a time"),
        detail: slot.deadline_at.map(|deadline| {
            // TODO: i18n once copy is approved.
            format!(
                "{} · {}",
                if overdue {
                    "Play-by time passed"
                } else {
                    "Play by"
                },
                format_schedule_time(i18n.get_locale(), deadline),
            )
        }),
        predecessor_game_href: None,
    }
}

fn compare_relevant_at(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Ordering {
    left.is_none()
        .cmp(&right.is_none())
        .then_with(|| left.cmp(&right))
}

fn compare_summaries(left: &ScheduleSummary, right: &ScheduleSummary) -> Ordering {
    left.state
        .cmp(&right.state)
        .then_with(|| compare_relevant_at(left.relevant_at, right.relevant_at))
}

pub(crate) fn participant_has_schedulable_slots(
    tournament: TournamentState,
    user_id: Uuid,
) -> bool {
    fixed_slot_fields(tournament).into_iter().any(|slot| {
        slot.try_get()
            .is_some_and(|slot| schedule_is_available(&slot, user_id))
    })
}

pub(crate) fn participant_schedule_opponents_needing_time(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> usize {
    let tournament_id = tournament.tournament_id();
    let offers = schedules.with(|schedules| offers_by_slot(schedules, &tournament_id));
    opponent_slots(tournament, user_id)
        .into_iter()
        .filter(|opponent| {
            opponent.slots.iter().any(|slot| {
                slot.with(|slot| {
                    slot.scheduled_at.is_none()
                        || offers
                            .get(&slot.id)
                            .and_then(|offers| offers.latest_pending.as_ref())
                            .is_some_and(|offer| {
                                offer.opponent_id == user_id && offer.has_future_candidate(now)
                            })
                })
            })
        })
        .count()
}

fn opponent_slots(tournament: TournamentState, user_id: Uuid) -> Vec<OpponentSlots> {
    let slot_order = slot_ordering(tournament);
    let memberships = tournament.common.memberships().get();
    let mut grouped = HashMap::<Uuid, (UserResponse, Vec<ArcField<SlotResponse>>)>::new();
    for slot_field in fixed_slot_fields(tournament) {
        let Some(slot) = slot_field.try_get() else {
            continue;
        };
        if !schedule_is_available(&slot, user_id) {
            continue;
        }
        let opponent_id = if slot.white() == user_id {
            slot.black()
        } else {
            slot.white()
        };
        let Some(opponent) = memberships.players.get(&opponent_id).cloned() else {
            continue;
        };
        grouped
            .entry(opponent_id)
            .or_insert_with(|| (opponent, Vec::new()))
            .1
            .push(slot_field);
    }
    grouped
        .into_values()
        .map(|(user, mut slots)| {
            slots.sort_by_key(|slot| slot.try_get().map(|slot| (slot_order(&slot), slot.id)));
            let slot_ids = slots
                .iter()
                .filter_map(|slot| slot.try_get_untracked().map(|slot| slot.id))
                .collect();
            OpponentSlots {
                user,
                slot_ids,
                slots,
            }
        })
        .collect()
}

fn opponent_groups(
    i18n: I18nContext<Locale, I18nKeys>,
    tournament: TournamentState,
    opponents: &[OpponentSlots],
    slot_offers: &HashMap<Uuid, SlotOffers>,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<OpponentGroup> {
    let mut opponents = opponents
        .iter()
        .map(|opponent| {
            let summary = opponent
                .slots
                .iter()
                .filter_map(|slot| slot.try_get())
                .filter(|slot| slot.game.is_some())
                .enumerate()
                .map(|(order, slot)| {
                    let pending = slot_offers
                        .get(&slot.id)
                        .and_then(|slot| slot.latest_pending.as_ref());
                    (
                        schedule_summary(i18n, tournament, &slot, pending, user_id, now),
                        order,
                    )
                })
                .min_by(|(left_summary, left_order), (right_summary, right_order)| {
                    compare_summaries(left_summary, right_summary)
                        .then_with(|| left_order.cmp(right_order))
                })
                .map(|(summary, _)| summary);
            OpponentGroup {
                user: opponent.user.clone(),
                slot_ids: opponent
                    .slots
                    .iter()
                    .filter_map(|slot| slot.try_get_untracked().map(|slot| slot.id))
                    .collect(),
                summary,
            }
        })
        .collect::<Vec<_>>();
    opponents.sort_by(|left, right| {
        match (&left.summary, &right.summary) {
            (Some(left), Some(right)) => compare_summaries(left, right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| {
            left.user
                .username
                .to_lowercase()
                .cmp(&right.user.username.to_lowercase())
        })
        .then_with(|| left.user.uid.cmp(&right.user.uid))
    });
    opponents
}

fn status_class(state: ScheduleState) -> &'static str {
    match state {
        ScheduleState::Starting => "text-pillbug-teal",
        ScheduleState::Late | ScheduleState::Overdue => "text-ladybug-red",
        ScheduleState::ReplyNeeded | ScheduleState::Unscheduled => {
            "text-orange-700 dark:text-orange-300"
        }
        ScheduleState::ProposalSent => "text-gray-600 dark:text-gray-300",
        ScheduleState::Scheduled => "text-teal-700 dark:text-teal-300",
    }
}

fn sidebar_label(summary: &ScheduleSummary) -> String {
    if summary.state == ScheduleState::Scheduled {
        // TODO: i18n once copy is approved.
        summary.relevant_at.map_or_else(
            || summary.label.clone(),
            |scheduled_at| {
                format!(
                    "Next: {}",
                    scheduled_at.with_timezone(&Local).format("%b %-d"),
                )
            },
        )
    } else {
        summary.label.clone()
    }
}

fn open_schedule_editor(
    selected_slot: RwSignal<Option<Uuid>>,
    dialog_el: NodeRef<Dialog>,
    slot_id: Uuid,
) {
    if selected_slot.try_set(Some(slot_id)).is_some() {
        return;
    }
    if let Some(dialog) = dialog_el.try_get().flatten() {
        let _ = dialog.show_modal();
    }
}

#[component]
fn ParticipantOpponentLink(opponent_id: Uuid) -> impl IntoView {
    let context = expect_context::<ScheduleContext>();
    view! {
        {move || {
            let route = current_schedule_route(&context)?;
            let current_username = match &route {
                ScheduleRoute::Index => None,
                ScheduleRoute::Opponent(username) => Some(username),
                ScheduleRoute::Invalid => return None,
            };
            context
                .opponents
                .with(|opponents| {
                    opponents.iter().find(|opponent| opponent.user.uid == opponent_id).cloned()
                })
                .map(|opponent| {
                    let username = opponent.user.username;
                    let href = format!("{}/{}", context.schedule_root.get_value(), username);
                    let route_selected = current_username
                        .is_some_and(|current| current.eq_ignore_ascii_case(&username));
                    let default_selected = current_username.is_none()
                        && context
                            .opponents
                            .with(|opponents| {
                                opponents.first().is_some_and(|first| first.user.uid == opponent_id)
                            });
                    let title = opponent
                        .summary
                        .as_ref()
                        .and_then(|summary| summary.detail.clone());
                    let status = opponent
                        .summary
                        .as_ref()
                        .map(|summary| { (status_class(summary.state), sidebar_label(summary)) });
                    view! {
                        <A
                            href=href
                            scroll=false
                            attr:title=title
                            attr:aria-current=route_selected.then_some("page")
                            attr:class=if route_selected {
                                "grid grid-cols-[minmax(0,1fr)_auto] gap-2 items-center px-3 py-3 border-l-2 border-pillbug-teal bg-pillbug-teal/10 no-link-style"
                            } else if default_selected {
                                "grid grid-cols-[minmax(0,1fr)_auto] gap-2 items-center px-3 py-3 border-l-2 border-transparent text-gray-800 hover:bg-blue-light/70 dark:text-gray-100 dark:hover:bg-pillbug-teal/15 no-link-style tournament-two:border-pillbug-teal tournament-two:bg-pillbug-teal/10"
                            } else {
                                "grid grid-cols-[minmax(0,1fr)_auto] gap-2 items-center px-3 py-3 border-l-2 border-transparent text-gray-800 hover:bg-blue-light/70 dark:text-gray-100 dark:hover:bg-pillbug-teal/15 no-link-style"
                            }
                        >
                            <span class="text-sm font-semibold truncate">{username}</span>
                            {status
                                .map(|(class, label)| {
                                    view! {
                                        <span class=format!(
                                            "truncate max-w-28 text-xs font-semibold text-right {class}",
                                        )>{label}</span>
                                    }
                                })}
                        </A>
                    }
                })
        }}
    }
}

#[component]
fn ParticipantSlotRow(slot_id: Uuid) -> impl IntoView {
    let context = expect_context::<ScheduleContext>();
    let i18n = use_i18n();
    view! {
        {move || {
            current_schedule_route(&context)?;
            let current_user = context.user_id.get()?;
            let slot = context
                .slots
                .with(|slots| {
                    slots
                        .iter()
                        .find(|slot| slot.with_untracked(|slot| slot.id == slot_id))
                        .cloned()
                })?
                .get();
            let identity = slot_card_label(i18n, context.tournament, &slot)
                .unwrap_or_else(|| short_game_label(context.tournament, &slot));
            let (summary, editor_label) = context
                .slot_offers
                .with(|slot_offers| {
                    let pending = slot_offers
                        .get(&slot_id)
                        .and_then(|slot| slot.latest_pending.as_ref());
                    let summary = schedule_summary(
                        i18n,
                        context.tournament,
                        &slot,
                        pending,
                        current_user,
                        context.now.get(),
                    );
                    let editor_label = editor_button_label(
                        pending,
                        current_user,
                        slot.scheduled_at.is_some(),
                        context.now.get(),
                    );
                    (summary, editor_label)
                });
            let game_href = slot.game.as_ref().map(|game| format!("/game/{}", game.game_id.0));
            let ScheduleSummary { state, label, detail, predecessor_game_href, .. } = summary;
            let class = status_class(state);
            let editor_primary = state == ScheduleState::ReplyNeeded;
            let selected_slot = context.selected_slot;
            let dialog_el = context.dialog_el;
            Some(
                view! {
                    <article
                        id=format!("tournament-slot-{slot_id}")
                        tabindex="-1"
                        class="grid gap-2 items-center py-3 px-3 border-t first:border-t-0 scroll-mt-4 border-black/10 target:ring-2 target:ring-pillbug-teal sm:grid-cols-[minmax(0,1fr)_auto] dark:border-white/10 dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even odd:bg-even-light even:bg-odd-light"
                    >
                        <div class="min-w-0">
                            <p class="text-xs font-semibold text-gray-500 dark:text-gray-400">
                                {identity}
                            </p>
                            {if let Some(href) = predecessor_game_href {
                                view! {
                                    <a
                                        class=format!("text-sm font-semibold ui-text-link {class}")
                                        href=href
                                    >
                                        {label}
                                    </a>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <p class=format!("text-sm font-semibold {class}")>{label}</p>
                                }
                                    .into_any()
                            }}
                            {detail
                                .map(|detail| {
                                    view! {
                                        <p class="text-xs text-gray-600 dark:text-gray-300">
                                            {detail}
                                        </p>
                                    }
                                })}
                        </div>
                        <div class="flex flex-wrap gap-1.5 sm:justify-end">
                            {game_href
                                .map(|href| {
                                    view! {
                                        <a
                                            class="ui-button ui-button-secondary ui-button-sm"
                                            href=href
                                        >
                                            // TODO: i18n once copy is approved.
                                            "Game"
                                        </a>
                                    }
                                })}
                            <button
                                type="button"
                                class=if editor_primary {
                                    "ui-button ui-button-primary ui-button-sm"
                                } else {
                                    "ui-button ui-button-secondary ui-button-sm"
                                }
                                on:click=move |_| {
                                    if current_schedule_route(&context).is_some() {
                                        open_schedule_editor(selected_slot, dialog_el, slot_id);
                                    }
                                }
                            >
                                {editor_label}
                            </button>
                        </div>
                    </article>
                },
            )
        }}
    }
}

#[component]
fn ParticipantOpponentDetail(opponent: OpponentGroup, show_mobile_back: bool) -> impl IntoView {
    let context = expect_context::<ScheduleContext>();
    let username = opponent.user.username.clone();
    let count = opponent.slot_ids.len();
    let schedule_root = context.schedule_root.get_value();
    view! {
        <div class="flex flex-col min-h-full">
            <header class="flex gap-3 justify-between items-center py-3 px-3 border-b sm:px-4 shrink-0 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-raised">
                <div class="min-w-0">
                    <Show when=move || show_mobile_back>
                        <A
                            href=schedule_root.clone()
                            scroll=false
                            attr:class="inline-flex gap-1 items-center mb-1 text-sm font-semibold no-link-style text-pillbug-teal tournament-two:hidden"
                        >
                            // TODO: i18n once copy is approved.
                            "‹ All opponents"
                        </A>
                    </Show>
                    <UserIdentity user=opponent.user class="h-9" />
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                        // TODO: i18n once copy is approved.
                        {format!("{count} game{} to schedule", if count == 1 { "" } else { "s" })}
                    </p>
                </div>
                <MessageButton username />
            </header>
            <div>
                {opponent
                    .slot_ids
                    .into_iter()
                    .map(|slot_id| view! { <ParticipantSlotRow slot_id /> })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn ParticipantScheduleLayout(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    user_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    reveal_slot_fragment();
    let i18n = use_i18n();
    let location = use_location();
    let params = use_params_map();
    let pathname = location.pathname;
    let source_tournament_id = tournament.tournament_id();
    let tournament_id = params
        .get_untracked()
        .get("nanoid")
        .map(TournamentId)
        .unwrap_or(source_tournament_id);
    let route_tournament_id = tournament_id.clone();
    let route_active = Signal::derive(move || {
        let pathname = pathname.get();
        schedule_route(&pathname, &route_tournament_id).is_some()
            && tournament
                .common
                .lifecycle()
                .try_get()
                .is_some_and(|lifecycle| lifecycle.tournament_id == route_tournament_id)
    });
    let slots = Signal::derive(move || fixed_slot_fields(tournament));
    let schedule_tournament_id = tournament_id.clone();
    let slot_offers = Memo::new(move |_| {
        schedules.with(|schedules| offers_by_slot(schedules, &schedule_tournament_id))
    });
    let opponent_slots = Memo::new(move |_| {
        user_id
            .get()
            .map(|user_id| opponent_slots(tournament, user_id))
            .unwrap_or_default()
    });
    let now = use_ticking_now();
    let opponents = Memo::new(move |_| {
        user_id
            .get()
            .map(|user_id| {
                opponent_slots.with(|opponent_slots| {
                    slot_offers.with(|slot_offers| {
                        opponent_groups(
                            i18n,
                            tournament,
                            opponent_slots,
                            slot_offers,
                            user_id,
                            now.get(),
                        )
                    })
                })
            })
            .unwrap_or_default()
    });

    let selected_slot = RwSignal::new(None::<Uuid>);
    let dialog_el = NodeRef::<Dialog>::new();
    let schedule_root = format!("/tournament/{}/schedule", tournament_id.0);
    let context = ScheduleContext {
        tournament_id: StoredValue::new(tournament_id.clone()),
        tournament,
        slots,
        slot_offers,
        user_id,
        now,
        opponents,
        selected_slot,
        dialog_el,
        schedule_root: StoredValue::new(schedule_root.clone()),
        pathname,
        route_active,
    };
    provide_context(context);

    let requested = requested_schedule_fragment();
    let auto_opened_schedule = RwSignal::new(None::<(Uuid, String)>);
    let auto_tournament_id = tournament_id.clone();
    let auto_tournament = tournament;
    let auto_slots = slots;
    let auto_slot_offers = slot_offers;
    let auto_user = user_id;
    let auto_selected_slot = selected_slot;
    Effect::new(move |_| {
        let route = params.get();
        if route.get("nanoid").as_deref() != Some(auto_tournament_id.0.as_str()) {
            auto_opened_schedule.set(None);
            return;
        }
        let Some(slot_id) = requested.get() else {
            auto_opened_schedule.set(None);
            return;
        };
        let Some(route_username) = route.get("username") else {
            return;
        };
        if auto_opened_schedule.get_untracked() == Some((slot_id, route_username.clone())) {
            return;
        }
        let Some(current_user) = auto_user.get() else {
            return;
        };
        let matches_route = auto_slots.with(|slots| {
            slots
                .iter()
                .find(|slot| slot.with_untracked(|slot| slot.id == slot_id))
                .is_some_and(|slot| {
                    let Some(slot) = slot.try_get() else {
                        return false;
                    };
                    if !schedule_is_available(&slot, current_user) {
                        return false;
                    }
                    let opponent_id = if slot.white() == current_user {
                        slot.black()
                    } else {
                        slot.white()
                    };
                    auto_tournament
                        .common
                        .memberships()
                        .get()
                        .players
                        .get(&opponent_id)
                        .is_some_and(|opponent| {
                            opponent.username.eq_ignore_ascii_case(&route_username)
                        })
                })
        });
        if !matches_route {
            return;
        }
        if !auto_slot_offers.with(|index| {
            index
                .get(&slot_id)
                .is_some_and(|schedule| schedule.has_history)
        }) {
            return;
        }
        let Some(dialog) = dialog_el.try_get().flatten() else {
            return;
        };
        auto_selected_slot.set(Some(slot_id));
        if dialog.open() || dialog.show_modal().is_ok() {
            auto_opened_schedule.set(Some((slot_id, route_username)));
        }
    });

    let opponent_ids = move || {
        if current_schedule_route(&context).is_none() {
            return Vec::new();
        }
        context
            .opponents
            .try_with(|opponents| {
                opponents
                    .iter()
                    .map(|opponent| opponent.user.uid)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    view! {
        <div class="space-y-3">
            // TODO: i18n once copy is approved.
            <h2 class="break-words ui-page-title">
                {move || format!("Scheduling · {}", tournament.common.lifecycle().get().name)}
            </h2>
            <div class="overflow-hidden rounded-lg border shadow-sm border-black/10 bg-even-light/95 tournament-two:grid tournament-two:h-[min(68vh,44rem)] tournament-two:grid-cols-[minmax(15rem,19rem)_minmax(0,1fr)] tournament-two:grid-rows-[minmax(0,1fr)] dark:border-white/10 dark:bg-surface-panel">
                <aside class=move || {
                    format!(
                        "h-full min-h-0 min-w-0 overflow-hidden flex-col border-black/10 bg-even-light/95 dark:border-white/10 dark:bg-surface-panel tournament-two:!flex tournament-two:border-r {}",
                        if current_schedule_route(&context) == Some(ScheduleRoute::Index) {
                            "flex"
                        } else {
                            "hidden"
                        },
                    )
                }>
                    <div class="flex gap-2 justify-between items-center py-2.5 px-3 border-b border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-raised">
                        // TODO: i18n once copy is approved.
                        <h3 class="font-bold">"Opponents"</h3>
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                            {move || {
                                if current_schedule_route(&context).is_none() {
                                    return 0;
                                }
                                context.opponents.try_with(Vec::len).unwrap_or_default()
                            }}
                        </span>
                    </div>
                    <nav class="overflow-y-auto flex-1 min-h-0 divide-y divide-black/10 dark:divide-white/10">
                        <Show
                            when=move || {
                                if current_schedule_route(&context).is_none() {
                                    return false;
                                }
                                context
                                    .opponents
                                    .try_with(|opponents| !opponents.is_empty())
                                    .unwrap_or(false)
                            }
                            fallback=|| {
                                view! {
                                    // TODO: i18n once copy is approved.
                                    <p class="p-4 text-sm text-gray-600 dark:text-gray-300">
                                        "Nothing to schedule"
                                    </p>
                                }
                            }
                        >
                            <For each=opponent_ids key=|opponent_id| *opponent_id let:opponent_id>
                                <ParticipantOpponentLink opponent_id />
                            </For>
                        </Show>
                    </nav>
                </aside>
                <main class=move || {
                    let route = current_schedule_route(&context);
                    format!(
                        "h-full min-w-0 min-h-0 flex-col overflow-y-auto bg-even-light/95 dark:bg-surface-panel {} tournament-two:!flex",
                        match route {
                            Some(ScheduleRoute::Index) => "hidden",
                            Some(ScheduleRoute::Opponent(_)) => "flex",
                            Some(ScheduleRoute::Invalid) => "hidden",
                            None => "hidden",
                        },
                    )
                }>
                    <Outlet />
                </main>
            </div>
        </div>
        <ScheduleEditorDialog tournament schedules user_id selected_slot dialog_el route_active />
    }
}

#[component]
pub fn ParticipantScheduleIndex() -> impl IntoView {
    let context = expect_context::<ScheduleContext>();
    view! {
        {move || {
            if current_schedule_route(&context) != Some(ScheduleRoute::Index) {
                return None;
            }
            match context.opponents.with(|opponents| opponents.first().cloned()) {
                Some(opponent) => {
                    Some(
                        view! { <ParticipantOpponentDetail opponent show_mobile_back=false /> }
                            .into_any(),
                    )
                }
                None => {
                    Some(
                        view! {
                            <div class="flex flex-1 justify-center items-center p-6">
                                // TODO: i18n once copy is approved.
                                <p class="text-sm text-gray-600 dark:text-gray-300">
                                    "Nothing to schedule"
                                </p>
                            </div>
                        }
                            .into_any(),
                    )
                }
            }
        }}
    }
}

#[component]
pub fn ParticipantScheduleOpponent() -> impl IntoView {
    let context = expect_context::<ScheduleContext>();
    view! {
        {move || {
            let ScheduleRoute::Opponent(username) = current_schedule_route(&context)? else {
                return None;
            };
            let selected = {
                context
                    .opponents
                    .with(|opponents| {
                        opponents
                            .iter()
                            .find(|opponent| opponent.user.username.eq_ignore_ascii_case(&username))
                            .cloned()
                    })
            };
            Some(
                match selected {
                    Some(opponent) => {
                        view! { <ParticipantOpponentDetail opponent show_mobile_back=true /> }
                            .into_any()
                    }
                    None => {
                        view! {
                            <div class="p-3 space-y-3 sm:p-4">
                                <A
                                    href=context.schedule_root.get_value()
                                    scroll=false
                                    attr:class="inline-flex text-sm font-semibold no-link-style text-pillbug-teal tournament-two:hidden"
                                >
                                    // TODO: i18n once copy is approved.
                                    "‹ All opponents"
                                </A>
                                // TODO: i18n once copy is approved.
                                <p class="text-sm text-gray-600 dark:text-gray-300">
                                    "This opponent has no remaining scheduling tasks."
                                </p>
                            </div>
                        }
                            .into_any()
                    }
                },
            )
        }}
    }
}
