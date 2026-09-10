use crate::{
    common::{tournament_path_matches, TournamentAction},
    i18n::*,
    providers::{
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatStore,
    },
};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use reactive_stores::Store;
use shared_types::{tournament::Format, TournamentStatus};
use uuid::Uuid;

#[cfg(feature = "hydrate")]
fn confirm_withdrawal(message: &str) -> bool {
    window().confirm_with_message(message).unwrap_or(false)
}

#[cfg(not(feature = "hydrate"))]
fn confirm_withdrawal(_message: &str) -> bool {
    false
}

#[component]
pub fn TournamentWithdrawal(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
    organizer: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let selected = RwSignal::new(Option::<Uuid>::None);
    let user_id = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let pathname = use_location().pathname;
    let expected_tournament_id = StoredValue::new(common.lifecycle().get_untracked().tournament_id);
    let withdrawable_entrants = Signal::derive(move || match format {
        TournamentFormatStore::Arena(_) => None,
        TournamentFormatStore::RoundRobin(state) => Some(state.withdrawable_entrants().get()),
        TournamentFormatStore::Swiss(state) => Some(state.withdrawable_entrants().get()),
        TournamentFormatStore::Elimination(state) => Some(state.withdrawable_entrants().get()),
    });
    let route_active = Signal::derive(move || {
        let pathname = pathname.get();
        let expected_tournament_id = expected_tournament_id.get_value();
        tournament_path_matches(&pathname, &expected_tournament_id)
            && common.lifecycle().get().tournament_id == expected_tournament_id
    });
    let send = Callback::new(move |player| {
        if route_active.try_get() != Some(true) {
            return;
        }
        let Some(expected_tournament_id) = expected_tournament_id.try_get_value() else {
            return;
        };
        let Some(tournament_id) = common.lifecycle().try_get().and_then(|lifecycle| {
            (lifecycle.tournament_id == expected_tournament_id).then_some(lifecycle.tournament_id)
        }) else {
            return;
        };
        api.get()
            .tournament(TournamentAction::Withdraw(tournament_id, player));
    });

    view! {
        {move || {
            if !route_active.try_get()? {
                return None;
            }
            let current_tournament_id = expected_tournament_id.try_get_value()?;
            let current_user = user_id.try_get()?;
            let selected_player = selected.try_get()?;
            let lifecycle = common.lifecycle().try_get()?;
            if lifecycle.tournament_id != current_tournament_id {
                return None;
            }
            let memberships = common.memberships().get();
            let capabilities = withdrawable_entrants.try_get()??;
            let in_progress = matches!(lifecycle.status, TournamentStatus::InProgress)
                && format.format() != Format::Arena;
            let withdrawable = current_user.is_some_and(|id| capabilities.contains(&id));
            let mut candidates = memberships
                .players
                .values()
                .filter(|user| capabilities.contains(&user.uid))
                .map(|user| (user.uid, user.username.clone()))
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)));
            let self_action = (in_progress && withdrawable)
                .then(|| {
                    view! {
                        <button
                            class="ui-button ui-button-danger ui-button-sm"
                            on:click=move |_| {
                                if route_active.try_get() == Some(true)
                                    && confirm_withdrawal(
                                        t_string!(
                                            i18n, tournaments.view.withdrawal.self_confirmation
                                        ),
                                    )
                                {
                                    if let Some(id) = user_id.try_get_untracked().flatten() {
                                        let _ = send.try_run(id);
                                    }
                                }
                            }
                        >
                            {t!(i18n, tournaments.view.withdrawal.self_action)}
                        </button>
                    }
                });
            let organizer_action = (in_progress && organizer.try_get().unwrap_or(false))
                .then(|| {
                    view! {
                        <div class="flex flex-wrap gap-2 items-center">
                            <select
                                class="ui-field-select"
                                on:change=move |event| {
                                    if route_active.try_get() == Some(true) {
                                        selected.set(event_target_value(&event).parse().ok());
                                    }
                                }
                            >
                                <option value="">
                                    {t!(i18n, tournaments.view.withdrawal.select_entrant)}
                                </option>
                                {candidates
                                    .into_iter()
                                    .map(|candidate| {
                                        view! {
                                            <option value=candidate.0.to_string()>{candidate.1}</option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                            <button
                                class="ui-button ui-button-danger ui-button-sm"
                                prop:disabled=selected_player.is_none()
                                on:click=move |_| {
                                    if route_active.try_get() != Some(true) {
                                        return;
                                    }
                                    let Some(id) = selected.try_get_untracked().flatten() else {
                                        return;
                                    };
                                    let Some(username) = common
                                        .memberships()
                                        .with_untracked(|memberships| {
                                            memberships
                                                .players
                                                .get(&id)
                                                .map(|user| user.username.clone())
                                        }) else {
                                        return;
                                    };
                                    let message = format!(
                                        "{username}\n{}",
                                        t_string!(
                                            i18n, tournaments.view.withdrawal.entrant_confirmation
                                        ),
                                    );
                                    if confirm_withdrawal(&message) {
                                        let _ = send.try_run(id);
                                        selected.set(None);
                                    }
                                }
                            >
                                {t!(i18n, tournaments.view.withdrawal.entrant_action)}
                            </button>
                        </div>
                    }
                });
            Some(

                view! {
                    {self_action}
                    {organizer_action}
                },
            )
        }}
    }
}
