use crate::{
    common::{with_class, UserAction},
    components::{
        atoms::{
            direct_challenge_button::DirectChallengeButton,
            invite_button::InviteButton,
            kick_button::KickButton,
            rating::Rating,
            status_indicator::StatusIndicator,
            uninvite_button::UninviteButton,
        },
        molecules::user_identity::UserIdentity,
    },
    responses::UserResponse,
};
use leptos::{
    either::{Either, EitherOf4},
    prelude::*,
};
use shared_types::GameSpeed;

#[component]
pub fn UserRow(
    user: UserResponse,
    actions: Vec<UserAction>,
    #[prop(optional)] game_speed: Option<StoredValue<GameSpeed>>,
) -> impl IntoView {
    let username = StoredValue::new(user.username.clone());
    let user_id = StoredValue::new(user.uid);
    let rating = StoredValue::new(if let Some(speed) = game_speed {
        user.ratings.get(&speed.get_value()).cloned()
    } else {
        None
    });

    let (display_actions, select_callback): (_, Option<Callback<String>>) = {
        let user_id = user_id.get_value();
        let mut select_cb: Option<Callback<String>> = None;
        let display: Vec<_> = actions
            .into_iter()
            .filter_map(|action| match action {
                UserAction::Challenge => {
                    let opponent = username.get_value();
                    let disabled = user.bot;
                    // TODO: Allow users to direct challenge the bot once it can manage its own time
                    Some(EitherOf4::A(
                        view! { <DirectChallengeButton user_id opponent disabled /> },
                    ))
                }
                UserAction::Invite(tournament_id) => Some(EitherOf4::B(
                    view! { <InviteButton user_id tournament_id /> },
                )),
                UserAction::Uninvite(tournament_id) => Some(EitherOf4::C(
                    view! { <UninviteButton user_id tournament_id /> },
                )),
                UserAction::Kick(tournament) => Some(EitherOf4::D(
                    view! { <KickButton user_id tournament=*tournament /> },
                )),
                UserAction::Select(cb) => {
                    select_cb = Some(Callback::new(move |username: String| {
                        cb.run(Some(username))
                    }));
                    None
                }
                _ => None,
            })
            .collect();
        (display.into_iter().collect_view(), select_cb)
    };

    let row_class = with_class(
        "ui-dense-table-row",
        "flex p-1 items-center justify-between h-10 rounded",
    );

    if let Some(select_callback) = select_callback {
        let username_for_click = user.username.clone();
        let user_deleted = user.deleted;
        let user_bot = user.bot;

        Either::Left(view! {
            <button
                type="button"
                class=with_class(&row_class, "w-full cursor-pointer text-left")
                on:click=move |_| select_callback.run(username_for_click.clone())
            >
                <div class="flex items-center min-w-0">
                    <StatusIndicator username=username.get_value() deleted=user_deleted />
                    <span class="text-xs font-bold truncate max-w-[120px]">
                        {user.username.clone()} <Show when=move || user_bot>
                            <span class="ml-1 text-[80%]">"BOT"</span>
                        </Show>
                    </span>
                </div>
                <Show when=move || { rating.with_value(|r| r.is_some()) }>
                    <Rating rating=rating.get_value().expect("Rating is some") />
                </Show>
            </button>
        })
    } else {
        Either::Right(view! {
            <div class=row_class>
                <div class="flex flex-1 justify-between mr-2 min-w-0">
                    <UserIdentity user=user link_class="truncate max-w-[120px]" />
                    <Show when=move || { rating.with_value(|r| r.is_some()) }>
                        <Rating rating=rating.get_value().expect("Rating is some") />
                    </Show>
                </div>
                {display_actions}
            </div>
        })
    }
}
