use crate::{
    common::with_class,
    components::atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator},
    responses::UserResponse,
};
use leptos::prelude::*;

#[component]
pub fn UserIdentity(
    user: UserResponse,
    #[prop(optional, default = true)] show_hover_ratings: bool,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional)] link_class: &'static str,
) -> impl IntoView {
    let hover_user = show_hover_ratings.then_some(user.clone());

    view! {
        <div class=with_class("flex items-center min-w-0", class.unwrap_or_default())>
            <StatusIndicator username=user.username.clone() deleted=user.deleted />
            <ProfileLink
                patreon=user.patreon
                bot=user.bot
                username=user.username
                deleted=user.deleted
                extend_tw_classes=link_class
                user_is_hoverable=hover_user.into()
            />
        </div>
    }
}
