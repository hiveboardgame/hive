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
    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";

    view! {
        <tr class="h-6 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <td class=td_class>
                <div class="flex justify-center items-center">{standing}</div>
            </td>
            <td class=td_class>
                <div class="flex items-center">
                    <StatusIndicator username=user().username />
                    {profile_link()}
                </div>
            </td>
            {tiebreakers
                .iter()
                .map(|tiebreaker| {
                    view! {
                        <td class=td_class>
                            <div class="flex justify-center items-center">
                                {*scores.get(tiebreaker).unwrap_or(&0.0)}
                            </div>
                        </td>
                    }
                })
                .collect_view()}
        </tr>
    }
}
