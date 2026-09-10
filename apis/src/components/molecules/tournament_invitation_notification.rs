use crate::{
    common::{tournament_admission_viewer, TournamentAction},
    components::molecules::time_row::TimeRow,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext},
    responses::{
        AdmissionRestrictions,
        TournamentAbstractResponse,
        TournamentAdmission,
        ViewerRelationship,
    },
};
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn TournamentInvitationNotification(tournament: TournamentAbstractResponse) -> impl IntoView {
    let i18n = use_i18n();
    let tournament_id = StoredValue::new(tournament.tournament_id.clone());
    let api = expect_context::<ApiRequestsProvider>().0;
    let seats_taken = tournament.seats.map_or_else(
        || tournament.players.to_string(),
        |seats| format!("{}/{seats}", tournament.players),
    );
    let auth = expect_context::<AuthContext>();
    let now = use_ticking_now();
    let admission_clock = tournament.admission_clock();
    let admission_input = TournamentAdmission {
        entry_open: true,
        full: tournament
            .seats
            .is_some_and(|seats| tournament.players as i32 >= seats),
        restrictions: AdmissionRestrictions {
            invite_only: tournament.invite_only,
            band_lower: tournament.band_lower,
            band_upper: tournament.band_upper,
        },
        relationship: ViewerRelationship {
            joined: tournament.joined,
            invited: tournament.invited,
            organizing: tournament.organizing,
        },
        clock: admission_clock,
        bot_admission: tournament.configuration.bot_admission,
    };
    let admission = Signal::derive(move || {
        let mut input = admission_input;
        input.entry_open = tournament.started_at.is_none()
            && tournament.finished_at.is_none()
            && tournament
                .starts_at
                .is_none_or(|starts_at| now.get() < starts_at);
        input.decision(tournament_admission_viewer(&auth, admission_clock))
    });

    let decline = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationDecline(
            tournament_id.get_value(),
        ));
    };
    let accept = move |_| {
        if !admission.get_untracked().can_enter() {
            return;
        }
        let api = api.get();
        api.tournament(TournamentAction::InvitationAccept(
            tournament_id.get_value(),
        ));
    };
    let clock = tournament.summary_clock();

    view! {
        <div class="ui-notification-item">
            <div class="relative flex-1 min-w-0">
                <div class="ui-notification-label">
                    {t!(i18n, notifications.tournament_invitation.label)}
                </div>
                <div class="ui-notification-title">{tournament.name}</div>
                <div class="ui-notification-meta">
                    <div class="min-w-0">
                        {clock
                            .map(|clock| {
                                view! {
                                    <TimeRow
                                        time_control=Some(clock)
                                        extend_tw_classes="text-xs leading-tight"
                                    />
                                }
                            })}
                    </div>
                    <div class="whitespace-nowrap">
                        {t!(
                            i18n,
                            notifications.tournament_invitation.players,
                            count = seats_taken.clone(),
                        )}
                    </div>
                </div>
                <Show when=move || !admission.get().can_enter()>
                    <p class="mt-1 text-xs text-gray-600 dark:text-gray-300">
                        {move || admission.get().label()}
                    </p>
                </Show>
                <a
                    class="absolute top-0 left-0 z-10 size-full"
                    href=format!("/tournament/{}", tournament_id.get_value())
                ></a>
            </div>
            <div class="ui-notification-actions">
                <Show
                    when=move || admission.get().can_enter()
                    fallback=move || {
                        view! {
                            <a
                                class="relative z-20 text-xs font-medium"
                                href=format!("/tournament/{}", tournament_id.get_value())
                                title=move || admission.get().label()
                            >
                                // TODO: i18n once copy is approved.
                                "View invitation"
                            </a>
                        }
                    }
                >
                    <button
                        title=move || {
                            t_string!(i18n, notifications.tournament_invitation.accept).to_string()
                        }
                        on:click=accept
                        class="z-20 ui-button ui-button-primary ui-button-icon"
                    >
                        <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />
                    </button>
                </Show>
                <button
                    title=move || {
                        t_string!(i18n, notifications.tournament_invitation.decline).to_string()
                    }
                    on:click=decline
                    class="z-20 ui-button ui-button-danger ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                </button>
            </div>
        </div>
    }
}
