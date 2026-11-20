use crate::{
    common::login_redirect_url,
    components::{
        atoms::{
            block_toggle_button::BlockToggleButton,
            direct_challenge_button::DirectChallengeButton,
            message_button::MessageButton,
        },
        molecules::{empty_state::EmptyState, user_identity::UserIdentity},
        organisms::{games_filter::GamesFilter, stats::Stats},
    },
    functions::users::get_profile,
    i18n::*,
    providers::{
        calculate_initial_batch_size,
        provide_games_search_context,
        AuthContext,
        AuthIdentity,
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
    if path.ends_with("/tournaments/finished") || path.ends_with("/tournaments/upcoming") {
        GameProgress::Playing
    } else if path.ends_with("/unstarted") {
        GameProgress::Unstarted
    } else if path.ends_with("/playing") {
        GameProgress::Playing
    } else if path.ends_with("/finished") {
        GameProgress::Finished
    } else {
        GameProgress::Playing
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProfileTab {
    Games(GameProgress),
    Tournaments,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TournamentView {
    Upcoming,
    Current,
    Finished,
}

pub fn tournament_view_from_path(path: &str) -> TournamentView {
    if path.ends_with("/tournaments/upcoming") {
        TournamentView::Upcoming
    } else if path.ends_with("/tournaments/finished") {
        TournamentView::Finished
    } else {
        TournamentView::Current
    }
}

pub fn profile_tab_from_path(path: &str) -> ProfileTab {
    if path.ends_with("/tournaments")
        || path.ends_with("/tournaments/upcoming")
        || path.ends_with("/tournaments/finished")
    {
        ProfileTab::Tournaments
    } else {
        ProfileTab::Games(tab_from_path(path))
    }
}

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[component]
pub fn ProfileMe(#[prop(optional)] tab: &'static str) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let navigate = use_navigate();
    Effect::new(move |_| {
        let opts = NavigateOptions {
            replace: true,
            ..Default::default()
        };
        match (auth.identity.get(), auth.user.get()) {
            (Some(AuthIdentity::User(_)), Some(account)) => {
                navigate(&format!("/@/{}{tab}", account.username), opts)
            }
            (Some(AuthIdentity::Anonymous), _) => navigate(&untrack(login_redirect_url), opts),
            _ => {}
        }
    });
    view! { <p class="p-4 dark:text-white">{t!(i18n, profile.page.redirecting)}</p> }
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
    let profile_tab = Signal::derive(move || profile_tab_from_path(&location.pathname.get()));
    let tournament_view =
        Signal::derive(move || tournament_view_from_path(&location.pathname.get()));

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
    let segment_classes = |active| {
        let state = if active {
            "bg-button-dawn text-white dark:bg-button-dawn"
        } else {
            "bg-even-light text-gray-800 hover:bg-blue-light/70 dark:bg-surface-raised dark:text-gray-100 dark:hover:bg-pillbug-teal/15"
        };
        format!(
            "inline-flex min-h-10 items-center justify-center px-3 text-sm font-bold leading-tight no-link-style transition-colors focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-pillbug-teal sm:min-h-7 {state}"
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
                        <EmptyState
                            title=move || t_string!(i18n, common.loading)
                            class="max-w-sm"
                        />
                    </div>
                }
            }>
                {move || {
                    user.get()
                        .map(|user| {
                            match user {
                                Ok(user) => {
                                    let username = StoredValue::new(user.username.clone());
                                    let ctx = ctx.clone();
                                    let profile_user = user.clone();
                                    let profile_user_id = user.uid;
                                    let profile_is_bot = user.bot;
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

                                                        <DirectChallengeButton
                                                            user_id=user.uid
                                                            opponent=username.get_value()
                                                        />

                                                        <Show when=move || {
                                                            !profile_is_bot
                                                                && auth_user
                                                                    .with(|viewer| {
                                                                        viewer
                                                                            .as_ref()
                                                                            .is_some_and(|viewer| {
                                                                                viewer.user.uid != profile_user_id
                                                                            })
                                                                    })
                                                        }>
                                                            <MessageButton username=username.get_value() />
                                                            <BlockToggleButton blocked_user_id=profile_user_id />
                                                        </Show>
                                                    </div>
                                                </div>

                                                <div class="lg:flex lg:flex-col lg:items-center lg:mx-auto lg:max-w-4xl">
                                                    <Stats user />

                                                    <div class="flex flex-wrap gap-2 items-center mx-1 mt-1 mb-3 sm:gap-3 sm:mb-4 lg:justify-center lg:mt-4">
                                                        <Show
                                                            when=move || profile_tab() != ProfileTab::Tournaments
                                                            fallback=move || {
                                                                view! {
                                                                    <a
                                                                        href=format!("/@/{}/playing", username.get_value())
                                                                        class=format!(
                                                                            "{} {}",
                                                                            "rounded border border-black/10 dark:border-white/10",
                                                                            segment_classes(false),
                                                                        )
                                                                    >
                                                                        {t!(i18n, common.games)}
                                                                    </a>
                                                                    <span
                                                                        class="w-px h-6 bg-black/20 dark:bg-white/20"
                                                                        aria-hidden="true"
                                                                    ></span>
                                                                    <div
                                                                        role="group"
                                                                        aria-label=move || t_string!(i18n, common.tournaments)
                                                                        class="inline-flex rounded border divide-x border-black/10 divide-black/10 [&>:first-child]:rounded-l [&>:last-child]:rounded-r dark:border-white/10 dark:divide-white/10"
                                                                    >
                                                                        <span class="hidden items-center px-3 text-xs font-bold tracking-wide text-gray-600 uppercase sm:inline-flex dark:text-gray-300 bg-odd-light dark:bg-surface-muted">
                                                                            {t!(i18n, common.tournaments)}
                                                                        </span>
                                                                        <a
                                                                            href=format!(
                                                                                "/@/{}/tournaments/upcoming",
                                                                                username.get_value(),
                                                                            )
                                                                            aria-current=move || {
                                                                                (tournament_view() == TournamentView::Upcoming)
                                                                                    .then_some("page")
                                                                            }
                                                                            class=move || segment_classes(
                                                                                tournament_view() == TournamentView::Upcoming,
                                                                            )
                                                                        >
                                                                            {t!(i18n, common.upcoming)}
                                                                        </a>
                                                                        <a
                                                                            href=format!("/@/{}/tournaments", username.get_value())
                                                                            aria-current=move || {
                                                                                (tournament_view() == TournamentView::Current)
                                                                                    .then_some("page")
                                                                            }
                                                                            class=move || segment_classes(
                                                                                tournament_view() == TournamentView::Current,
                                                                            )
                                                                        >
                                                                            {t!(i18n, profile.tournaments.current)}
                                                                        </a>
                                                                        <a
                                                                            href=format!(
                                                                                "/@/{}/tournaments/finished",
                                                                                username.get_value(),
                                                                            )
                                                                            aria-current=move || {
                                                                                (tournament_view() == TournamentView::Finished)
                                                                                    .then_some("page")
                                                                            }
                                                                            class=move || segment_classes(
                                                                                tournament_view() == TournamentView::Finished,
                                                                            )
                                                                        >
                                                                            {t!(i18n, common.finished)}
                                                                        </a>
                                                                    </div>
                                                                }
                                                            }
                                                        >
                                                            <div
                                                                role="group"
                                                                aria-label=move || t_string!(i18n, common.games)
                                                                class="inline-flex rounded border divide-x border-black/10 divide-black/10 [&>:first-child]:rounded-l [&>:last-child]:rounded-r dark:border-white/10 dark:divide-white/10"
                                                            >
                                                                <span class="hidden items-center px-3 text-xs font-bold tracking-wide text-gray-600 uppercase sm:inline-flex dark:text-gray-300 bg-odd-light dark:bg-surface-muted">
                                                                    {t!(i18n, common.games)}
                                                                </span>
                                                                <a
                                                                    href=format!("/@/{}/unstarted", username.get_value())
                                                                    aria-current=move || {
                                                                        (profile_tab()
                                                                            == ProfileTab::Games(GameProgress::Unstarted))
                                                                            .then_some("page")
                                                                    }
                                                                    class=move || segment_classes(
                                                                        profile_tab() == ProfileTab::Games(GameProgress::Unstarted),
                                                                    )
                                                                >
                                                                    {t!(i18n, common.pending)}
                                                                </a>
                                                                <a
                                                                    href=format!("/@/{}/playing", username.get_value())
                                                                    aria-current=move || {
                                                                        (profile_tab() == ProfileTab::Games(GameProgress::Playing))
                                                                            .then_some("page")
                                                                    }
                                                                    class=move || segment_classes(
                                                                        profile_tab() == ProfileTab::Games(GameProgress::Playing),
                                                                    )
                                                                >
                                                                    {t!(i18n, common.playing)}
                                                                </a>
                                                                <a
                                                                    href=format!("/@/{}/finished", username.get_value())
                                                                    aria-current=move || {
                                                                        (profile_tab() == ProfileTab::Games(GameProgress::Finished))
                                                                            .then_some("page")
                                                                    }
                                                                    class=move || segment_classes(
                                                                        profile_tab() == ProfileTab::Games(GameProgress::Finished),
                                                                    )
                                                                >
                                                                    {t!(i18n, common.finished)}
                                                                </a>
                                                                <GamesFilter
                                                                    username=username.get_value()
                                                                    ctx=ctx.clone()
                                                                />
                                                            </div>
                                                            <span
                                                                class="w-px h-6 bg-black/20 dark:bg-white/20"
                                                                aria-hidden="true"
                                                            ></span>
                                                            <a
                                                                href=format!("/@/{}/tournaments", username.get_value())
                                                                class=format!(
                                                                    "{} {}",
                                                                    "rounded border border-black/10 dark:border-white/10",
                                                                    segment_classes(false),
                                                                )
                                                            >
                                                                {t!(i18n, common.tournaments)}
                                                            </a>
                                                        </Show>
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
                                                <EmptyState
                                                    title=move || t_string!(i18n, profile.page.user_not_found)
                                                    class="max-w-sm"
                                                />
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
