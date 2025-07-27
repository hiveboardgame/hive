use crate::common::TournamentAction;
use crate::components::atoms::select_options::SelectOption;
use crate::components::update_from_event::update_from_input_parsed;
use crate::providers::ApiRequestsProvider;
use crate::responses::TournamentResponse;
use leptos::prelude::*;
use shared_types::{PrettyString, ScoringMode};

#[component]
pub fn TournamentSettings(
    user_is_organizer: bool,
    tournament: StoredValue<TournamentResponse>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let current_scoring = move || tournament.with_value(|t| t.scoring.clone());
    let new_scoring = RwSignal::new(current_scoring());

    // Update the signal when tournament data changes
    Effect::new(move |_| {
        new_scoring.set(current_scoring());
    });

    let update_scoring = move |_| {
        if user_is_organizer && new_scoring.get() != current_scoring() {
            let tournament_id = tournament.with_value(|t| t.tournament_id.clone());
            let api = api.get();
            let action = TournamentAction::UpdateScoringMode(tournament_id, new_scoring.get());
            api.tournament(action);
        }
    };

    let scoring_changed = move || new_scoring.get() != current_scoring();

    view! {
        <div class="flex flex-col items-center p-4 border rounded-lg bg-light-light dark:bg-dark-light">
            <h3 class="mb-4 text-lg font-bold">"Tournament Settings"</h3>
            
            <div class="flex flex-col gap-4 w-full max-w-sm">
                <div class="flex flex-col">
                    <label class="mb-2 text-sm font-medium">"Scoring Mode:"</label>
                    <Show 
                        when=move || user_is_organizer
                        fallback=move || view! {
                            <div class="px-3 py-2 border rounded bg-gray-100 dark:bg-gray-800">
                                {current_scoring().pretty_string()}
                            </div>
                        }
                    >
                        <select
                            class="px-3 py-2 border rounded bg-odd-light dark:bg-gray-700"
                            name="Scoring Mode"
                            on:change=update_from_input_parsed(new_scoring)
                        >
                            <SelectOption
                                value=new_scoring
                                is="Game"
                                text=ScoringMode::Game.pretty_string()
                            />
                            <SelectOption
                                value=new_scoring
                                is="Match"
                                text=ScoringMode::Match.pretty_string()
                            />
                        </select>
                    </Show>
                </div>

                <Show when=move || user_is_organizer && scoring_changed()>
                    <button
                        class="px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                        on:click=update_scoring
                    >
                        "Update Scoring Mode"
                    </button>
                </Show>

                <Show when=move || user_is_organizer>
                    <div class="text-xs text-gray-600 dark:text-gray-400">
                        <p><strong>"Game Scoring:"</strong>" Each individual game counts (Win=1pt, Draw=0.5pt)"</p>
                        <p><strong>"Match Scoring:"</strong>" Players' games are grouped into matches. Winner of each match gets 1pt, tied matches give 0.5pt each"</p>
                    </div>
                </Show>
            </div>
        </div>
    }
} 