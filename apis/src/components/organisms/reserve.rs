use crate::{
    components::molecules::{
        analysis_and_download::AnalysisAndDownload,
        annotation_toolbar::AnnotationToggle,
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
        analysis::AnalysisContext,
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        Config,
    },
};
use hive_lib::{Board, Color, GameStatus};
use leptos::prelude::*;

pub use crate::hiveground::ReserveLayout as Alignment;

pub const MOBILE_RESERVE_VIEWBOX: &str = "-42 -52 438 96";

#[component]
pub fn Reserve(
    // Analysis/history pass fixed colors; live play passes player-color signals.
    #[prop(into)] color: Signal<Color>,
    alignment: Alignment,
    interaction: HivegroundInteraction,
    history_board: Memo<Board>,
    #[prop(optional)] viewbox_str: Option<&'static str>,
) -> impl IntoView {
    let interaction = interaction.disable_stack_inspection();
    let analysis = use_context::<AnalysisContext>().is_some();
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let tile_opts = Signal::derive(move || config().tile);
    let paint = Memo::new(move |_| tile_opts.with(HivegroundPaint::new));
    let (default_viewbox_str, viewbox_styles) = match alignment {
        Alignment::SingleRow => (
            "-40 -55 450 100",
            "inline h-full max-h-[inherit] w-fit max-w-full",
        ),
        Alignment::DoubleRow => {
            if let Some(viewbox_str) = viewbox_str {
                (viewbox_str, "")
            } else {
                ("-32 -55 250 180", "p-1")
            }
        }
    };
    let viewbox_str = viewbox_str.unwrap_or(default_viewbox_str);
    let board_view = game_state.board_view();
    let move_info = game_state.move_info();
    let state = game_state.state();
    let last_turn = game_state.is_last_turn_as_signal();
    let game_response = game_state.game_response();
    let status = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map_or(GameStatus::NotStarted, |game| game.game_status.clone())
        })
    });
    let user_color = game_state.user_color_as_signal(auth_context.identity);
    let tournament = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|game| game.tournament.is_some())
        })
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
        let tournament = tournament.get();
        let board_view = board_view.get();
        let status = status.get();
        let viewing_past_turn = board_view.is_history() && !last_turn();
        let options = ReserveRenderOptions {
            reserve_color,
            alignment,
            interactivity: ReserveInteractivity {
                viewing_past_turn,
                status,
                user_color: user_color(),
                tournament,
                analysis,
            },
        };
        if board_view.is_history() && !analysis {
            history_board.with(|history_board| {
                state.with(|state| {
                    move_info.with(|move_info| {
                        build_reserve_render_model(state, history_board, move_info, options.clone())
                    })
                })
            })
        } else {
            state.with(|state| {
                move_info.with(|move_info| {
                    build_reserve_render_model(state, &state.board, move_info, options.clone())
                })
            })
        }
    });

    view! {
        <svg
            width="100%"
            height="100%"
            class=move || { format!("transition-none {viewbox_styles} {}", reserve_sepia_class()) }
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
    history_board: Memo<Board>,
) -> impl IntoView {
    let top_color = Signal::derive(move || player_color().opposite_color());
    let bottom_color = Signal::derive(player_color);

    view! {
        <Reserve color=top_color alignment=Alignment::DoubleRow interaction history_board />
        <div class="flex justify-center items-start">
            <Show when=show_buttons>
                <ControlButtons />
            </Show>
            <AnalysisAndDownload />
            <AnnotationToggle />
        </div>
        <Reserve color=bottom_color alignment=Alignment::DoubleRow interaction history_board />
    }
}
