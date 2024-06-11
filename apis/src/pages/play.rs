use crate::{
    components::{
        atoms::history_button::{HistoryButton, HistoryNavigation},
        layouts::base_layout::{ControlsSignal, OrientationSignal},
        molecules::{
            analysis_and_download::AnalysisAndDownload, control_buttons::ControlButtons,
            game_info::GameInfo, user_with_rating::UserWithRating,
        },
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve},
            side_board::SideboardTabs,
        },
    },
    providers::{game_state::GameStateSignal, AuthContext},
};
use hive_lib::{Color, Position};
use leptos::*;

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let orientation_signal = expect_context::<OrientationSignal>();
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let white_and_black = create_read_slice(game_state.signal, |gs| (gs.white_id, gs.black_id));
    let show_buttons = move || {
        user().map_or(false, |user| {
            let (white_id, black_id) = white_and_black();
            Some(user.id) == black_id || Some(user.id) == white_id
        })
    };
    let player_is_black = create_memo(move |_| {
        user().map_or(false, |user| {
            let black_id = white_and_black().1;
            Some(user.id) == black_id
        })
    });
    let parent_container_style = move || {
        if orientation_signal.orientation_vertical.get() {
            "flex flex-col"
        } else {
            "grid grid-cols-board-xs sm:grid-cols-board-sm lg:grid-cols-board-lg xxl:grid-cols-board-xxl grid-rows-6 pr-2"
        }
    };
    let go_to_game = Callback::new(move |()| {
        let mut game_state = expect_context::<GameStateSignal>();
        if game_state.signal.get_untracked().is_last_turn() {
            game_state.view_game();
        }
    });
    let bottom_color = move || {
        if player_is_black() {
            Color::Black
        } else {
            Color::White
        }
    };
    let top_color = move || bottom_color().opposite_color();
    let controls_signal = expect_context::<ControlsSignal>();
    let show_controls =
        Signal::derive(move || !controls_signal.hidden.get() || game_state.is_finished()());

    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 select-none bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=orientation_signal.orientation_vertical
                fallback=move || {
                    view! {
                        <GameInfo extend_tw_classes="absolute pl-4 pt-2 bg-board-dawn dark:bg-board-twilight"/>
                        <Board/>
                        <div class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full">
                            <DisplayTimer placement=Placement::Top vertical=false/>
                            <SideboardTabs player_is_black=player_is_black/>
                            <DisplayTimer placement=Placement::Bottom vertical=false/>
                        </div>
                    }
                }
            >

                <div class="flex flex-col flex-grow h-full min-h-0">
                    <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                        <Show when=show_controls>
                            <div class="flex flex-row-reverse justify-between items-center shrink">
                                <AnalysisAndDownload/>
                                <Show when=show_buttons>
                                    <ControlButtons/>
                                </Show>
                            </div>
                        </Show>

                        <div class="flex justify-between ml-1 h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=top_color()/>
                            <DisplayTimer vertical=true placement=Placement::Top/>
                        </div>
                        <div class="flex gap-1 border-b-[1px] border-dashed border-gray-500 justify-between px-1 bg-inherit">
                            <UserWithRating
                                side=top_color()
                                is_tall=orientation_signal.orientation_vertical
                            />
                            <GameInfo/>
                        </div>

                    </div>
                    <Board overwrite_tw_classes="flex grow min-h-0"/>
                    <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                        <div class="flex gap-1 border-t-[1px] border-dashed border-gray-500">
                            <UserWithRating
                                side=bottom_color()
                                is_tall=orientation_signal.orientation_vertical
                            />
                        </div>
                        <div class="flex justify-between mb-2 ml-1 h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=bottom_color()/>
                            <DisplayTimer vertical=true placement=Placement::Bottom/>
                        </div>
                        <Show when=show_controls>
                            <div class="grid grid-cols-4 gap-8 pb-1">
                                <HistoryButton action=HistoryNavigation::First/>
                                <HistoryButton action=HistoryNavigation::Previous/>
                                <HistoryButton
                                    action=HistoryNavigation::Next
                                    post_action=go_to_game
                                />
                                <HistoryButton action=HistoryNavigation::MobileLast/>
                            </div>
                        </Show>

                    </div>
                </div>
            </Show>
        </div>
    }
}
