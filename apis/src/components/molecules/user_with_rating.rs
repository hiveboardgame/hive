use hive_lib::{color::Color, game_status::GameStatus};
use leptos::*;

use crate::{
    components::{
        atoms::profile_link::ProfileLink, molecules::rating_and_change::RatingAndChangeDynamic,
    },
    providers::game_state::GameStateSignal,
    responses::user::UserResponse,
};

#[component]
pub fn UserWithRating(
    player: StoredValue<UserResponse>,
    side: Color,
    #[prop(optional)] text_color: &'static str,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let is_finished = create_memo(move |_| {
        matches!(
            (game_state.signal)().state.game_status,
            GameStatus::Finished(_)
        )
    });
    view! {
        <ProfileLink username=player().username extend_tw_classes=text_color/>
        <Show
            when=is_finished
            fallback=move || {
                view! { <p class=format!("{text_color}")>{player().rating}</p> }
            }
        >

            <RatingAndChangeDynamic extend_tw_classes=text_color side=side/>
        </Show>
    }
}
