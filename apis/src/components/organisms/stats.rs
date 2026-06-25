use crate::{
    common::with_class,
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

const METRIC_TILE_CLASS: &str =
    "rounded-lg border border-black/5 bg-odd-light/70 text-center dark:border-white/10 dark:bg-surface-muted";
const METRIC_MUTED_TEXT_CLASS: &str = "text-gray-500 dark:text-gray-400";
const ACCENT_METRIC_TEXT_CLASS: &str = "text-pillbug-teal";
const SUCCESS_METRIC_TEXT_CLASS: &str = "text-green-600 dark:text-green-400";
const DANGER_METRIC_TEXT_CLASS: &str = "text-red-600 dark:text-red-400";
const WARNING_METRIC_TEXT_CLASS: &str = "text-yellow-600 dark:text-yellow-400";

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
                            class=with_class(
                                "ui-card-row",
                                "flex w-fit flex-shrink-0 cursor-pointer flex-col items-center gap-1 p-2 md:flex-row",
                            )
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
                                        <div class=with_class(METRIC_TILE_CLASS, "p-3")>
                                            <div class=with_class(
                                                METRIC_MUTED_TEXT_CLASS,
                                                "text-sm",
                                            )>Rating</div>
                                            <div class=with_class(
                                                ACCENT_METRIC_TEXT_CLASS,
                                                "text-xl font-bold",
                                            )>
                                                <Rating rating=rating.clone() />
                                            </div>
                                        </div>
                                        <div class="p-3 text-center rounded-lg border border-pillbug-teal/20 bg-pillbug-teal/10 dark:border-pillbug-teal/30 dark:bg-pillbug-teal/10">
                                            <div class=with_class(
                                                ACCENT_METRIC_TEXT_CLASS,
                                                "text-sm",
                                            )>Win Rate</div>
                                            <div class=with_class(
                                                ACCENT_METRIC_TEXT_CLASS,
                                                "text-xl font-bold",
                                            )>
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
                                        <div class=with_class(METRIC_TILE_CLASS, "min-w-0 p-2")>
                                            <div class=with_class(
                                                METRIC_MUTED_TEXT_CLASS,
                                                "truncate text-xs",
                                            )>Total</div>
                                            <div class="text-sm font-semibold text-gray-900 dark:text-gray-100">
                                                {rating.played}
                                            </div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center bg-green-50 rounded-lg border border-green-500/20 dark:border-green-400/20 dark:bg-green-900/20">
                                            <div class=with_class(
                                                SUCCESS_METRIC_TEXT_CLASS,
                                                "truncate text-xs",
                                            )>Wins</div>
                                            <div class=with_class(
                                                SUCCESS_METRIC_TEXT_CLASS,
                                                "text-sm font-semibold",
                                            )>{rating.win}</div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center rounded-lg border border-ladybug-red/20 bg-ladybug-red/10 dark:border-ladybug-red/30 dark:bg-ladybug-red/10">
                                            <div class=with_class(
                                                DANGER_METRIC_TEXT_CLASS,
                                                "truncate text-xs",
                                            )>Losses</div>
                                            <div class=with_class(
                                                DANGER_METRIC_TEXT_CLASS,
                                                "text-sm font-semibold",
                                            )>{rating.loss}</div>
                                        </div>
                                        <div class="p-2 min-w-0 text-center bg-yellow-50 rounded-lg border border-orange-twilight/25 dark:border-orange-twilight/35 dark:bg-orange-twilight/10">
                                            <div class=with_class(
                                                WARNING_METRIC_TEXT_CLASS,
                                                "truncate text-xs",
                                            )>Draws</div>
                                            <div class=with_class(
                                                WARNING_METRIC_TEXT_CLASS,
                                                "text-sm font-semibold",
                                            )>{rating.draw}</div>
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
