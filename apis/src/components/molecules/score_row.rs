use crate::{
    components::atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator},
    responses::UserResponse,
};
use leptos::prelude::*;
use shared_types::{PlayerScores, Tiebreaker};

#[component]
pub fn ScoreRow(
    user: UserResponse,
    standing: String,
    finished: i32,
    tiebreakers: Vec<Tiebreaker>,
    scores: PlayerScores,
) -> impl IntoView {
    let user = Signal::derive(move || user.clone());
    let profile_link = move || {
        view! {
            <ProfileLink
                patreon=user().patreon
                bot=user().patreon
                username=user().username
                extend_tw_classes="truncate max-w-[120px]"
                user_is_hoverable=user.into()
            />
        }
    };
    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let div_class = "flex justify-center items-center";
    let scores_view = tiebreakers
        .iter()
        .map(|tiebreaker| {
            view! {
                <td class=td_class>
                    <div class=div_class>{*scores.get(tiebreaker).unwrap_or(&0.0)}</div>
                </td>
            }
        })
        .collect_view();

    view! {
        <tr class="h-6 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <td class=td_class>
                <div class=div_class>{standing}</div>
            </td>
            <td class=td_class>
                <div class="flex items-center">
                    <StatusIndicator username=user().username />
                    {profile_link()}
                </div>
            </td>
            {scores_view}
            <td class=td_class>
                <div class=div_class>{finished}</div>
            </td>
        </tr>
    }
}
