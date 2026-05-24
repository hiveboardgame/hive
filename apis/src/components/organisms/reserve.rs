use crate::{
    components::molecules::{
        analysis_and_download::AnalysisAndDownload,
        control_buttons::ControlButtons,
        hiveground_stacks::HivegroundStacks,
    },
    hiveground::{
        build_reserve_render_model,
        HivegroundInteraction,
        HivegroundPaint,
        ReserveInteractivity,
        ReserveRenderOptions,
    },
    providers::{
        analysis::AnalysisSignal,
        game_state::{GameStateSignal, View},
        AuthContext,
        Config,
    },
};
use hive_lib::{Color, GameStatus, State};
use leptos::prelude::*;

pub use crate::hiveground::ReserveLayout as Alignment;

#[component]
pub fn Reserve(
    // Analysis/history pass fixed colors; live play passes player-color signals.
    #[prop(into)] color: Signal<Color>,
    alignment: Alignment,
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] viewbox_str: Option<&'static str>,
) -> impl IntoView {
    let interaction = interaction.disable_stack_inspection();
    let analysis = use_context::<AnalysisSignal>().is_some();
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let (viewbox_str, viewbox_styles) = match alignment {
        Alignment::SingleRow => ("-40 -55 450 100", "inline max-h-[inherit] h-full w-fit"),
        Alignment::DoubleRow => {
            if let Some(viewbox_str) = viewbox_str {
                (viewbox_str, "")
            } else {
                ("-32 -55 250 180", "p-1")
            }
        }
    };
    // TODO: Should be a Store, this is hacky
    let board_view = create_read_slice(game_state.signal, |gs| gs.view.clone());
    let move_info = create_read_slice(game_state.signal, |gs| gs.move_info.clone());
    let state = create_read_slice(game_state.signal, |gs| gs.state.clone());
    let last_turn = game_state.is_last_turn_as_signal();
    let status = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map_or(GameStatus::NotStarted, |g| g.game_status.clone())
    });
    let user_id = Signal::derive(move || {
        auth_context
            .user
            .with_untracked(|a| a.as_ref().map(|user| user.id))
    });
    let user_color = game_state.user_color_as_signal(user_id);
    let tournament = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .is_some_and(|gr| gr.tournament.is_some())
    });
    let reserve_sepia_class = move || {
        if !analysis {
            return "";
        }
        if color() != state.with(|s| s.turn_color) {
            return "sepia-[.75]";
        }
        ""
    };
    let stacked_pieces = Memo::new(move |_| {
        let reserve_color = color();
        let tournament = tournament();
        let board_view = board_view();
        let status = status();
        let viewing_past_turn = board_view == View::History && !last_turn();

        move_info.with(|move_info| {
            state.with(|state| {
                let reserve_board = match board_view {
                    View::Game => state.board.clone(),
                    View::History => {
                        history_state.with(|history_state| history_state.board.clone())
                    }
                };
                build_reserve_render_model(
                    state,
                    &reserve_board,
                    move_info,
                    ReserveRenderOptions {
                        reserve_color,
                        alignment,
                        interactivity: ReserveInteractivity {
                            viewing_past_turn,
                            status,
                            user_color: user_color(),
                            tournament,
                            analysis,
                        },
                    },
                )
            })
        })
    });

    view! {
        <svg
            width="100%"
            height="100%"
            class=move || {
                format!(
                    "duration-300 {viewbox_styles} {extend_tw_classes} {}",
                    reserve_sepia_class(),
                )
            }
            viewBox=viewbox_str
            xmlns="http://www.w3.org/2000/svg"
        >
            <HivegroundStacks model=stacked_pieces paint interaction />
        </svg>
    }
}

#[component]
pub fn ReserveContent(
    player_color: Memo<Color>,
    show_buttons: Signal<bool>,
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
) -> impl IntoView {
    let top_color = Signal::derive(move || player_color().opposite_color());
    let bottom_color = Signal::derive(player_color);

    view! {
        <Reserve color=top_color alignment=Alignment::DoubleRow interaction history_state />
        <div class="flex flex-row-reverse justify-center items-center">
            <AnalysisAndDownload />
            <Show when=show_buttons>
                <ControlButtons />
            </Show>
        </div>
        <Reserve color=bottom_color alignment=Alignment::DoubleRow interaction history_state />
    }
}
