use crate::i18n::*;
use crate::{
    common::UserAction,
    components::{atoms::rating::RatingWithIcon, molecules::user_row::UserRow},
    responses::UserResponse,
};
use leptos::*;
use shared_types::GameSpeed;

#[component]
pub fn DisplayProfile(user: UserResponse) -> impl IntoView {
    let i18n = use_i18n();
    let ratings = GameSpeed::all_rated_games()
        .iter()
        .map(|speed| {
            if let Some(rating) = user.ratings.get(speed) {
                view! {
                    <div class="p-2 border border-dark dark:border-white">
                        <RatingWithIcon rating=store_value(rating.clone())/>
                        <div>{t!(i18n, profile.stats_box.total, count = rating.played)}</div>
                        <div>{t!(i18n, profile.stats_box.wins, count = rating.win)}</div>
                        <div>{t!(i18n, profile.stats_box.losses, count = rating.loss)}</div>
                        <div>{t!(i18n, profile.stats_box.draws, count = rating.draw)}</div>
                    </div>
                }
                .into_view()
            } else {
                "".into_view()
            }
        })
        .collect_view();

    view! {
        <div class="m-1">
            <div class="flex flex-col items-start">
                <div class="max-w-fit">
                    <UserRow
                        actions=vec![UserAction::Challenge]
                        user=store_value(user.clone())
                        on_profile=true
                    />
                </div>
                <div class="flex flex-wrap gap-1">{ratings}</div>
            </div>

        </div>
    }
}
