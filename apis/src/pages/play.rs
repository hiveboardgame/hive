use crate::{
    components::{
        atoms::history_button::{HistoryButton, HistoryNavigation},
        molecules::{control_buttons::ControlButtons, user_with_rating::UserWithRating},
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve},
            side_board::SideboardTabs,
        },
    },
    functions::games::get::get_game_from_nanoid,
    providers::{auth_context::AuthContext, game_state::GameStateSignal},
    responses::user::UserResponse,
};
use hive_lib::{color::Color, position::Position};
use leptos::*;
use leptos_router::*;
use leptos_use::use_media_query;

#[derive(Params, PartialEq, Eq)]
struct PlayParams {
    nanoid: String,
}

#[derive(Clone)]
struct PlayersAndColors {
    top_player: StoredValue<UserResponse>,
    top_player_color: Color,
    bottom_player: StoredValue<UserResponse>,
    bottom_player_color: Color,
}

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let params = use_params::<PlayParams>();
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };

    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };

    let is_tall = use_media_query("(min-height: 100vw)");
    let game = create_blocking_resource(nanoid, move |_| get_game_from_nanoid(nanoid()));
    let nav_buttons_style =
        "flex place-items-center justify-center hover:bg-green-300 my-1 h-6 rounded-md border-cyan-500 border-2 drop-shadow-lg";

    // WARN: THIS IS A MOVE be very careful with what you do with signals!
    view! {
        <Transition>
            {move || {
                game()
                    .map(|data| match data {
                        Err(_) => view! { <pre>"Page not found"</pre> }.into_view(),
                        Ok(game) => {
                            let parent_container_style = if is_tall() {
                                "flex flex-col"
                            } else {
                                "grid grid-cols-board-xs sm:grid-cols-board-sm lg:grid-cols-board-lg xxl:grid-cols-board-xxl grid-rows-6 pr-1"
                            };
                            let game = store_value(game);
                            let mut game_state = expect_context::<GameStateSignal>();
                            game_state.set_game_id(store_value(nanoid()));
                            let white_player = store_value(game().white_player);
                            let black_player = store_value(game().black_player);
                            let state = game().create_state();
                            game_state.set_state(state, black_player().uid, white_player().uid);
                            game_state.join();
                            let show_buttons = move || {
                                user()
                                    .map_or(
                                        false,
                                        |user| {
                                            let game_state = game_state.signal.get();
                                            Some(user.id) == game_state.black_id
                                                || Some(user.id) == game_state.white_id
                                        },
                                    )
                            };
                            let player_is_black = create_memo(move |_| {
                                user()
                                    .map_or(
                                        false,
                                        |user| {
                                            let game_state = game_state.signal.get();
                                            Some(user.id) == game_state.black_id
                                        },
                                    )
                            });
                            let players = players_and_colors(
                                player_is_black(),
                                white_player,
                                black_player,
                            );
                            let go_to_game = Callback::new(move |()| {
                                let mut game_state = expect_context::<GameStateSignal>();
                                if game_state.signal.get_untracked().is_last_turn() {
                                    game_state.view_game();
                                }
                            });
                            view! {
                                <div class=move || {
                                    format!(
                                        "max-h-[100dvh] min-h-[100dvh] pt-10 {parent_container_style} {extend_tw_classes}",
                                    )
                                }>
                                    <Show
                                        when=is_tall
                                        fallback=move || {
                                            view! {
                                                <Board/>
                                                <div class="grid col-start-9 col-span-2 row-span-full grid-cols-2 grid-rows-6">
                                                    <DisplayTimer
                                                        side=players.top_player_color
                                                        placement=Placement::Top
                                                        player=players.top_player
                                                        // time_control=time_control()
                                                        vertical=false
                                                    />
                                                    <SideboardTabs player_is_black=player_is_black/>
                                                    <DisplayTimer
                                                        side=players.bottom_player_color
                                                        placement=Placement::Bottom
                                                        player=players.bottom_player
                                                        // time_control=time_control()
                                                        vertical=false
                                                    />
                                                </div>
                                            }
                                        }
                                    >

                                        <div class="flex flex-col flex-grow h-full min-h-0">
                                            <div class="flex flex-col shrink flex-grow">
                                                <div class="flex justify-between shrink">

                                                    <Show when=show_buttons>
                                                        <ControlButtons/>
                                                    </Show>
                                                </div>
                                                <div class="flex max-h-16 justify-between h-full">
                                                    <Reserve
                                                        alignment=Alignment::SingleRow
                                                        color=players.top_player_color
                                                    />
                                                // <DisplayTimer
                                                // side=players.top_player_color
                                                // player=players.top_player
                                                // time_control=time_control()
                                                // vertical=true
                                                // />
                                                </div>
                                                <div class="ml-2 flex gap-1">
                                                    <UserWithRating
                                                        player=players.top_player
                                                        side=players.top_player_color
                                                    />
                                                </div>

                                            </div>
                                            <Board overwrite_tw_classes="flex grow min-h-0"/>
                                            <div class="flex flex-col shrink flex-grow">
                                                <div class="ml-2 flex gap-1">
                                                    <UserWithRating
                                                        player=players.bottom_player
                                                        side=players.bottom_player_color
                                                    />
                                                </div>
                                                <div class="flex max-h-16 justify-between h-full">
                                                    <Reserve
                                                        alignment=Alignment::SingleRow
                                                        color=players.bottom_player_color
                                                    />

                                                // <DisplayTimer
                                                // side=players.bottom_player_color
                                                // player=players.bottom_player
                                                // time_control=time_control()
                                                // vertical=true
                                                // />
                                                </div>
                                                <div class="grid grid-cols-4 gap-8">
                                                    <HistoryButton
                                                        nav_buttons_style=nav_buttons_style
                                                        action=HistoryNavigation::First
                                                    />

                                                    <HistoryButton
                                                        nav_buttons_style=nav_buttons_style
                                                        action=HistoryNavigation::Previous
                                                    />

                                                    <HistoryButton
                                                        nav_buttons_style=nav_buttons_style
                                                        action=HistoryNavigation::Next
                                                        post_action=go_to_game
                                                    />

                                                    <HistoryButton
                                                        nav_buttons_style=nav_buttons_style
                                                        action=HistoryNavigation::MobileLast
                                                    />

                                                </div>
                                            </div>
                                        </div>
                                    </Show>
                                </div>
                            }
                                .into_view()
                        }
                    })
            }}

        </Transition>
    }
}

fn players_and_colors(
    player_is_black: bool,
    white_player: StoredValue<UserResponse>,
    black_player: StoredValue<UserResponse>,
) -> PlayersAndColors {
    if player_is_black {
        PlayersAndColors {
            top_player: white_player,
            top_player_color: Color::White,
            bottom_player: black_player,
            bottom_player_color: Color::Black,
        }
    } else {
        PlayersAndColors {
            top_player: black_player,
            top_player_color: Color::Black,
            bottom_player: white_player,
            bottom_player_color: Color::White,
        }
    }
}
