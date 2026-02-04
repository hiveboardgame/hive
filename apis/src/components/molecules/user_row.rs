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
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
    #[prop(optional)] on_profile: bool,
    #[prop(optional)] selection_mode: bool,
) -> impl IntoView {
    let username = StoredValue::new(user.username.clone());
    let user_is_hoverable = StoredValue::new(if on_profile || selection_mode {
        None
    } else {
        Some(user.clone())
    });
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

    let (display_actions, select_callback): (_, Option<Callback<String>>) = {
        let user_id = user_id.get_value();
        let mut select_cb: Option<Callback<String>> = None;
        let display: Vec<_> = actions
            .into_iter()
            .filter_map(|action| match action {
                UserAction::Challenge => Some(if user.bot {
                    // TODO: Allow users to direct challenge the bot once it can manage its own time
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
                UserAction::Select(cb) => {
                    select_cb = Some(Callback::new(move |username: String| cb.run(Some(username))));
                    None
                }
                _ => None,
            })
            .collect();
        (display.into_iter().collect_view(), select_cb)
    };

    let has_select = select_callback.is_some();
    let row_class = if has_select {
        format!("flex p-1 items-center justify-between h-10 w-64 {color} cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 rounded")
    } else {
        format!("flex p-1 items-center justify-between h-10 w-64 {color}")
    };

    let username_for_click = user.username.clone();
    let click_handler = move |_| {
        if let Some(ref cb) = select_callback {
            cb.run(username_for_click.clone());
        }
    };

    view! {
        <div
            class=row_class
            on:click=click_handler
        >
            <div class="flex justify-between mr-2 w-48">
                <div class="flex items-center">
                    <StatusIndicator username=username.get_value() />
                    <Show
                        when=move || !selection_mode
                        fallback=move || {
                            view! {
                                <span class="truncate max-w-[120px] font-bold text-xs">
                                    {user.username.clone()}
                                    <Show when=move || user.bot>
                                        <span class="text-[80%] ml-1">"BOT"</span>
                                    </Show>
                                </span>
                            }
                        }
                    >
                        <ProfileLink
                            patreon=user.patreon
                            bot=user.bot
                            username=username.get_value()
                            extend_tw_classes="truncate max-w-[120px]"
                            user_is_hoverable=user_is_hoverable.get_value().into()
                        />
                    </Show>
                </div>
                <Show when=move || { rating.with_value(|r| r.is_some()) }>
                    <Rating rating=rating.get_value().expect("Rating is some") />
                </Show>

            </div>
            {display_actions}
        </div>
    }
}
