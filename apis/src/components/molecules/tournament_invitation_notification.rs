use crate::{
    common::TournamentAction,
    components::molecules::time_row::TimeRow,
    providers::ApiRequestsProvider,
    responses::TournamentAbstractResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TimeInfo;

#[component]
pub fn TournamentInvitationNotification(tournament: TournamentAbstractResponse) -> impl IntoView {
    let tournament_id = StoredValue::new(tournament.tournament_id.clone());
    let api = expect_context::<ApiRequestsProvider>().0;
    let seats_taken = format!("{}/{}", tournament.players, tournament.seats);
    let seats_full = tournament.players as i32 >= tournament.seats;

    let decline = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationDecline(
            tournament_id.get_value(),
        ));
    };
    let accept = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationAccept(
            tournament_id.get_value(),
        ));
    };
    let time_info = TimeInfo {
        mode: tournament.time_mode,
        base: tournament.time_base,
        increment: tournament.time_increment,
    };

    view! {
        <div class="ui-notification-item">
            <div class="relative flex-1 min-w-0">
                <div class="ui-notification-label">Tournament Invitation</div>
                <div class="ui-notification-title">{tournament.name}</div>
                <div class="ui-notification-meta">
                    <div class="min-w-0">
                        <TimeRow time_info extend_tw_classes="text-xs leading-tight" />
                    </div>
                    <div class="whitespace-nowrap">Players: {seats_taken}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 size-full"
                    href=format!("/tournament/{}", tournament_id.get_value())
                ></a>
            </div>
            <div class="ui-notification-actions">
                <button
                    title="Accept Invitation"
                    on:click=accept
                    prop:disabled=seats_full
                    class="z-20 ui-button ui-button-primary ui-button-icon"
                >
                    <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />
                </button>
                <button
                    title="Decline Invitation"
                    on:click=decline
                    class="z-20 ui-button ui-button-danger ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                </button>
            </div>
        </div>
    }
}
