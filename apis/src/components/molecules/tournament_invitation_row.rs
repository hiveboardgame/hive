use crate::common::TournamentAction;
use crate::components::molecules::time_row::TimeRow;
use crate::providers::ApiRequests;
use crate::responses::TournamentResponse;
use leptos::*;
use leptos_icons::*;
use shared_types::TimeInfo;

#[component]
pub fn TournamentInvitationRow(tournament: RwSignal<TournamentResponse>) -> impl IntoView {
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let seats_taken = format!("{}/{}", tournament().players.len(), tournament().seats);
    let seats_full = move || tournament().players.len() as i32 >= tournament().seats;

    let decline = move |_| {
        let api = ApiRequests::new();
        api.tournament(TournamentAction::InvitationDecline(
            tournament().tournament_id,
        ));
    };
    let accept = move |_| {
        let api = ApiRequests::new();
        api.tournament(TournamentAction::InvitationAccept(
            tournament().tournament_id,
        ));
    };
    let time_info = TimeInfo {
        mode: tournament().time_mode,
        base: tournament().time_base,
        increment: tournament().time_increment,
    };

    view! {
        <div class="flex items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <div class="flex relative">
                <div class=div_class>
                    <div>{tournament().name}</div>
                </div>
                <div class=div_class>
                    <TimeRow time_info/>
                </div>
                <div class=div_class>
                    <div>Players: {seats_taken}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 w-full h-full"
                    href=format!("/tournament/{}", tournament().tournament_id)
                ></a>
            </div>
            <div class=div_class>
                <button
                    title="Accept Invitation"
                    on:click=accept
                    prop:disabled=seats_full
                    class="z-20 p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                >
                    <Icon icon=icondata::AiCheckOutlined class="w-6 h-6"/>
                </button>
                <button
                    title="Decline Invitation"
                    on:click=decline
                    class="z-20 p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95"
                >
                    <Icon icon=icondata::IoCloseSharp class="w-6 h-6"/>
                </button>
            </div>
        </div>
    }
}
