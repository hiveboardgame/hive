use crate::{
    components::{
        atoms::history_button::{HistoryButton, HistoryNavigation},
        molecules::{
            analysis_and_download::AnalysisAndDownload, control_buttons::ControlButtons,
            game_info::GameInfo, user_with_rating::UserWithRating,
        },
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            dropdowns::ChatDropdown,
            reserve::{Alignment, Reserve},
            side_board::SideboardTabs,
        },
    },
    providers::{game_state::GameStateSignal, AuthContext},
};
use hive_lib::{Color, Position};
use leptos::*;
use leptos_use::use_media_query;
use shared_types::SimpleDestination;

#[derive(Clone)]
pub struct TargetStack(pub RwSignal<Option<Position>>);

#[component]
pub fn Play(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    provide_context(TargetStack(RwSignal::new(None)));
    let mobile_chat = create_rw_signal(false);
    provide_context(mobile_chat);
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let show_buttons = move || {
        user().map_or(false, |user| {
            let game_state = game_state.signal.get();
            Some(user.id) == game_state.black_id || Some(user.id) == game_state.white_id
        })
    };
    let player_is_black = create_memo(move |_| {
        user().map_or(false, |user| {
            let game_state = game_state.signal.get();
            Some(user.id) == game_state.black_id
        })
    });

    let is_tall = use_media_query("(min-height: 100vw)");
    let is_tall_or_chat = Signal::derive(move || is_tall() || mobile_chat());
    let parent_container_style = move || {
        if is_tall_or_chat() {
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

    view! {
        <div class=move || {
            format!(
                "max-h-[100dvh] min-h-[100dvh] pt-10 select-none bg-board-dawn dark:bg-board-twilight {} {extend_tw_classes}",
                parent_container_style(),
            )
        }>
            <Show
                when=is_tall_or_chat
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
                        <div class="flex flex-row-reverse justify-between items-center shrink">
                            <ChatDropdown destination=SimpleDestination::Game/>
                            <AnalysisAndDownload/>
                            <Show when=show_buttons>
                                <ControlButtons/>
                            </Show>
                        </div>
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=top_color()/>
                            <DisplayTimer vertical=true placement=Placement::Top/>
                        </div>
                        <div class="flex gap-1 border-b-[1px] border-dashed border-gray-500 justify-between px-1 bg-inherit">
                            <UserWithRating side=top_color() is_tall=is_tall_or_chat/>
                            <GameInfo/>
                        </div>

                    </div>
                    <Board overwrite_tw_classes="flex grow min-h-0"/>
                    <div class="flex flex-col flex-grow shrink bg-board-dawn dark:bg-reserve-twilight">
                        <div class="flex gap-1 border-t-[1px] border-dashed border-gray-500">
                            <UserWithRating side=bottom_color() is_tall=is_tall_or_chat/>
                        </div>
                        <div class="flex justify-between h-full max-h-16">
                            <Reserve alignment=Alignment::SingleRow color=bottom_color()/>
                            <DisplayTimer vertical=true placement=Placement::Bottom/>
                        </div>
                        <div class="grid grid-cols-4 gap-8 pb-1">
                            <HistoryButton action=HistoryNavigation::First/>
                            <HistoryButton action=HistoryNavigation::Previous/>
                            <HistoryButton action=HistoryNavigation::Next post_action=go_to_game/>
                            <HistoryButton action=HistoryNavigation::MobileLast/>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
