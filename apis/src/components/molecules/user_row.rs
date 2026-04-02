use crate::{
    common::UserAction,
    components::atoms::{
        block_button::BlockButton,
        direct_challenge_button::DirectChallengeButton,
        invite_button::InviteButton,
        kick_button::KickButton,
        profile_link::ProfileLink,
        rating::Rating,
        status_indicator::StatusIndicator,
        unblock_button::UnblockButton,
        uninvite_button::UninviteButton,
    },
    providers::AuthContext,
    responses::UserResponse,
};
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::GameSpeed;

#[component]
pub fn UserRow(
    user: UserResponse,
    actions: Vec<UserAction>,
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
    #[prop(optional)] on_profile: bool,
) -> impl IntoView {
    let username = StoredValue::new(user.username.clone());
    let user_is_hoverable = if on_profile { None } else { Some(user.clone()) };
    let user_id = StoredValue::new(user.uid);
    let rating = StoredValue::new(if let Some(speed) = game_speed {
        user.ratings.get(&speed.get_value()).cloned()
    } else {
        None
    });
    let color = if on_profile {
        "bg-light dark:bg-gray-950"
    } else {
        "dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light"
    };

    let display_actions = {
        let user_id_val = user_id.get_value();
        actions
            .iter()
            .filter_map(|action| match action {
                UserAction::Challenge => Some(view! {
                    <DirectChallengeButton
                        user_id=user_id_val
                        opponent=username.get_value()
                        disabled=user.bot
                    />
                }.into_any()),
                UserAction::Invite(tournament_id) => Some(view! {
                    <InviteButton user_id=user_id_val tournament_id=tournament_id.clone() />
                }.into_any()),
                UserAction::Uninvite(tournament_id) => Some(view! {
                    <UninviteButton user_id=user_id_val tournament_id=tournament_id.clone() />
                }.into_any()),
                UserAction::Kick(tournament) => Some(view! {
                    <KickButton user_id=user_id_val tournament=(**tournament).clone() />
                }.into_any()),
                _ => None,
            })
            .collect_view()
    };

    let (show_block, show_unblock) = {
        let has_block = actions.iter().any(|a| matches!(a, UserAction::Block));
        let has_unblock = actions.iter().any(|a| matches!(a, UserAction::Unblock));
        (has_block, has_unblock)
    };
    let show_message = actions.iter().any(|a| matches!(a, UserAction::Message));
    let user_id_for_buttons = user_id.get_value();
    let auth = expect_context::<AuthContext>();
    let message_href = move || {
        let username_encoded = urlencoding::encode(&user.username).to_string();
        format!("/messages?dm={}&username={}", user_id_for_buttons, username_encoded)
    };

    view! {
        <div class=format!("flex p-1 items-center justify-between h-10 w-64 {color}")>
            <div class="flex justify-between mr-2 w-48">
                <div class="flex items-center">
                    <StatusIndicator username=username.get_value() />
                    <ProfileLink
                        patreon=user.patreon
                        bot=user.bot
                        username=username.get_value()
                        extend_tw_classes="truncate max-w-[120px]"
                        user_is_hoverable=user_is_hoverable.into()
                    />
                </div>
                <Show when=move || { rating.with_value(|r| r.is_some()) }>
                    <Rating rating=rating.get_value().expect("Rating is some") />
                </Show>

            </div>
            <div class="flex items-center gap-1">
                {display_actions}
                <Show when=move || show_message && !user.bot && auth.user.get().as_ref().map_or(false, |me| me.user.uid != user_id_for_buttons)>
                    <a
                        href=message_href()
                        class="no-link-style inline-flex items-center justify-center size-8 rounded-lg text-white bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 transition-transform duration-300 [&_svg]:size-5 [&_svg]:shrink-0"
                        title="Message"
                    >
                        <Icon icon=icondata_hi::HiChatBubbleBottomCenterTextOutlineLg attr:class="size-5" />
                    </a>
                </Show>
                <Show when=move || show_block>
                    <BlockButton blocked_user_id=user_id_for_buttons />
                </Show>
                <Show when=move || show_unblock>
                    <UnblockButton blocked_user_id=user_id_for_buttons />
                </Show>
            </div>
        </div>
    }
}
