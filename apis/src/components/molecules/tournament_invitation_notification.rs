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
        <div class="flex justify-between items-center p-2 w-full text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex relative flex-grow">
                <div class="sm:py-2 sm:px-2 xs:py-1 xs:px-1">
                    <div class="text-sm font-medium">Tournament Invitation</div>
                    <div class="font-bold">{tournament.name}</div>
                </div>
                <div class="sm:py-2 sm:px-2 xs:py-1 xs:px-1">
                    <TimeRow time_info />
                </div>
                <div class="sm:py-2 sm:px-2 xs:py-1 xs:px-1">
                    <div>Players: {seats_taken}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 size-full"
                    href=format!("/tournament/{}", tournament_id.get_value())
                ></a>
            </div>
            <div class="flex gap-2">
                <button
                    title="Accept Invitation"
                    on:click=accept
                    prop:disabled=seats_full
                    class="z-20 p-1 text-white rounded transition-transform duration-300 active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal disabled:hover:bg-transparent"
                >
                    <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />
                </button>
                <button
                    title="Decline Invitation"
                    on:click=decline
                    class="z-20 p-1 text-white rounded transition-transform duration-300 hover:bg-red-400 active:scale-95 bg-ladybug-red"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                </button>
            </div>
        </div>
    }
}
