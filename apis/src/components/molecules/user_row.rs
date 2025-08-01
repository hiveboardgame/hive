use crate::{
    common::UserAction,
    components::atoms::{
        direct_challenge_button::DirectChallengeButton, invite_button::InviteButton,
        kick_button::KickButton, profile_link::ProfileLink, rating::Rating,
        status_indicator::StatusIndicator, uninvite_button::UninviteButton,
    },
    responses::UserResponse,
};
use leptos::{either::EitherOf5, prelude::*};
use shared_types::GameSpeed;

#[component]
pub fn UserRow(
    user: UserResponse,
    actions: Vec<UserAction>,
    #[prop(optional)] end_str: String,
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
        let user_id = user_id.get_value();
        actions
            .into_iter()
            .filter_map(|action| match action {
                UserAction::Challenge => Some(if user.bot {
                    //TODO: Allow users to direct challenge the bot once it can manage it's own time
                    EitherOf5::A(view! { <DirectChallengeButton user_id opponent=username.get_value() disabled=true /> })
                } else {
                    EitherOf5::B(
                        view! { <DirectChallengeButton user_id opponent=username.get_value() /> },
                    )
                }),
                UserAction::Invite(tournament_id) => Some(EitherOf5::C(
                    view! { <InviteButton user_id tournament_id /> },
                )),
                UserAction::Uninvite(tournament_id) => Some(EitherOf5::D(
                    view! { <UninviteButton user_id tournament_id /> },
                )),
                UserAction::Kick(tournament) => Some(EitherOf5::E(
                    view! { <KickButton user_id tournament=*tournament /> },
                )),
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
                        extend_tw_classes="truncate max-w-[125px]"
                        user_is_hoverable=user_is_hoverable.into()
                    />
                </div>
                <Show when=move || { rating.with_value(|r| r.is_some()) }>
                    <Rating rating=rating.get_value().expect("Rating is some") />
                </Show>

            </div>
            {display_actions}
            {end_str}
        </div>
    }
}
