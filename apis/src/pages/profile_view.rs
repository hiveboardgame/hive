use crate::{
    components::{
        atoms::{
            block_toggle_button::BlockToggleButton,
            profile_link::ProfileLink,
            status_indicator::StatusIndicator,
        },
        organisms::{games_filter::GamesFilter, stats::Stats},
    },
    functions::{
        blocks_mutes::get_blocked_user_ids,
        users::{can_message_user, get_profile},
    },
    i18n::*,
    providers::{
        calculate_initial_batch_size,
        chat::Chat,
        provide_games_search_context,
        AuthContext,
    },
};
use leptos::{either::Either, html, prelude::*};
use leptos_icons::*;
use leptos_router::{
    components::A,
    hooks::{use_location, use_params},
    params::Params,
};
use leptos_use::use_element_bounding;
use shared_types::GameProgress;
use uuid::Uuid;

#[component]
fn ProfileBlockUnblock(profile_user_id: Uuid) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let viewer_id =
        Signal::derive(move || auth_context.user.with(|u| u.as_ref().map(|a| a.user.uid)));
    let block_list = Resource::new(
        move || (viewer_id.get(), chat.block_list_version.get()),
        move |(viewer_id, _)| async move {
            if viewer_id.is_none() {
                Ok(Vec::new())
            } else {
                get_blocked_user_ids().await
            }
        },
    );
    let is_blocked = RwSignal::new(false);
    Effect::watch(
        move || block_list.get(),
        move |result, _, _| {
            if let Some(Ok(blocked)) = result.clone() {
                is_blocked.set(blocked.contains(&profile_user_id));
            }
        },
        true,
    );
    view! {
        <BlockToggleButton
            blocked_user_id=profile_user_id
            is_blocked=Signal::derive(move || is_blocked.get())
            on_success=Callback::new(move |is_now_blocked| is_blocked.set(is_now_blocked))
        />
    }
}

#[component]
fn ProfileHeaderActions(
    profile_user_id: Uuid,
    profile_username: String,
    profile_is_bot: bool,
    viewer_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let can_message = Resource::new(
        move || viewer_id.get(),
        move |viewer_id| async move {
            if profile_is_bot || viewer_id.is_none_or(|vid| vid == profile_user_id) {
                Ok(false)
            } else {
                can_message_user(profile_user_id).await
            }
        },
    );
    let show_actions =
        Signal::derive(move || viewer_id.get().is_some_and(|vid| vid != profile_user_id));
    let show_message = Signal::derive(move || matches!(can_message.get(), Some(Ok(true))));
    let message_href = StoredValue::new(format!(
        "/messages?dm={}&username={}",
        profile_user_id,
        urlencoding::encode(&profile_username)
    ));

    view! {
        <Show when=move || show_actions.get()>
            <Show when=move || show_message.get()>
                <A
                    href=move || message_href.get_value()
                    attr:class="no-link-style inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-pillbug-teal text-white hover:bg-pillbug-teal/90 dark:bg-pillbug-teal dark:text-white dark:hover:bg-pillbug-teal/90 transition-colors [&_svg]:text-inherit"
                >
                    <Icon
                        icon=icondata_hi::HiChatBubbleBottomCenterTextOutlineLg
                        attr:class="size-5 shrink-0"
                    />
                    {t!(i18n, messages.page.message_button)}
                </A>
            </Show>
            <ProfileBlockUnblock profile_user_id />
        </Show>
    }
}

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
    let auth_context = expect_context::<AuthContext>();
    let viewer_id = Memo::new(move |_| auth_context.user.with(|u| u.as_ref().map(|a| a.user.uid)));
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
        <div class="flex overflow-hidden flex-col px-3 pt-12 bg-light h-[100vh] dark:bg-gray-950">
            <Transition fallback=move || {
                view! { <p>"Loading Profile..."</p> }
            }>
                {move || {
                    user.get()
                        .map(|user| {
                            if let Ok(user) = user {
                                let username = StoredValue::new(user.username.clone());
                                let msg_uid = user.uid;
                                let msg_username = user.username.clone();
                                Either::Left(
                                    view! {
                                        <div class="flex-shrink-0">
                                            <div class="flex flex-row flex-wrap justify-center mx-1 w-full text-lg sm:text-xl">
                                                <div class="flex gap-2 items-center">
                                                    <span class="shrink-0 [&_svg]:!size-5 [&_svg]:!min-w-5 [&_svg]:!min-h-5">
                                                        <StatusIndicator username=username.get_value() />
                                                    </span>
                                                    <ProfileLink
                                                        patreon=user.patreon
                                                        bot=user.bot
                                                        username=username.get_value()
                                                        extend_tw_classes="truncate max-w-[125px]"
                                                    />
                                                    <ProfileHeaderActions
                                                        profile_user_id=msg_uid
                                                        profile_username=msg_username.clone()
                                                        profile_is_bot=user.bot
                                                        viewer_id=Signal::derive(move || viewer_id.get())
                                                    />
                                                </div>
                                            </div>

                                            <div class="lg:flex lg:flex-col lg:items-center lg:mx-auto lg:max-w-4xl">
                                                <Stats user />

                                                <div class="grid gap-1 items-start m-1 lg:flex lg:gap-4 lg:items-center lg:mt-4 grid-cols-[1fr_auto]">
                                                    <div class="flex flex-wrap gap-1 min-w-0">
                                                        <A
                                                            href=format!("/@/{}/unstarted", username.get_value())
                                                            attr:class=move || radio_classes(
                                                                current_tab() == GameProgress::Unstarted,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.pending)}
                                                        </A>
                                                        <A
                                                            href=format!("/@/{}/playing", username.get_value())
                                                            attr:class=move || radio_classes(
                                                                current_tab() == GameProgress::Playing,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.playing)}
                                                        </A>
                                                        <A
                                                            href=format!("/@/{}/finished", username.get_value())
                                                            attr:class=move || radio_classes(
                                                                current_tab() == GameProgress::Finished,
                                                            )
                                                        >
                                                            {t!(i18n, profile.game_buttons.finished)}
                                                        </A>
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
