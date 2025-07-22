use crate::common::TournamentAction;
use crate::components::molecules::time_row::TimeRow;
use crate::providers::ApiRequestsProvider;
use crate::responses::TournamentAbstractResponse;
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
        <div class="flex items-center justify-between text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light w-full p-2">
            <div class="flex relative flex-grow">
                <div class="xs:py-1 xs:px-1 sm:py-2 sm:px-2">
                    <div class="text-sm font-medium">Tournament Invitation</div>
                    <div class="font-bold">{tournament.name}</div>
                </div>
                <div class="xs:py-1 xs:px-1 sm:py-2 sm:px-2">
                    <TimeRow time_info />
                </div>
                <div class="xs:py-1 xs:px-1 sm:py-2 sm:px-2">
                    <div>Players: {seats_taken}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 w-full h-full"
                    href=format!("/tournament/{}", tournament_id.get_value())
                ></a>
            </div>
            <div class="flex gap-2">
                <button
                    title="Accept Invitation"
                    on:click=accept
                    prop:disabled=seats_full
                    class="z-20 p-1 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                >
                    <Icon icon=icondata_ai::AiCheckOutlined attr:class="w-6 h-6" />
                </button>
                <button
                    title="Decline Invitation"
                    on:click=decline
                    class="z-20 p-1 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="w-6 h-6" />
                </button>
            </div>
        </div>
    }
}
