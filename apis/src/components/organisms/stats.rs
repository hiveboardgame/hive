use crate::{
    components::{
        atoms::{
            rating::{icon_for_speed, Rating},
            rating_history::RatingGraph,
        },
        molecules::modal::Modal,
    },
    responses::{RatingResponse, UserResponse},
};
use leptos::{html::Dialog, prelude::*};
use leptos_icons::*;
use shared_types::GameSpeed;

#[component]
pub fn Stats(user: UserResponse) -> impl IntoView {
    let user_ratings = StoredValue::new(
        GameSpeed::all_rated_games()
            .iter()
            .filter_map(|speed| {
                user.ratings
                    .get(speed)
                    .filter(|rating| rating.played > 0)
                    .cloned()
            })
            .collect::<Vec<_>>(),
    );

    let selected_rating = RwSignal::new(None::<RatingResponse>);
    let dialog_el = NodeRef::<Dialog>::new();

    view! {
        <div class="flex flex-wrap gap-2 justify-center p-3 mt-1">
            {user_ratings
                .get_value()
                .into_iter()
                .map(|rating| {
                    view! {
                        <button
                            class="flex flex-col flex-shrink-0 gap-1 items-center p-2 bg-gray-50 rounded-lg border border-gray-200 shadow-sm transition-all duration-200 cursor-pointer md:flex-row dark:bg-gray-800 dark:border-gray-600 hover:bg-blue-50 hover:border-blue-300 hover:shadow-md w-fit dark:hover:bg-gray-700 dark:hover:border-gray-500 hover:scale-[1.02]"
                            on:click={
                                let rating = rating.clone();
                                move |_| {
                                    selected_rating.set(Some(rating.clone()));
                                    if let Some(dialog) = dialog_el.get() {
                                        let _ = dialog.show_modal();
                                    }
                                }
                            }
                        >
                            <div class="flex flex-row gap-1 items-center">
                                <Icon
                                    icon=icon_for_speed(rating.speed)
                                    attr:class="flex-shrink-0 size-2 sm:size-3 md:size-4 lg:size-5"
                                />

                                <div class="text-xs font-bold sm:text-sm lg:text-lg text-pillbug-teal">
                                    <Rating rating=rating.clone() />
                                </div>
                            </div>

                            <div class="hidden text-xs text-gray-500 md:block lg:hidden dark:text-gray-400">
                                {rating.played} " games"
                            </div>

                            <div class="hidden lg:flex lg:flex-col lg:items-start lg:min-w-0">
                                <div class="max-w-full text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                                    {rating.speed.to_string()}
                                </div>
                                <div class="text-xs text-gray-500 dark:text-gray-400">
                                    {rating.played} " games"
                                </div>
                            </div>
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>

        <Modal dialog_el=dialog_el>
            {move || {
                selected_rating
                    .get()
                    .map(|rating| {
                        view! {
                            <div class="p-4 mx-auto w-full max-w-md">
                                <div class="flex gap-2 items-center mb-4">
                                    <Icon icon=icon_for_speed(rating.speed) attr:class="size-6" />
                                    <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                                        {rating.speed.to_string()} " Statistics"
                                    </h3>
                                </div>
                                <div class="space-y-3">
                                    <div class="grid grid-cols-2 gap-3">
                                        <div class="p-3 text-center bg-gray-50 rounded-lg dark:bg-gray-800">
                                            <div class="text-sm text-gray-500 dark:text-gray-400">
                                                Rating
                                            </div>
                                            <div class="text-xl font-bold text-pillbug-teal">
                                                <Rating rating=rating.clone() />
                                            </div>
                                        </div>
                                        <div class="p-3 text-center bg-blue-50 rounded-lg dark:bg-blue-900/20">
                                            <div class="text-sm text-blue-600 dark:text-blue-400">
                                                Win Rate
                                            </div>
                                            <div class="text-xl font-bold text-blue-600 dark:text-blue-400">
                                                {if rating.played > 0 {
                                                    format!(
                                                        "{:.1}%",
                                                        (rating.win as f64 / rating.played as f64) * 100.0,
                                                    )
                                                } else {
                                                    "0.0%".to_string()
                                                }}
                                            </div>
                                        </div>
                                    </div>
                                    <div class="grid grid-cols-4 gap-2">
                                        <div class="p-2 min-w-0 text-center bg-gray-50 rounded-lg dark:bg-gray-800">
                                            <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                                                Total
                                            </div>
                                            <div class="text-sm font-semibold text-gray-900 dark:text-gray-100">
                                                {rating.played}
                                            </div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center bg-green-50 rounded-lg dark:bg-green-900/20">
                                            <div class="text-xs text-green-600 dark:text-green-400 truncate">
                                                Wins
                                            </div>
                                            <div class="text-sm font-semibold text-green-600 dark:text-green-400">
                                                {rating.win}
                                            </div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center bg-red-50 rounded-lg dark:bg-red-900/20">
                                            <div class="text-xs text-red-600 dark:text-red-400 truncate">
                                                Losses
                                            </div>
                                            <div class="text-sm font-semibold text-red-600 dark:text-red-400">
                                                {rating.loss}
                                            </div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center bg-yellow-50 rounded-lg dark:bg-yellow-900/20">
                                            <div class="text-xs text-yellow-600 dark:text-yellow-400 truncate">
                                                Draws
                                            </div>
                                            <div class="text-sm font-semibold text-yellow-600 dark:text-yellow-400">
                                                {rating.draw}
                                            </div>
                                        </div>
                                    </div>

                                </div>
                            </div>
                            <RatingGraph user_id=rating.user_uid game_speed=rating.speed />
                        }
                    })
            }}
        </Modal>
    }
}
