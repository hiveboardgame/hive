use crate::{
    components::atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator},
    responses::UserResponse,
};
use leptos::*;
use shared_types::{PlayerScores, Tiebreaker};

#[component]
pub fn ScoreRow(
    user: StoredValue<UserResponse>,
    standing: String,
    tiebreakers: Vec<Tiebreaker>,
    scores: PlayerScores,
) -> impl IntoView {
    let profile_link = move || {
        view! {
            <ProfileLink
                patreon=user().patreon
                username=user().username
                extend_tw_classes="truncate max-w-[120px]"
                user_is_hoverable=user
            />
        }
    };

    view! {
        <div class="flex justify-between items-center p-1 w-64 h-10 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex justify-between mr-2 w-full">
                <div class="flex items-center w-6">{standing}</div>

                <div class="flex items-center">
                    <StatusIndicator username=user().username/>
                    {profile_link()}
                </div>

                {tiebreakers
                    .iter()
                    .map(|tiebreaker| {
                        view! {
                            <div class="flex items-center">
                                {*scores.get(tiebreaker).unwrap_or(&0.0)}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
