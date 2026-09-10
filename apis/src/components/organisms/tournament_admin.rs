use crate::{
    common::{TournamentAction, UserAction},
    components::molecules::{
        invite_user::InviteUser,
        pagination_controls::PaginationControls,
        user_row::UserRow,
    },
    providers::{ApiRequestsProvider, TournamentCommonStoreFields, TournamentState},
    responses::UserResponse,
};
use leptos::prelude::*;
use uuid::Uuid;

const ROSTER_PAGE_SIZE: usize = 8;

#[component]
fn MembershipSection(
    title: &'static str,
    users: Signal<Vec<UserResponse>>,
    actions: Signal<Vec<UserAction>>,
    #[prop(optional)] collapsed: bool,
    #[prop(optional)] reinvite: Option<Callback<Uuid>>,
    #[prop(optional)] reinvite_disabled: Option<Signal<bool>>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let page = RwSignal::new(1usize);
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
        <details open=!collapsed class="space-y-2 ui-setting-group">
            // TODO: i18n once copy is approved.
            <summary class="font-bold cursor-pointer">
                {move || format!("{title} ({})", users.with(Vec::len))}
            </summary>
            <div class="space-y-2">
                <Show when=move || {
                    users.with(Vec::len) > ROSTER_PAGE_SIZE || !query.get().is_empty()
                }>
                    // TODO: i18n once copy is approved.
                    <input
                        type="search"
                        class="w-full sm:max-w-sm ui-field-input"
                        placeholder=format!("Search {}", title.to_lowercase())
                        prop:value=move || query.get()
                        on:input=move |event| {
                            query.set(event_target_value(&event));
                            page.set(1);
                        }
                    />
                </Show>
                <Show when=move || filtered.with(Vec::is_empty)>
                    // TODO: i18n once copy is approved.
                    <p class="text-sm text-gray-600 dark:text-gray-300">
                        {move || {
                            if query.get().trim().is_empty() {
                                "No players in this section."
                            } else {
                                "No players match your search."
                            }
                        }}
                    </p>
                </Show>
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
                    key=|user| user.uid
                    let:user
                >
                    {
                        let user_id = user.uid;
                        view! {
                            <div class="flex gap-2 items-center">
                                <div class="flex-1 min-w-0">
                                    {move || {
                                        view! {
                                            <UserRow actions=actions.get() user=user.clone() />
                                        }
                                    }}
                                </div>
                                {reinvite
                                    .map(|reinvite| {
                                        view! {
                                            // TODO: i18n once copy is approved.
                                            <button
                                                type="button"
                                                class="ui-button ui-button-secondary ui-button-sm"
                                                prop:disabled=move || {
                                                    reinvite_disabled.is_some_and(|disabled| disabled.get())
                                                }
                                                on:click=move |_| reinvite.run(user_id)
                                            >
                                                "Re-invite"
                                            </button>
                                        }
                                    })}
                            </div>
                        }
                    }
                </For>
                <Show when=move || { filtered.with(Vec::len) > ROSTER_PAGE_SIZE }>
                    <PaginationControls
                        page=page.into()
                        total=Signal::derive(move || filtered.with(Vec::len))
                        page_size=ROSTER_PAGE_SIZE
                        on_page_change=Callback::new(move |next| page.set(next))
                    />
                </Show>
            </div>
        </details>
    }
}

#[component]
pub fn TournamentAdminControls(
    user_is_organizer_or_admin: Signal<bool>,
    tournament: TournamentState,
    editable: Signal<bool>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let tournament_id = StoredValue::new(tournament.tournament_id());
    let field_full = Signal::derive(move || {
        let lifecycle = tournament.common.lifecycle().get();
        lifecycle
            .seats
            .and_then(|capacity| usize::try_from(capacity).ok())
            .is_some_and(|capacity| tournament.common.memberships().get().players.len() >= capacity)
    });
    let allowed = Signal::derive(move || user_is_organizer_or_admin.get() && editable.get());
    let user_kick = Signal::derive(move || {
        if allowed.get() {
            vec![UserAction::Kick(tournament.tournament_id())]
        } else {
            vec![]
        }
    });
    let user_uninvite = Signal::derive(move || {
        if allowed.get() {
            vec![UserAction::Uninvite(tournament.tournament_id())]
        } else {
            vec![]
        }
    });
    let reinvite_disabled = Signal::derive(move || !allowed.get() || field_full.get());
    let reinvite = Callback::new(move |user_id| {
        if !reinvite_disabled.get_untracked() {
            api.get().tournament(TournamentAction::InvitationCreate(
                tournament_id.get_value(),
                user_id,
            ));
        }
    });
    view! {
        <div class="grid gap-3">
            <Show when=move || allowed.get()>
                <div class="space-y-2 sm:flex sm:gap-4 sm:items-start sm:space-y-0 ui-setting-group">
                    // TODO: i18n once copy is approved.
                    <p class="font-bold sm:pt-2 shrink-0">"Invite a player"</p>
                    <Show
                        when=move || !field_full.get()
                        fallback=|| {
                            view! {
                                // TODO: i18n once copy is approved.
                                <p class="text-sm ui-notice">
                                    "The field is full. No new invitations can be sent."
                                </p>
                            }
                        }
                    >
                        <InviteUser tournament />
                    </Show>
                </div>
            </Show>
            // TODO: i18n once copy is approved.
            <MembershipSection
                title="Accepted entrants"
                users=Signal::derive(move || {
                    tournament.common.memberships().get().players.into_values().collect()
                })
                actions=user_kick
            />
            // TODO: i18n once copy is approved.
            <MembershipSection
                title="Pending invitations"
                users=Signal::derive(move || tournament.common.memberships().get().invitees)
                actions=user_uninvite
            />
            // TODO: i18n once copy is approved.
            <MembershipSection
                title="Declined invitations"
                users=Signal::derive(move || {
                    tournament.common.memberships().get().declined_invitees
                })
                actions=Signal::derive(Vec::new)
                collapsed=true
                reinvite
                reinvite_disabled
            />
            // TODO: i18n once copy is approved.
            <p class="text-xs text-gray-600 dark:text-gray-300">
                "Only accepted entrants count toward the field."
            </p>
        </div>
    }
}
