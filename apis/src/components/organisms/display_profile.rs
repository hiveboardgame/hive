use crate::components::atoms::rating::{icon_for_speed, Rating};
use leptos_i18n::*;
use crate::responses::UserResponse;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameSpeed;

#[component]
pub fn DisplayProfile(user: UserResponse) -> impl IntoView {
    let i18n = use_i18n();
    let user_ratings = GameSpeed::all_rated_games()
        .iter()
        .filter_map(|speed| user.ratings.get(speed).cloned())
        .collect::<Vec<_>>();
    view! {
        <table class="w-full">
            <tr>
                <th class="w-4"></th>
                <th>Elo</th>
                <th>{t!(i18n, profile.stats_box.total)}</th>
                <th>{t!(i18n, profile.stats_box.wins)}</th>
                <th>{t!(i18n, profile.stats_box.losses)}</th>
                <th>{t!(i18n, profile.stats_box.draws)}</th>
            </tr>
            <For each=move || user_ratings.clone() key=|rating| rating.speed.clone() let:rating>
                <tr>
                    <td class="text-center">
                        <Icon icon=icon_for_speed(&rating.speed) />
                    </td>
                    <td class="text-center">
                        <Rating rating=rating.clone() />
                    </td>
                    <td class="text-center">{rating.played}</td>
                    <td class="text-center">{rating.win}</td>
                    <td class="text-center">{rating.loss}</td>
                    <td class="text-center">{rating.draw}</td>
                </tr>
            </For>
        </table>
    }
}
