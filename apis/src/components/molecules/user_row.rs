use crate::{
    common::UserAction,
    components::atoms::{
        direct_challenge_button::DirectChallengeButton,
        invite_button::InviteButton,
        kick_button::KickButton,
        message_button::MessageButton,
        profile_link::ProfileLink,
        rating::Rating,
        status_indicator::StatusIndicator,
        uninvite_button::UninviteButton,
    },
    providers::AuthContext,
    responses::UserResponse,
};
use leptos::prelude::*;
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
    let user_id_for_buttons = user_id.get_value();
    let auth = expect_context::<AuthContext>();

    let display_actions = {
        let user_id_val = user_id_for_buttons;
        let username_for_actions = user.username.clone();
        actions
            .iter()
            .filter_map(|action| match action {
                UserAction::Challenge => Some(
                    view! {
                    <DirectChallengeButton
                        user_id=user_id_val
                        opponent=username.get_value()
                        disabled=user.bot
                    />
                }
                    .into_any(),
                ),
                UserAction::Invite(tournament_id) => Some(
                    view! { <InviteButton user_id=user_id_val tournament_id=tournament_id.clone() /> }
                        .into_any(),
                ),
                UserAction::Uninvite(tournament_id) => Some(
                    view! { <UninviteButton user_id=user_id_val tournament_id=tournament_id.clone() /> }
                        .into_any(),
                ),
                UserAction::Kick(tournament) => Some(
                    view! { <KickButton user_id=user_id_val tournament=(**tournament).clone() /> }
                        .into_any(),
                ),
                UserAction::Message => {
                    let message_username = username_for_actions.clone();
                    Some(
                        view! {
                        <Show when=move || {
                            !user.bot
                                && auth
                                    .user
                                    .get()
                                    .as_ref()
                                    .is_some_and(|me| me.user.uid != user_id_for_buttons)
                        }>
                            <MessageButton
                                username=message_username.clone()
                                compact=true
                            />
                        </Show>
                    }
                        .into_any(),
                    )
                }
                _ => None,
            })
            .collect_view()
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
            <div class="flex gap-1 items-center">{display_actions}</div>
        </div>
    }
}
