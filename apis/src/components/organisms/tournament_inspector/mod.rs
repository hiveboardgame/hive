mod overview;
mod player;
mod selection;

pub(crate) use overview::TournamentOverviewInspector;
pub(crate) use player::player_standing;
pub use selection::TournamentSelection;

use self::{player::TournamentPlayerDetails, selection::return_focus_to_player};
use crate::{components::molecules::panel::Panel, providers::TournamentState};
use leptos::prelude::*;

#[component]
fn TournamentInspector(
    tournament: TournamentState,
    selection: RwSignal<Option<TournamentSelection>>,
    focus_scope_mounted: ArcRwSignal<bool>,
) -> impl IntoView {
    let close_selection = Callback::new(move |_: ()| {
        let selected = selection.get_untracked();
        selection.set(None);
        if let Some(TournamentSelection::Player(player)) = selected {
            return_focus_to_player(player, focus_scope_mounted.clone());
        }
    });

    view! {
        {move || match selection.get() {
            Some(TournamentSelection::Player(player)) => {
                view! {
                    <Panel class="min-w-0" body_class="p-0!">
                        <TournamentPlayerDetails
                            tournament
                            player
                            selection
                            close=close_selection
                        />
                    </Panel>
                }
                    .into_any()
            }
            None => ().into_any(),
        }}
    }
}
