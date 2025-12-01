use crate::components::atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator};
use crate::components::organisms::{games_filter::GamesFilter, stats::Stats};
use crate::functions::users::get_profile;
use crate::i18n::*;
use crate::providers::{calculate_initial_batch_size, provide_games_search_context};
use leptos::either::Either;
use leptos::{html, prelude::*};
use leptos_router::hooks::use_params;
use leptos_router::{hooks::use_location, params::Params};
use leptos_use::use_element_bounding;
use shared_types::GameProgress;

pub fn tab_from_path(path: &str) -> GameProgress {
    if path.ends_with("/unstarted") {
        GameProgress::Unstarted
    } else if path.ends_with("/playing") {
        GameProgress::Playing
    } else if path.ends_with("/finished") {
        GameProgress::Finished
    } else {
        GameProgress::Playing
    }
}

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let username =
        move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let user = LocalResource::new(move || get_profile(username()));

    let games_container_ref = NodeRef::<html::Div>::new();
    let bounding = use_element_bounding(games_container_ref);

    let initial_batch_size = Signal::derive(move || {
        calculate_initial_batch_size(bounding.height.get(), bounding.width.get())
    });

    let infinite_scroll_batch_size = Signal::derive(move || {
        let container_width = bounding.width.get();
        if container_width < 640.0 {
            3 // mobile (1 column)
        } else if container_width < 1024.0 {
            4 // sm to lg (2 columns)
        } else {
            6 // lg and above (3 columns)
        }
    });

    let ctx = provide_games_search_context(
        initial_batch_size,
        infinite_scroll_batch_size,
        games_container_ref,
    );

    let location = use_location();
    let current_tab = Signal::derive(move || tab_from_path(&location.pathname.get()));
    let i18n = use_i18n();
    let radio_classes = |active| {
        format!("no-link-style py-1 px-2 text-sm font-semibold rounded-lg border-2 transition-all duration-200 transform hover:scale-[1.02] cursor-pointer shadow-sm hover:shadow-md {}", 
            if active {
                "bg-pillbug-teal border-pillbug-teal text-white hover:bg-pillbug-teal/90" 
            } else {
                "bg-gray-50 border-gray-200 text-gray-700 hover:bg-gray-100 hover:border-gray-300 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:border-gray-500" 
            })
    };

    Effect::watch(
        ctx.next_batch.version(),
        move |_, _, _| {
            let next_batch = if let Some(Ok(next_batch)) = ctx.next_batch.value().get_untracked() {
                next_batch
            } else {
                vec![]
            };
            if next_batch.is_empty() {
                ctx.has_more.set_value(false);
            }
            ctx.games.update(|games| {
                if ctx.is_first_batch.get_value() {
                    *games = next_batch;
                } else {
                    games.extend(next_batch);
                }
            });
        },
        true,
    );

    view! {
        <div class="flex flex-col pt-12 px-3 bg-light dark:bg-gray-950 h-[100vh] overflow-hidden">
            <Transition fallback=move || {
                view! { <p>"Loading Profile..."</p> }
            }>
                {move || {
                    user.get()
                        .map(|user| {
                            if let Ok(user) = user {
                                let username = StoredValue::new(user.username.clone());
                                Either::Left(
                                    view! {
                                        <div class="flex-shrink-0">
                                            <div class="flex flex-row flex-wrap justify-center mx-1 w-full text-lg sm:text-xl">
                                                <div class="flex items-center">
                                                    <StatusIndicator username=username.get_value() />
                                                    <ProfileLink
                                                        patreon=user.patreon
                                                        bot=user.bot
                                                        username=username.get_value()
                                                        extend_tw_classes="truncate max-w-[125px]"
                                                    />
                                                </div>
                                            </div>

                                            <div class="lg:flex lg:flex-col lg:items-center lg:max-w-4xl lg:mx-auto">
                                                <Stats user />

                                                <div class="grid grid-cols-[1fr_auto] gap-1 items-start m-1 lg:flex lg:items-center lg:gap-4 lg:mt-4">
                                                    <div class="flex flex-wrap gap-1 min-w-0">
                                                        <a
                                                            href=format!("/@/{}/unstarted", username.get_value())
                                                            class=move || radio_classes(
                                                                current_tab() == GameProgress::Unstarted,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.pending)}
                                                        </a>
                                                        <a
                                                            href=format!("/@/{}/playing", username.get_value())
                                                            class=move || radio_classes(
                                                                current_tab() == GameProgress::Playing,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.playing)}
                                                        </a>
                                                        <a
                                                            href=format!("/@/{}/finished", username.get_value())
                                                            class=move || radio_classes(
                                                                current_tab() == GameProgress::Finished,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.finished)}
                                                        </a>
                                                    </div>

                                                    <GamesFilter
                                                        username=username.get_value()
                                                        ctx=ctx.clone()
                                                    />
                                                </div>
                                            </div>
                                        </div>

                                        <div class="flex flex-col flex-1 gap-1 m-1 min-h-0">
                                            {children()}
                                        </div>
                                    },
                                )
                            } else {
                                Either::Right(view! { <p>"User not found"</p> })
                            }
                        })
                }}
            </Transition>
        </div>
    }
}
