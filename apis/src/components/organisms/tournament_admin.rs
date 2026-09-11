use crate::{
    common::{tournament_path_matches, TournamentAction},
    components::{
        molecules::{
            invite_user::InviteUser,
            pagination_controls::PaginationControls,
            user_identity::UserIdentity,
        },
        organisms::tournament_withdrawal::TournamentWithdrawal,
    },
    providers::{
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        TournamentCommonStoreFields,
        TournamentState,
    },
};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use shared_types::{tournament::Format, TournamentStatus};

const ROSTER_PAGE_SIZE: usize = 12;
// Fixed maximum tracks keep a single invitation as compact as a populated roster.
pub(super) const MEMBERSHIP_GRID_CLASS: &str =
    "grid gap-x-4 gap-y-2 grid-cols-[repeat(auto-fill,minmax(min(100%,24rem),24rem))]";
pub(super) const MEMBERSHIP_ROW_CLASS: &str = "flex gap-3 items-center justify-between p-2 min-w-0 min-h-12 rounded border border-black/10 bg-black/[0.02] dark:border-white/10 dark:bg-white/[0.02]";

#[derive(Clone, Copy, PartialEq, Eq)]
enum RosterView {
    Accepted,
    Pending,
    Declined,
    Withdrawn,
}

#[component]
pub fn TournamentAdminControls(
    user_is_organizer_or_admin: Signal<bool>,
    tournament: TournamentState,
    editable: Signal<bool>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth = expect_context::<AuthContext>();
    let viewer = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let tournament_id = StoredValue::new(tournament.tournament_id());
    let pathname = use_location().pathname;
    let route_active = Signal::derive(move || {
        let id = tournament_id.get_value();
        tournament_path_matches(&pathname.get(), &id)
            && tournament.common.lifecycle().get().tournament_id == id
    });
    let pre_start = Signal::derive(move || {
        tournament.common.lifecycle().get().status == TournamentStatus::NotStarted
    });
    let fixed_field = tournament.format.format() != Format::Arena;
    let field_full = Signal::derive(move || {
        tournament
            .common
            .lifecycle()
            .get()
            .seats
            .and_then(|capacity| usize::try_from(capacity).ok())
            .is_some_and(|capacity| tournament.common.memberships().get().players.len() >= capacity)
    });
    let allowed = Signal::derive(move || {
        fixed_field && route_active.get() && user_is_organizer_or_admin.get() && editable.get()
    });
    let selected = RwSignal::new(RosterView::Accepted);
    let active = Signal::derive(move || match (pre_start.get(), selected.get()) {
        (false, RosterView::Pending | RosterView::Declined) | (true, RosterView::Withdrawn) => {
            RosterView::Accepted
        }
        (_, selected) => selected,
    });
    let query = RwSignal::new(String::new());
    let page = RwSignal::new(1usize);
    let users_for = move |section| {
        let memberships = tournament.common.memberships().get();
        match section {
            RosterView::Accepted => memberships
                .players
                .into_values()
                .filter(|user| !memberships.withdrawn.contains(&user.uid))
                .collect::<Vec<_>>(),
            RosterView::Pending => memberships.invitees,
            RosterView::Declined => memberships.declined_invitees,
            RosterView::Withdrawn => memberships
                .players
                .into_values()
                .filter(|user| memberships.withdrawn.contains(&user.uid))
                .collect(),
        }
    };
    let users = Signal::derive(move || users_for(active.get()));
    let filtered = Memo::new(move |_| {
        let query = query.get().trim().to_lowercase();
        let mut users = users.get();
        users.retain(|user| user.username.to_lowercase().contains(&query));
        users.sort_by_cached_key(|user| (user.username.to_lowercase(), user.uid));
        users
    });
    Effect::new(move |_| {
        let last = filtered.with(Vec::len).div_ceil(ROSTER_PAGE_SIZE).max(1);
        if page.get_untracked() > last {
            page.set(last);
        }
    });
    view! {
        <section class="space-y-4">
            <div class="flex flex-wrap gap-3 justify-between items-center">
                // TODO: i18n once copy is approved.
                <div class="flex flex-wrap gap-1" role="group" aria-label="Player status">
                    {[
                        (RosterView::Accepted, "Accepted"),
                        (RosterView::Pending, "Pending"),
                        (RosterView::Declined, "Declined"),
                        (RosterView::Withdrawn, "Withdrawn"),
                    ]
                        .into_iter()
                        .map(|(section, label)| {
                            view! {
                                <Show when=move || match section {
                                    RosterView::Accepted => true,
                                    RosterView::Pending | RosterView::Declined => {
                                        fixed_field && pre_start.get()
                                    }
                                    RosterView::Withdrawn => fixed_field && !pre_start.get(),
                                }>
                                    <button
                                        type="button"
                                        class=move || {
                                            if active.get() == section {
                                                "ui-button ui-button-secondary ui-button-sm"
                                            } else {
                                                "ui-button ui-button-ghost ui-button-sm"
                                            }
                                        }
                                        aria-pressed=move || (active.get() == section).to_string()
                                        on:click=move |_| {
                                            selected.set(section);
                                            query.set(String::new());
                                            page.set(1);
                                        }
                                    >
                                        // TODO: i18n once copy is approved.
                                        {move || {
                                            let label = if section == RosterView::Accepted {
                                                if !fixed_field {
                                                    "Joined"
                                                } else if !pre_start.get() {
                                                    "Playing"
                                                } else {
                                                    label
                                                }
                                            } else {
                                                label
                                            };
                                            format!("{label} ({})", users_for(section).len())
                                        }}
                                    </button>
                                </Show>
                            }
                        })
                        .collect_view()}
                </div>
                <Show when=move || allowed.get()>
                    <Show
                        when=move || !field_full.get()
                        fallback=|| {
                            view! {
                                // TODO: i18n once copy is approved.
                                <p class="text-sm text-gray-600 dark:text-gray-300">
                                    "The field is full."
                                </p>
                            }
                        }
                    >
                        <div class="w-full min-w-0 sm:w-72">
                            <InviteUser tournament />
                        </div>
                    </Show>
                </Show>
            </div>
            <Show when=move || {
                users.with(Vec::len) > ROSTER_PAGE_SIZE || !query.get().is_empty()
            }>
                // TODO: i18n once copy is approved.
                <input
                    type="search"
                    class="w-full sm:max-w-xs ui-field-input"
                    placeholder="Search players"
                    aria-label="Search players"
                    prop:value=move || query.get()
                    on:input=move |event| {
                        query.set(event_target_value(&event));
                        page.set(1);
                    }
                />
            </Show>
            <Show when=move || filtered.with(Vec::is_empty)>
                <p class="py-4 text-sm text-gray-600 dark:text-gray-300" role="status">
                    // TODO: i18n once copy is approved.
                    {move || {
                        if !query.get().trim().is_empty() {
                            "No players match your search."
                        } else {
                            match active.get() {
                                RosterView::Accepted if !fixed_field => {
                                    "No players yet. Players can join from Overview."
                                }
                                RosterView::Accepted if pre_start.get() => {
                                    "No accepted players yet. Invite a player to get started."
                                }
                                RosterView::Accepted => "No active players.",
                                RosterView::Pending => "No pending invitations.",
                                RosterView::Declined => "No declined invitations.",
                                RosterView::Withdrawn => "No withdrawn players.",
                            }
                        }
                    }}
                </p>
            </Show>
            <Show when=move || !filtered.with(Vec::is_empty)>
                <div class=MEMBERSHIP_GRID_CLASS>
                    <For
                        each=move || {
                            filtered
                                .with(|users| {
                                    users
                                        .iter()
                                        .skip((page.get() - 1) * ROSTER_PAGE_SIZE)
                                        .take(ROSTER_PAGE_SIZE)
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                        }
                        key=move |user| (user.uid, active.get() as u8)
                        let:user
                    >
                        {
                            let uid = user.uid;
                            view! {
                                <div class=MEMBERSHIP_ROW_CLASS>
                                    <div class="min-w-0">
                                        <UserIdentity user link_class="truncate" />
                                    </div>
                                    <Show when=move || {
                                        allowed.get() && viewer.get().is_some_and(|id| id != uid)
                                    }>
                                        <button
                                            type="button"
                                            class="shrink-0 ui-button ui-button-ghost ui-button-sm"
                                            prop:disabled=move || {
                                                active.get() == RosterView::Declined && field_full.get()
                                            }
                                            on:click=move |_| {
                                                if !allowed.get_untracked()
                                                    || viewer.get_untracked() == Some(uid)
                                                {
                                                    return;
                                                }
                                                let id = tournament_id.get_value();
                                                let action = match active.get_untracked() {
                                                    RosterView::Accepted => TournamentAction::Kick(id, uid),
                                                    RosterView::Pending => {
                                                        TournamentAction::InvitationRetract(id, uid)
                                                    }
                                                    RosterView::Declined if !field_full.get_untracked() => {
                                                        TournamentAction::InvitationCreate(id, uid)
                                                    }
                                                    _ => return,
                                                };
                                                api.get().tournament(action);
                                            }
                                        >
                                            // TODO: i18n once copy is approved.
                                            {move || match active.get() {
                                                RosterView::Accepted => "Remove",
                                                RosterView::Pending => "Cancel invite",
                                                RosterView::Declined => "Re-invite",
                                                RosterView::Withdrawn => "",
                                            }}
                                        </button>
                                    </Show>
                                    <Show when=move || !pre_start.get()>
                                        <TournamentWithdrawal
                                            common=tournament.common
                                            format=tournament.format
                                            organizer=user_is_organizer_or_admin
                                            player_id=uid
                                        />
                                    </Show>
                                </div>
                            }
                        }
                    </For>
                </div>
            </Show>
            <Show when=move || { filtered.with(Vec::len) > ROSTER_PAGE_SIZE }>
                <PaginationControls
                    page=page.into()
                    total=Signal::derive(move || filtered.with(Vec::len))
                    page_size=ROSTER_PAGE_SIZE
                    on_page_change=Callback::new(move |next| page.set(next))
                />
            </Show>
            <Show when=move || fixed_field && pre_start.get()>
                // TODO: i18n once copy is approved.
                <p class="text-xs text-gray-600 dark:text-gray-300">
                    "Only accepted players count toward the field."
                </p>
            </Show>
        </section>
    }
}
