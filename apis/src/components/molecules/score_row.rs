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
    /// The player's rating at the tournament's own speed, which is the only one
    /// that says anything about the field they are in. `None` when they have no
    /// rating at that speed yet.
    rating: Option<u64>,
    /// Left mid-event. Struck through rather than tagged: they keep their
    /// position and everything they scored, so the row is still theirs — it just
    /// stopped being added to.
    #[prop(optional)]
    withdrawn: bool,
) -> impl IntoView {
    let user = StoredValue::new(user);
    let profile_link = move || {
        view! {
            <ProfileLink
                patreon=user.with_value(|u| u.patreon)
                bot=user.with_value(|u| u.bot)
                username=user.with_value(|u| u.username.clone())
                deleted=user.with_value(|u| u.deleted)
                extend_tw_classes="truncate max-w-[120px]"
                user_is_hoverable=user.get_value().into()
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
        <tr class="h-6 ui-dense-table-row max-w-fit [&>td:nth-child(4)]:pl-2 sm:[&>td:nth-child(4)]:pl-3">
            <td class=td_class>
                <div class=div_class>{standing}</div>
            </td>
            <td class=td_class>
                <div
                    class=if withdrawn {
                        "flex items-center line-through decoration-2 opacity-60"
                    } else {
                        "flex items-center"
                    }
                    title=if withdrawn { "Withdrew before the tournament finished" } else { "" }
                >
                    <StatusIndicator
                        username=user.with_value(|u| u.username.clone())
                        deleted=user.with_value(|u| u.deleted)
                    />
                    {profile_link()}
                </div>
            </td>
            <td class=td_class>
                <div class=div_class>
                    <span class="tabular-nums text-gray-600 dark:text-gray-300">
                        {rating.map_or_else(|| String::from("—"), |rating| rating.to_string())}
                    </span>
                </div>
            </td>
            {scores_view}
            <td class=td_class>
                <div class=div_class>{finished}</div>
            </td>
        </tr>
    }
}
