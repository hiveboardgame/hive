use crate::{
    common::UserAction,
    components::atoms::{
        direct_challenge_button::DirectChallengeButton, invite_button::InviteButton,
        kick_button::KickButton, profile_link::ProfileLink, rating::Rating,
        status_indicator::StatusIndicator, uninvite_button::UninviteButton,
    },
    responses::UserResponse,
};
use leptos::{either::EitherOf4, prelude::*};
use shared_types::GameSpeed;

#[component]
pub fn UserRow(
    user: StoredValue<UserResponse>,
    actions: Vec<UserAction>,
    #[prop(optional)] end_str: String,
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
    #[prop(optional)] on_profile: bool,
) -> impl IntoView {
    let user = Signal::derive(move || user.get_value());
    let rating = move || {
        if let Some(speed) = game_speed {
            user().ratings.get(&speed.get_value()).cloned()
        } else {
            None
        }
    };
    let color = if on_profile {
        "bg-light dark:bg-gray-950"
    } else {
        "dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light"
    };
    let profile_link = move || {
        if on_profile {
            view! {
                <ProfileLink
                    patreon=user().patreon
                    username=user().username
                    extend_tw_classes="truncate max-w-[120px]"
                />
            }
        } else {
            view! {
                <ProfileLink
                    patreon=user().patreon
                    username=user().username
                    extend_tw_classes="truncate max-w-[120px]"
                    user_is_hoverable=user()
                />
            }
        }
    };

    let display_actions = move || {
        let mut views = vec![];
        for action in actions {
            match action {
                UserAction::Challenge => {
                    views.push(EitherOf4::A(
                        view! { <DirectChallengeButton user=user() /> },
                    ));
                }
                UserAction::Invite(tournament_id) => {
                    views.push(EitherOf4::B(
                        view! { <InviteButton user=user() tournament_id /> },
                    ));
                }
                UserAction::Uninvite(tournament_id) => {
                    views.push(EitherOf4::C(
                        view! { <UninviteButton user=user() tournament_id /> },
                    ));
                }
                UserAction::Kick(tournament) => {
                    views.push(EitherOf4::D(
                        view! { <KickButton user=user() tournament=*tournament /> },
                    ));
                }
                _ => {}
            };
        }
        views.collect_view()
    };

    view! {
        <div class=format!("flex p-1 items-center justify-between h-10 w-64 {color}")>
            <div class="flex justify-between mr-2 w-48">
                <div class="flex items-center">
                    <StatusIndicator username=user().username />
                    {profile_link()}
                </div>
                <Show when=move || { rating().is_some() }>
                    <Rating rating=rating().expect("Rating is some") />
                </Show>

            </div>
            {display_actions()}
            {end_str}
        </div>
    }
}
