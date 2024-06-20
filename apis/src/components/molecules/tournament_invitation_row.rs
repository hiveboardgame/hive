use crate::components::atoms::status_indicator::StatusIndicator;
use crate::components::molecules::time_row::TimeRow;
use crate::providers::ApiRequests;
use crate::{
    components::atoms::game_type::GameType,
    components::atoms::profile_link::ProfileLink,
    functions::hostname::hostname_and_port,
    providers::{game_state::GameStateSignal, AuthContext, ColorScheme},
    responses::TournamentResponse,
};
use leptos::*;
use leptos_icons::*;
use leptos_router::*;
use leptos_use::use_window;
use shared_types::ChallengeVisibility;

#[component]
pub fn TournamentInvitationRow(tournament: StoredValue<TournamentResponse>) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    let tournament_address = move || {
        format!(
            "{}/tournaments/{}",
            hostname_and_port(),
            tournament().tournament_id
        )
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let uid = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    };

    view! {
        <div> Tournament row</div>
        <tr class="items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <td class=td_class>
                <div>Tournament</div>
            </td>
        </tr>
    }
}
