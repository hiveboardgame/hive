use crate::{
    common::{
        challenge_action_flags,
        challenge_displayed_color,
        challenge_displayed_player,
        challenge_viewer_role,
    },
    components::{
        atoms::{
            color_choice_hex::ColorChoiceHex,
            game_type::GameType,
            profile_link::ProfileLink,
            status_indicator::StatusIndicator,
        },
        molecules::{modal::Modal, time_row::TimeRow},
    },
    hooks::clipboard_copy::use_clipboard_copy,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext},
    responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::{html::Dialog, prelude::*};
use leptos_icons::*;
use leptos_use::use_window;
use shared_types::{ChallengeId, TimeInfo};

const CHALLENGE_LEADING_RAIL_CLASS: &str = "flex items-center justify-center gap-1";
const CHALLENGE_MOBILE_ACTIONS_CLASS: &str =
    "flex shrink-0 flex-col items-center justify-center gap-1 max-[359px]:min-h-[4.25rem] min-[360px]:max-[639px]:min-w-[4.25rem] min-[360px]:max-[639px]:flex-row sm:hidden";
const CHALLENGE_DESKTOP_ACTIONS_CLASS: &str =
    "hidden shrink-0 items-center justify-center gap-1 sm:flex";

#[component]
pub fn ChallengeRow(
    challenge: ChallengeResponse,
    #[prop(default = 1)] count: usize,
    #[prop(default = Vec::new())] challenge_ids: Vec<ChallengeId>,
) -> impl IntoView {
    let challenge_value = StoredValue::new(challenge.clone());
    let ChallengeResponse {
        challenge_id,
        game_type,
        rated,
        time_mode,
        time_base,
        time_increment,
        ..
    } = challenge;
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let admin = auth_context.admin;
    let challenge_id = StoredValue::new(challenge_id);
    let all_challenge_ids = StoredValue::new(challenge_ids);
    let group_count = count;
    let challenge_address = move || {
        let origin = use_window()
            .as_ref()
            .and_then(|window| window.location().origin().ok())
            .unwrap_or_default();
        format!("{origin}/challenge/{}", challenge_id.get_value())
    };
    let clipboard = use_clipboard_copy();
    let copy_state = clipboard.copied;
    let copy_text = clipboard.copy_text;
    let copy = move |_| copy_text(challenge_address());
    let copy_button_class = move || {
        if copy_state.get() {
            "ui-button ui-button-success ui-button-icon"
        } else {
            "ui-button ui-button-primary ui-button-icon"
        }
    };

    let td_class = "px-1 py-1 sm:py-2 sm:px-2";
    let accept_button_classes = StoredValue::new("ui-button ui-button-primary ui-button-icon");
    let cancel_button_classes = StoredValue::new("ui-button ui-button-danger ui-button-icon");
    let viewer_role = Memo::new(move |_| {
        let viewer_id = user.with(|user| user.as_ref().map(|user| user.id));
        challenge_value.with_value(|challenge| challenge_viewer_role(challenge, viewer_id))
    });
    let action_flags = Memo::new(move |_| {
        challenge_value.with_value(|challenge| {
            challenge_action_flags(
                challenge,
                viewer_role.get(),
                admin.get().unwrap_or(false),
                true,
            )
        })
    });
    let displayed_color = Memo::new(move |_| {
        challenge_value
            .with_value(|challenge| challenge_displayed_color(challenge, viewer_role.get()))
    });
    let color_label = move || match displayed_color.get() {
        ColorChoice::White => t_string!(i18n, home.challenge_details.plays.white),
        ColorChoice::Black => t_string!(i18n, home.challenge_details.plays.black),
        ColorChoice::Random => t_string!(i18n, home.challenge_details.plays.random),
    };
    let admin_cancel_dialog = NodeRef::<Dialog>::new();
    let admin_cancel_button_classes = StoredValue::new("ui-button ui-button-danger ui-button-icon");
    let admin_confirm_button_classes = StoredValue::new("ui-button ui-button-danger ui-button-sm");
    let admin_dismiss_button_classes =
        StoredValue::new("ui-button ui-button-secondary ui-button-sm");
    let challenge_ids_to_cancel = move || {
        let ids = all_challenge_ids.get_value();
        if ids.is_empty() {
            vec![challenge_id.get_value()]
        } else {
            ids
        }
    };

    let action_buttons = move || {
        view! {
            <Show when=move || action_flags.with(|flags| flags.admin_cancel)>
                <button
                    title=move || { t_string!(i18n, home.challenge_details.admin_cancel_title) }
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
            <Show when=move || action_flags.with(|flags| flags.copy_link)>
                <button on:click=copy class=copy_button_class>
                    <Icon icon=icondata_ai::AiCopyOutlined attr:class="size-6" />
                </button>
            </Show>
            <Show when=move || action_flags.with(|flags| flags.cancel)>
                <button
                    on:click=move |_| {
                        api.get().challenges_cancel(challenge_ids_to_cancel());
                    }
                    class=cancel_button_classes.get_value()
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                </button>
            </Show>
            <Show when=move || action_flags.with(|flags| flags.accept)>
                <button
                    on:click=move |_| {
                        api.get().challenge_accept(challenge_id.get_value());
                    }
                    class=accept_button_classes.get_value()
                >
                    <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />
                </button>
            </Show>
            <Show when=move || action_flags.with(|flags| flags.decline)>
                <button
                    on:click=move |_| {
                        api.get().challenges_cancel(challenge_ids_to_cancel());
                    }
                    class=cancel_button_classes.get_value()
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                </button>
            </Show>
        }
    };

    let time_info = TimeInfo {
        mode: time_mode,
        base: time_base,
        increment: time_increment,
    };
    view! {
        <tr class="cursor-pointer ui-dense-table-row">
            <td class=format!("w-20 sm:w-12 {td_class}")>
                <Show when=move || action_flags.with(|flags| flags.admin_cancel)>
                    <Modal dialog_el=admin_cancel_dialog>
                        <div class="flex flex-col items-center p-4 max-w-xs">
                            <p class="mb-4 text-center">
                                {t!(i18n, home.challenge_details.admin_cancel_confirm)}
                            </p>
                            <div class="flex gap-2">
                                <button
                                    class=admin_confirm_button_classes.get_value()
                                    on:click=move |_| {
                                        api.get().challenges_cancel(challenge_ids_to_cancel());
                                        if let Some(dialog) = admin_cancel_dialog.get() {
                                            dialog.close();
                                        }
                                    }
                                >
                                    {t!(i18n, home.challenge_details.admin_cancel_confirm_button)}
                                </button>
                                <button
                                    class=admin_dismiss_button_classes.get_value()
                                    on:click=move |_| {
                                        if let Some(dialog) = admin_cancel_dialog.get() {
                                            dialog.close();
                                        }
                                    }
                                >
                                    {t!(i18n, home.challenge_details.admin_cancel_dismiss_button)}
                                </button>
                            </div>
                        </div>
                    </Modal>
                </Show>
                <div class=CHALLENGE_LEADING_RAIL_CLASS>
                    <span class=move || {
                        if group_count > 1 {
                            "inline-flex h-5 items-center justify-center text-[10px] font-bold leading-none text-gray-900 dark:text-gray-100 sm:min-w-7 sm:justify-start sm:text-xs"
                        } else {
                            "hidden h-5 items-center justify-center text-[10px] font-bold leading-none text-gray-900 dark:text-gray-100 sm:invisible sm:inline-flex sm:min-w-7 sm:justify-start sm:text-xs"
                        }
                    }>{format!("x{}", group_count.max(1))}</span>
                    <div class=CHALLENGE_MOBILE_ACTIONS_CLASS>{action_buttons()}</div>
                </div>
            </td>
            <td class=format!("w-16 xs:w-20 sm:w-24 {td_class}")>
                <div class="flex justify-center items-center">
                    {move || {
                        challenge_value
                            .with_value(|challenge| {
                                let (user, _) = challenge_displayed_player(
                                    challenge,
                                    viewer_role.get(),
                                );
                                view! {
                                    <div class="flex items-center">
                                        <StatusIndicator
                                            username=user.username.clone()
                                            deleted=user.deleted
                                        />
                                        <ProfileLink
                                            username=user.username.clone()
                                            patreon=user.patreon
                                            bot=user.bot
                                            deleted=user.deleted
                                            extend_tw_classes="truncate max-w-[60px] xs:max-w-[80px] sm:max-w-[120px] md:max-w-[140px] lg:max-w-[160px]"
                                        />
                                    </div>
                                }
                            })
                    }}
                </div>
            </td>
            <td class=td_class>
                <div class="flex gap-1 justify-center items-center">
                    {move || {
                        view! {
                            <ColorChoiceHex
                                color_choice=displayed_color.get()
                                extend_tw_classes="size-4"
                            />
                        }
                    }} <span class="hidden md:inline">{color_label}</span>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <p>
                        {move || {
                            challenge_value
                                .with_value(|challenge| {
                                    let (_, rating) = challenge_displayed_player(
                                        challenge,
                                        viewer_role.get(),
                                    );
                                    rating
                                })
                        }}
                    </p>
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
                                t_string!(i18n, common.yes)
                            } else {
                                t_string!(i18n, common.no)
                            }
                        }}

                    </span>
                </div>
            </td>
            <td class=format!("hidden sm:table-cell {td_class}")>
                <div class=CHALLENGE_DESKTOP_ACTIONS_CLASS>{action_buttons()}</div>
            </td>
        </tr>
    }
}
