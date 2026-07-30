use std::collections::HashMap;

use crate::{
    common::TournamentAction,
    components::layouts::base_layout::OrientationSignal,
    i18n::*,
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
    },
    responses::GameResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{GameId, ReadyUser};
use uuid::Uuid;

/// Berserk is declared per side, so the offer disappears once *this* player has
/// taken it — the opponent's choice is theirs to make separately.
fn already_berserked(game: &GameResponse, user: Option<Uuid>) -> bool {
    match user {
        Some(user) if game.white_player.uid == user => game.white_berserked,
        Some(user) if game.black_player.uid == user => game.black_berserked,
        _ => true,
    }
}

#[component]
pub fn Unstarted(
    game_id: Memo<GameId>,
    white_and_black_ids: Signal<(Option<Uuid>, Option<Uuid>)>,
    user_is_player: Signal<bool>,
    ready: RwSignal<HashMap<GameId, Vec<ReadyUser>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
    let auth_user =
        Signal::derive(move || auth_context.user.with(|a| a.as_ref().map(|u| u.user.uid)));
    let game_state = expect_context::<GameStateStore>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let game_response = game_state.game_response();
    let white = Memo::new(move |_| {
        (game_response.with(|game_response| {
            game_response.as_ref().map(|gr| {
                if gr.white_player.deleted {
                    t_string!(i18n, profile.deleted_user).to_string()
                } else {
                    gr.white_player.username.clone()
                }
            })
        }),)
    });
    let black = Memo::new(move |_| {
        (game_response.with(|game_response| {
            game_response.as_ref().map(|gr| {
                if gr.black_player.deleted {
                    t_string!(i18n, profile.deleted_user).to_string()
                } else {
                    gr.black_player.username.clone()
                }
            })
        }),)
    });
    let icon_for_color = move |id: Option<Uuid>| {
        let ready_map = ready.get();
        let current_game_id = game_id.get();

        let is_ready = if let Some(user_id) = id {
            ready_map
                .get(&current_game_id)
                .map(|users| {
                    users
                        .iter()
                        .any(|ready_user| ready_user.proposer_id == user_id)
                })
                .unwrap_or(false)
        } else {
            false
        };

        let icon = if is_ready {
            icondata_ai::AiCheckOutlined
        } else {
            icondata_io::IoCloseSharp
        };
        view! { <Icon icon attr:class="size-6" /> }
    };

    let start = move |_| {
        let api = api.get();
        api.tournament_game_start(game_id());
    };

    // Berserk buys arena scoring points with clock time, and the reduction has
    // to apply to the full starting clock — so it is only offered here, before
    // either side has moved, and only in an arena.
    let can_berserk = Signal::derive(move || {
        user_is_player.get()
            && game_response.with(|response| {
                response.as_ref().is_some_and(|game| {
                    game.tournament
                        .as_ref()
                        .is_some_and(|tournament| tournament.mode == "Arena")
                        && !game.finished
                        && game.turn == 0
                        && !already_berserked(game, auth_user.get())
                })
            })
    });
    let berserk = move |_| {
        api.get()
            .tournament(TournamentAction::Berserk(game_id.get_untracked()));
    };
    let style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex grow min-h-0 justify-center items-center h-full w-full"
        } else {
            "col-start-1 row-start-1 col-span-8 row-span-6"
        }
    };
    view! {
        <div class=style>
            <div class="flex flex-col gap-1 justify-center items-center h-full">
                <div class="flex gap-1 items-center">
                    <div class="flex gap-1 items-center">
                        {white} {move || icon_for_color(white_and_black_ids().0)}
                    </div>
                    "—"
                    <div class="flex gap-1 items-center">
                        {black} {move || icon_for_color(white_and_black_ids().1)}
                    </div>
                </div>
                <Show
                    when=user_is_player
                    fallback=move || {
                        view! { <div class="p-1">{t!(i18n, game.start_when.both_ready)}</div> }
                    }
                >

                    {t!(i18n, game.start_when.both_click)}
                    <button
                        on:click=start

                        class="ui-button ui-button-primary ui-button-md"
                    >
                        Ready
                    </button>
                    <Show when=can_berserk>
                        <button
                            on:click=berserk
                            title="Give up half your clock and all of your increment for extra arena points"
                            class="ui-button ui-button-danger ui-button-md"
                        >
                            "Berserk"
                        </button>
                    </Show>

                </Show>
            </div>
        </div>
    }
}
