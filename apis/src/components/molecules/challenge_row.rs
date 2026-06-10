use crate::{
    components::{
        atoms::{
            game_type::GameType,
            profile_link::ProfileLink,
            status_indicator::StatusIndicator,
        },
        molecules::{modal::Modal, time_row::TimeRow},
    },
    functions::{auth::guest::guest_login, hostname::hostname_and_port},
    i18n::*,
    providers::{
        websocket::{ConnectionReadyState, WebsocketContext},
        ApiRequestsProvider,
        AuthContext,
        Config,
    },
    responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::{either::Either, html::Dialog, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};
use shared_types::{ChallengeId, ChallengeVisibility, TimeInfo};

const BUTTON_BASE_CLASSES: &str = "px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform active:scale-95 focus:outline-none focus:shadow-outline font-bold";

#[component]
pub fn ChallengeRow(
    challenge: ChallengeResponse,
    single: bool,
    #[prop(default = 1)] count: usize,
    #[prop(default = Vec::new())] challenge_ids: Vec<ChallengeId>,
) -> impl IntoView {
    let ChallengeResponse {
        challenge_id,
        challenger,
        opponent,
        game_type,
        rated,
        visibility,
        color_choice,
        challenger_rating,
        time_mode,
        time_base,
        time_increment,
        speed,
        ..
    } = challenge;
    let i18n = use_i18n();
    let config = expect_context::<Config>().0;
    let api = expect_context::<ApiRequestsProvider>().0;
    let user = expect_context::<AuthContext>().user;
    let challenger_id = challenger.uid;
    let opponent_id = opponent.as_ref().map(|o| o.uid);
    let has_opponent = opponent.is_some();
    let challenge_id = StoredValue::new(challenge_id);
    let visibility = StoredValue::new(visibility);
    let all_challenge_ids = StoredValue::new(challenge_ids);
    let group_count = count;
    let color_choice = StoredValue::new(color_choice);

    // Accepting while logged out provisions a guest, then sends the accept once
    // the websocket has reconnected as that guest (send() drops messages while
    // the socket is mid-reconnect). Rated challenges aren't open to guests, so
    // send those visitors to login instead.
    let auth_context = expect_context::<AuthContext>();
    let ready_state = expect_context::<WebsocketContext>().ready_state;
    let guest_action = Action::new(|_: &()| async { guest_login().await });
    let pending_accept = RwSignal::new(false);
    Effect::watch(
        move || guest_action.value().get(),
        move |val, _, _| {
            if let Some(Ok(_)) = val {
                auth_context.refresh(true);
            }
        },
        false,
    );
    Effect::watch(
        move || (ready_state.get(), user.with(|u| u.is_some())),
        move |(rs, has_user), _, _| {
            if pending_accept.get_untracked() && *rs == ConnectionReadyState::Open && *has_user {
                api.get().challenge_accept(challenge_id.get_value());
                pending_accept.set(false);
            }
        },
        false,
    );
    let accept = move || {
        if user.with(|a| a.is_some()) {
            api.get().challenge_accept(challenge_id.get_value());
        } else if rated {
            use_navigate()("/login", Default::default());
        } else {
            pending_accept.set(true);
            guest_action.dispatch(());
        }
    };

    let icon = move || {
        let prefers_dark = config.with(|c| c.prefers_dark);
        match color_choice.get_value() {
            ColorChoice::Random => icondata_bs::BsHexagonHalf,
            ColorChoice::White => {
                if prefers_dark {
                    icondata_bs::BsHexagonFill
                } else {
                    icondata_bs::BsHexagon
                }
            }
            ColorChoice::Black => {
                if prefers_dark {
                    icondata_bs::BsHexagon
                } else {
                    icondata_bs::BsHexagonFill
                }
            }
        }
    };
    let icon_class = move || {
        let prefers_dark = config.with(|c| c.prefers_dark);
        match color_choice.get_value() {
            ColorChoice::Random => "pb-[2px]",
            ColorChoice::White => {
                if prefers_dark {
                    "fill-white pb-[2px]"
                } else {
                    "stroke-black pb-[2px]"
                }
            }
            ColorChoice::Black => {
                if prefers_dark {
                    "stroke-white pb-[2px]"
                } else {
                    "fill-black pb-[2px]"
                }
            }
        }
    };
    let challenge_address = move || {
        format!(
            "{}/challenge/{}",
            hostname_and_port(),
            challenge_id.get_value()
        )
    };
    let copy_state = RwSignal::new(false);

    let interval = StoredValue::new(use_interval_fn_with_options(
        move || copy_state.set(false),
        2000, // 2 seconds
        UseIntervalFnOptions::default().immediate(false),
    ));

    let copy = move |_| {
        let interval = interval.get_value();
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist")
            .navigator()
            .clipboard();
        let _ = clipboard.write_text(&challenge_address());
        copy_state.set(true);
        (interval.pause)();
        (interval.resume)();
    };
    let copy_button_class = move || {
        if copy_state.get() {
            format!("{BUTTON_BASE_CLASSES} bg-grasshopper-green hover:bg-green-500")
        } else {
            format!("{BUTTON_BASE_CLASSES} bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal")
        }
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let accept_button_classes = StoredValue::new(format!("{BUTTON_BASE_CLASSES} bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"));
    let cancel_button_classes = StoredValue::new(format!(
        "{BUTTON_BASE_CLASSES} bg-ladybug-red hover:bg-red-400"
    ));
    let displayed_user = Memo::new(move |_| {
        if user.with(|a| a.as_ref().map(|user| user.id)) == Some(challenger_id) {
            if let Some(opponent) = opponent.as_ref() {
                return (
                    opponent.username.clone(),
                    opponent.patreon,
                    opponent.bot,
                    opponent.rating_for_speed(&speed),
                );
            }
        }

        (
            challenger.username.clone(),
            challenger.patreon,
            challenger.bot,
            challenger_rating,
        )
    });
    let viewer_is_challenger =
        move || user.with(|a| a.as_ref().map(|user| user.id)) == Some(challenger_id);
    let viewer_is_opponent = move || {
        opponent_id.is_some() && user.with(|a| a.as_ref().map(|user| user.id)) == opponent_id
    };
    let viewer_is_admin = move || user.with(|a| a.as_ref().is_some_and(|v| v.user.admin));
    let show_admin_cancel =
        move || viewer_is_admin() && !viewer_is_challenger() && !viewer_is_opponent();
    let admin_cancel_dialog = NodeRef::<Dialog>::new();
    let admin_cancel_button_classes = StoredValue::new(format!(
        "{BUTTON_BASE_CLASSES} bg-orange-500 hover:bg-orange-400"
    ));
    let admin_confirm_button_classes = StoredValue::new(format!(
        "{BUTTON_BASE_CLASSES} bg-ladybug-red hover:bg-red-400"
    ));
    let admin_dismiss_button_classes = StoredValue::new(format!(
        "{BUTTON_BASE_CLASSES} bg-blue-600 hover:bg-blue-700"
    ));

    let time_info = TimeInfo {
        mode: time_mode,
        base: time_base,
        increment: time_increment,
    };
    view! {
        <tr class="items-center text-center cursor-pointer max-w-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <td class=td_class>
                <div>
                    <Icon icon=Signal::derive(icon) attr:class=Signal::derive(icon_class) />
                </div>
            </td>
            <td class=format!("w-10 sm:w-24 {td_class}")>
                <div class="flex justify-center items-center">
                    {move || {
                        displayed_user
                            .with(|(username, patreon, bot, _)| {
                                view! {
                                    <div class="flex items-center">
                                        <StatusIndicator username=username.clone() />
                                        <ProfileLink
                                            username=username.clone()
                                            patreon=*patreon
                                            bot=*bot
                                            extend_tw_classes="truncate max-w-[60px] xs:max-w-[80px] sm:max-w-[120px] md:max-w-[140px] lg:max-w-[160px]"
                                        />
                                        {if group_count > 1 {
                                            Either::Left(
                                                view! {
                                                    <span class="py-0.5 px-1.5 ml-1 text-xs font-bold text-white rounded-full bg-pillbug-teal">
                                                        {format!("x{}", group_count)}
                                                    </span>
                                                },
                                            )
                                        } else {
                                            Either::Right(view! { "" })
                                        }}
                                    </div>
                                }
                            })
                    }}
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <p>{move || displayed_user.with(|(_, _, _, rating)| *rating)}</p>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <GameType game_type />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <TimeRow
                        time_info
                        extend_tw_classes="break-words text-xs sm:text-sm max-w-[40px] xs:max-w-[50px] sm:max-w-[60px] md:max-w-[80px] lg:max-w-[100px] whitespace-normal"
                    />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <span class="font-bold">
                        {move || {
                            if rated {
                                t_string!(i18n, home.challenge_details.rated.yes)
                            } else {
                                t_string!(i18n, home.challenge_details.rated.no)
                            }
                        }}

                    </span>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <Show when=show_admin_cancel>
                        <Modal dialog_el=admin_cancel_dialog>
                            <div class="flex flex-col items-center p-4 max-w-xs">
                                <p class="mb-4 text-center">
                                    {t!(i18n, home.challenge_details.admin_cancel_confirm)}
                                </p>
                                <div class="flex gap-2">
                                    <button
                                        class=admin_confirm_button_classes.get_value()
                                        on:click=move |_| {
                                            let ids = all_challenge_ids.get_value();
                                            let ids_to_cancel = if ids.is_empty() {
                                                vec![challenge_id.get_value()]
                                            } else {
                                                ids
                                            };
                                            api.get().challenges_cancel(ids_to_cancel);
                                            if let Some(dialog) = admin_cancel_dialog.get() {
                                                dialog.close();
                                            }
                                        }
                                    >
                                        {t!(
                                            i18n, home.challenge_details.admin_cancel_confirm_button
                                        )}
                                    </button>
                                    <button
                                        class=admin_dismiss_button_classes.get_value()
                                        on:click=move |_| {
                                            if let Some(dialog) = admin_cancel_dialog.get() {
                                                dialog.close();
                                            }
                                        }
                                    >
                                        {t!(
                                            i18n, home.challenge_details.admin_cancel_dismiss_button
                                        )}
                                    </button>
                                </div>
                            </div>
                        </Modal>
                        <button
                            title=move || {
                                t_string!(i18n, home.challenge_details.admin_cancel_title)
                            }
                            on:click=move |_| {
                                if let Some(dialog) = admin_cancel_dialog.get() {
                                    let _ = dialog.show_modal();
                                }
                            }
                            class=admin_cancel_button_classes.get_value()
                        >
                            <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                        </button>
                    </Show>
                    <Show
                        when=move || { !viewer_is_challenger() }

                        fallback=move || {
                            view! {
                                <Show when=move || {
                                    visibility.with_value(|v| *v == ChallengeVisibility::Private)
                                        && !single
                                }>
                                    <button on:click=copy class=copy_button_class>
                                        <Icon
                                            icon=icondata_ai::AiCopyOutlined
                                            attr:class="size-6"
                                        />
                                    </button>
                                </Show>
                                <button
                                    on:click=move |_| {
                                        let ids = all_challenge_ids.get_value();
                                        let ids_to_cancel = if ids.is_empty() {
                                            vec![challenge_id.get_value()]
                                        } else {
                                            ids
                                        };
                                        api.get().challenges_cancel(ids_to_cancel);
                                    }
                                    class=cancel_button_classes.get_value()
                                >
                                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                                </button>
                            }
                        }
                    >

                        <button on:click=move |_| accept() class=accept_button_classes.get_value()>
                            <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />

                        </button>
                        {if has_opponent {
                            Either::Left(
                                view! {
                                    <button
                                        on:click=move |_| {
                                            let ids = all_challenge_ids.get_value();
                                            let ids_to_cancel = if ids.is_empty() {
                                                vec![challenge_id.get_value()]
                                            } else {
                                                ids
                                            };
                                            api.get().challenges_cancel(ids_to_cancel);
                                        }
                                        class=cancel_button_classes.get_value()
                                    >
                                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />

                                    </button>
                                },
                            )
                        } else {
                            Either::Right(view! { "" })
                        }}

                    </Show>
                </div>
            </td>
        </tr>
    }
}
