use crate::{
    common::with_class,
    components::{
        atoms::{block_toggle_button::BlockToggleButton, message_button::MessageButton},
        molecules::{empty_state::EmptyState, user_identity::UserIdentity},
        organisms::{games_filter::GamesFilter, stats::Stats},
    },
    functions::users::get_profile,
    i18n::*,
    providers::{
        calculate_initial_batch_size,
        chat::Chat,
        provide_games_search_context,
        AuthContext,
    },
};
use leptos::{either::Either, html, prelude::*};
use leptos_router::{
    hooks::{use_location, use_navigate, use_params},
    params::Params,
    NavigateOptions,
};
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
pub fn ProfileMe() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let navigate = use_navigate();
    Effect::new(move |_| {
        let opts = NavigateOptions {
            replace: true,
            ..Default::default()
        };
        match (auth.logged_in.get(), auth.user.get()) {
            (Some(true), Some(account)) => navigate(&format!("/@/{}", account.username), opts),
            (Some(false), _) => navigate("/login", opts),
            _ => {}
        }
    });
    view! { <p class="p-4 dark:text-white">"Redirecting…"</p> }
}

#[component]
pub fn ProfileView(children: ChildrenFn) -> impl IntoView {
    let params = use_params::<UsernameParams>();
    let username =
        move || params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default());
    let user = LocalResource::new(move || get_profile(username()));

    let games_container_ref = NodeRef::<html::Div>::new();
    let bounding = use_element_bounding(games_container_ref);
    let location = use_location();
    let current_tab = Signal::derive(move || tab_from_path(&location.pathname.get()));

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
        current_tab.get_untracked(),
    );

    let i18n = use_i18n();
    let auth_user = expect_context::<AuthContext>().user;
    let blocked_user_ids = expect_context::<Chat>().blocked_user_ids;
    let radio_classes = |active| {
        with_class(
            if active {
                "ui-choice ui-choice-sm ui-choice-active cursor-pointer"
            } else {
                "ui-choice ui-choice-sm ui-choice-inactive cursor-pointer"
            },
            "no-link-style",
        )
    };

    Effect::watch(
        ctx.next_batch.version(),
        move |_, _, _| {
            let Some(Ok(batch)) = ctx.next_batch.value().get_untracked() else {
                return;
            };
            ctx.next_batch_token.set(batch.next_batch.clone());
            if batch.next_batch.is_none() {
                ctx.has_more.set_value(false);
            }
            ctx.games.update(|games| {
                if ctx.is_first_batch.get_value() {
                    *games = batch.games;
                } else {
                    games.extend(batch.games);
                }
            });
        },
        true,
    );

    view! {
        <div class="flex overflow-hidden flex-col px-3 pt-12 bg-light h-[100vh] dark:bg-app-dark">
            <Transition fallback=move || {
                view! {
                    <div class="flex flex-1 justify-center items-center">
                        <EmptyState title="Loading Profile..." class="max-w-sm" />
                    </div>
                }
            }>
                {move || {
                    user.get()
                        .map(|user| {
                            match user {
                                Ok(user) => {
                                    let username = StoredValue::new(user.username.clone());
                                    let profile_user = user.clone();
                                    let profile_user_id = user.uid;
                                    let profile_is_bot = user.bot;
                                    let is_profile_blocked = Signal::derive(move || {
                                        blocked_user_ids
                                            .with(|blocked| blocked.contains(&profile_user_id))
                                    });
                                    Either::Left(
                                        view! {
                                            <div class="flex-shrink-0">
                                                <div class="flex flex-row flex-wrap justify-center mx-1 w-full text-lg sm:text-xl">
                                                    <div class="flex flex-wrap gap-2 justify-center items-center min-w-0">
                                                        <UserIdentity
                                                            user=profile_user
                                                            show_hover_ratings=false
                                                            link_class="truncate max-w-[125px]"
                                                        />
                                                        {(!profile_is_bot)
                                                            .then(|| {
                                                                view! {
                                                                    <Show when=move || {
                                                                        auth_user
                                                                            .with(|viewer| {
                                                                                viewer
                                                                                    .as_ref()
                                                                                    .is_some_and(|viewer| {
                                                                                        viewer.user.uid != profile_user_id
                                                                                    })
                                                                            })
                                                                    }>
                                                                        <MessageButton username=username.get_value() />
                                                                        <BlockToggleButton
                                                                            blocked_user_id=profile_user_id
                                                                            is_blocked=is_profile_blocked
                                                                        />
                                                                    </Show>
                                                                }
                                                            })}
                                                    </div>
                                                </div>

                                                <div class="lg:flex lg:flex-col lg:items-center lg:mx-auto lg:max-w-4xl">
                                                    <Stats user />

                                                    <div class="grid gap-1 items-start m-1 lg:flex lg:gap-4 lg:items-center lg:mt-4 grid-cols-[1fr_auto]">
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
                                }
                                Err(_) => {
                                    Either::Right(
                                        view! {
                                            <div class="flex flex-1 justify-center items-center">
                                                <EmptyState title="User not found" class="max-w-sm" />
                                            </div>
                                        },
                                    )
                                }
                            }
                        })
                }}
            </Transition>
        </div>
    }
}
