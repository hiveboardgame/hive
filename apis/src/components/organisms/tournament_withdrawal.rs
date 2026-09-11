use crate::{
    common::{tournament_path_matches, TournamentAction},
    components::molecules::modal::Modal,
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
use leptos::{html::Dialog, prelude::*};
use leptos_router::hooks::use_location;
use reactive_stores::Store;
use shared_types::TournamentStatus;
use uuid::Uuid;

#[component]
pub fn TournamentWithdrawal(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
    organizer: Signal<bool>,
    #[prop(optional)] player_id: Option<Uuid>,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
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

    let target = Signal::derive(move || player_id.or(user_id.get()));
    let allowed = Signal::derive(move || {
        route_active.get()
            && common.lifecycle().get().status == TournamentStatus::InProgress
            && user_id.get().is_some()
            && (player_id.is_none() || organizer.get())
            && target.get().is_some_and(|id| {
                common.memberships().get().players.contains_key(&id)
                    && withdrawable_entrants
                        .get()
                        .is_some_and(|ids| ids.contains(&id))
            })
    });
    let dialog_el = NodeRef::<Dialog>::new();
    let reviewed = RwSignal::new(None::<(Uuid, String)>);
    let cancel = Callback::new(move |_: ()| {
        if let Some(dialog) = dialog_el.try_get().flatten() {
            dialog.close();
        }
        let _ = reviewed.try_set(None);
    });
    Effect::new(move |_| {
        if !allowed.get()
            || reviewed.with(|reviewed| {
                reviewed
                    .as_ref()
                    .is_some_and(|(id, _)| Some(*id) != target.get())
            })
        {
            cancel.run(());
        }
    });
    let confirm = move |_| {
        if allowed.try_get_untracked() != Some(true) {
            return;
        }
        let Some((id, _)) = reviewed.try_get_untracked().flatten() else {
            return;
        };
        if target.try_get_untracked().flatten() == Some(id) {
            let _ = send.try_run(id);
        }
        cancel.run(());
    };
    view! {
        <Show when=move || allowed.get()>
            <button
                type="button"
                class="shrink-0 ui-button ui-button-secondary ui-button-sm"
                on:click=move |_| {
                    if allowed.try_get_untracked() != Some(true) {
                        return;
                    }
                    let Some(id) = target.try_get_untracked().flatten() else {
                        return;
                    };
                    let Some(username) = common
                        .memberships()
                        .with_untracked(|memberships| {
                            memberships.players.get(&id).map(|user| user.username.clone())
                        }) else {
                        return;
                    };
                    reviewed.set(Some((id, username)));
                    if let Some(dialog) = dialog_el.try_get().flatten() {
                        let _ = dialog.show_modal();
                    }
                }
            >
                // TODO: i18n once copy is approved.
                "Withdraw"
            </button>
        </Show>
        // TODO: i18n once copy is approved.
        <Modal dialog_el aria_label="Confirm withdrawal" on_close=cancel>
            <Show when=move || reviewed.with(Option::is_some)>
                <div class="px-4 pb-4 space-y-4 w-[min(90vw,26rem)]">
                    // TODO: i18n once copy is approved.
                    <h2 class="text-xl font-bold">
                        {move || {
                            if player_id.is_some() {
                                reviewed
                                    .with(|reviewed| {
                                        reviewed
                                            .as_ref()
                                            .map(|(_, name)| format!("Withdraw {name}?"))
                                    })
                            } else {
                                Some(String::from("Withdraw from tournament?"))
                            }
                        }}
                    </h2>
                    <p class="text-sm text-gray-700 dark:text-gray-200">
                        {if player_id.is_some() {
                            t_string!(i18n, tournaments.view.withdrawal.entrant_confirmation)
                                .to_string()
                        } else {
                            t_string!(i18n, tournaments.view.withdrawal.self_confirmation)
                                .to_string()
                        }}
                    </p>
                    <div class="flex gap-2 justify-end">
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| cancel.run(())
                        >
                            // TODO: i18n once copy is approved.
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-danger ui-button-sm"
                            prop:disabled=move || !allowed.get()
                            on:click=confirm
                        >
                            // TODO: i18n once copy is approved.
                            "Withdraw"
                        </button>
                    </div>
                </div>
            </Show>
        </Modal>
    }
}
