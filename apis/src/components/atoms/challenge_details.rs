use crate::{
    common::{
        challenge_action_flags,
        challenge_displayed_player,
        challenge_viewer_role,
        with_class,
        ServerResult,
    },
    components::{
        atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator},
        molecules::time_row::TimeRow,
    },
    providers::{websocket::WebsocketContext, ApiRequestsProvider, AuthContext},
    responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TimeInfo;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    Accept,
    Remove,
}

const CHALLENGE_DETAILS_CLASS: &str =
    "relative flex w-full items-center gap-3 text-left text-sm text-gray-900 dark:text-gray-100";
const CHALLENGE_DETAILS_BODY_CLASS: &str = "min-w-0 flex-1";
const CHALLENGE_PLAYER_GRID_CLASS: &str =
    "grid min-w-0 grid-cols-[1.25rem_minmax(0,1fr)] items-center gap-x-1 gap-y-1";
const CHALLENGE_LEADING_CELL_CLASS: &str = "flex h-5 w-5 items-center justify-center";
const CHALLENGE_PLAYER_LINE_CLASS: &str = "flex min-w-0 items-baseline gap-1.5";
const CHALLENGE_RATING_CLASS: &str =
    "shrink-0 text-[13px] font-semibold leading-none tabular-nums text-gray-700 dark:text-gray-100";
const CHALLENGE_META_CLASS: &str =
    "flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-gray-600 dark:text-gray-300";
const CHALLENGE_ACTIONS_CLASS: &str = "relative z-20 flex shrink-0 gap-2";

#[component]
pub fn ChallengeDetails(
    challenge: ChallengeResponse,
    #[prop(optional)] label: Option<&'static str>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let admin = auth_context.admin;
    let api = expect_context::<ApiRequestsProvider>().0;
    let websocket = expect_context::<WebsocketContext>();
    let challenge = StoredValue::new(challenge);
    let ChallengeResponse {
        challenge_id,
        game_type,
        rated,
        color_choice,
        time_mode,
        time_base,
        time_increment,
        ..
    } = challenge.get_value();
    let challenge_id = StoredValue::new(challenge_id);
    let time_info = TimeInfo {
        mode: time_mode,
        base: time_base,
        increment: time_increment,
    };
    let color_icon = match color_choice {
        ColorChoice::Random => icondata_bs::BsHexagonHalf,
        ColorChoice::White => icondata_bs::BsHexagon,
        ColorChoice::Black => icondata_bs::BsHexagonFill,
    };
    let pending_action = RwSignal::new(None::<PendingAction>);
    Effect::new(move |_| {
        if matches!(websocket.message.get(), Some(ServerResult::Err(_))) {
            pending_action.set(None);
        }
    });
    let action_flags = Memo::new(move |_| {
        if pending_action.get().is_some() {
            return Default::default();
        }

        challenge.with_value(|challenge| {
            let viewer_id = user.with(|user| user.as_ref().map(|user| user.id));
            let role = challenge_viewer_role(challenge, viewer_id);
            challenge_action_flags(challenge, role, admin.get().unwrap_or(false), true)
        })
    });
    let show_accept = Signal::derive(move || action_flags.with(|flags| flags.accept));
    let show_decline_or_cancel = Signal::derive(move || {
        action_flags.with(|flags| flags.decline || flags.cancel || flags.admin_cancel)
    });
    let has_pending_action = Signal::derive(move || pending_action.get().is_some());
    let has_actions =
        Signal::derive(move || has_pending_action() || show_accept() || show_decline_or_cancel());

    view! {
        <div class=with_class(CHALLENGE_DETAILS_CLASS, class.unwrap_or_default())>
            <div class=CHALLENGE_DETAILS_BODY_CLASS>
                {label.map(|label| view! { <div class="ui-notification-label">{label}</div> })}
                <div class=CHALLENGE_PLAYER_GRID_CLASS>
                    {move || {
                        challenge
                            .with_value(|challenge| {
                                let viewer_id = user.with(|user| user.as_ref().map(|user| user.id));
                                let role = challenge_viewer_role(challenge, viewer_id);
                                let (user, rating) = challenge_displayed_player(challenge, role);
                                view! {
                                    <div class=CHALLENGE_LEADING_CELL_CLASS>
                                        <StatusIndicator
                                            username=user.username.clone()
                                            deleted=user.deleted
                                        />
                                    </div>
                                    <div class=CHALLENGE_PLAYER_LINE_CLASS>
                                        <ProfileLink
                                            username=user.username.clone()
                                            patreon=user.patreon
                                            bot=user.bot
                                            deleted=user.deleted
                                            extend_tw_classes="truncate max-w-[9rem]"
                                            wrapper_tw_classes="min-w-0 shrink"
                                        />
                                        <span class=CHALLENGE_RATING_CLASS>{rating}</span>
                                    </div>
                                }
                            })
                    }} <div class=CHALLENGE_LEADING_CELL_CLASS>
                        <Icon icon=color_icon attr:class="size-3.5 shrink-0" />
                    </div> <div class=CHALLENGE_META_CLASS>
                        <span>{game_type}</span>
                        <TimeRow time_info extend_tw_classes="text-xs leading-tight" />
                        <span class="font-bold">{if rated { "Rated" } else { "Casual" }}</span>
                    </div>
                </div>
            </div>
            <Show when=has_actions>
                <div class=CHALLENGE_ACTIONS_CLASS>
                    <Show
                        when=has_pending_action
                        fallback=move || {
                            view! {
                                <Show when=show_accept>
                                    <button
                                        title="Accept Challenge"
                                        on:click=move |_| {
                                            pending_action.set(Some(PendingAction::Accept));
                                            api.get().challenge_accept(challenge_id.get_value());
                                        }
                                        class="z-20 ui-button ui-button-primary ui-button-icon"
                                    >
                                        <Icon
                                            icon=icondata_ai::AiCheckOutlined
                                            attr:class="size-6"
                                        />
                                    </button>
                                </Show>
                                <Show when=show_decline_or_cancel>
                                    <button
                                        title=move || {
                                            action_flags
                                                .with(|flags| {
                                                    if flags.cancel || flags.admin_cancel {
                                                        "Cancel Challenge"
                                                    } else {
                                                        "Decline Challenge"
                                                    }
                                                })
                                        }
                                        on:click=move |_| {
                                            pending_action.set(Some(PendingAction::Remove));
                                            api.get().challenges_cancel(vec![challenge_id.get_value()]);
                                        }
                                        class="z-20 ui-button ui-button-danger ui-button-icon"
                                    >
                                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                                    </button>
                                </Show>
                            }
                        }
                    >
                        <span class="inline-flex items-center py-1 px-2 text-xs font-semibold text-gray-700 rounded-sm dark:text-gray-200 bg-surface-muted">
                            {move || match pending_action.get() {
                                Some(PendingAction::Accept) => "Accepting...",
                                Some(PendingAction::Remove) => "Removing...",
                                None => "",
                            }}
                        </span>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
