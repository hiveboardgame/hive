mod auth_redirect;
mod challenge_action;
mod challenge_viewer;
mod chat_link;
mod client_message;
mod config_options;
mod cookie_options;
mod date_time;
mod download;
mod fitted_pagination;
mod focus;
mod game_action;
mod game_display;
mod game_reaction;
mod locale;
mod markdown;
mod move_info;
mod overlay_paint;
mod password;
mod piece_paint;
mod piece_type;
mod project_links;
mod rating_change_info;
mod route_path;
mod schedule_action;
mod server_result;
mod standings_criterion;
mod standings_score;
#[cfg(feature = "ssr")]
mod static_assets;
mod svg_assets;
mod svg_pos;
mod time_control;
mod time_signals;
mod tournament_action;
mod tournament_admission;
mod tournament_display;
mod tournament_format_label;
mod tournament_pairing_label;
mod tournament_standings;
mod ui_error;
mod ui_utils;
mod user_action;
#[cfg(feature = "ssr")]
pub(crate) use auth_redirect::safe_return_path;
pub(crate) use auth_redirect::{
    auth_page_url,
    current_page_path,
    login_redirect_url,
    use_return_path,
};
pub use challenge_action::ChallengeAction;
pub use challenge_viewer::{
    challenge_action_flags,
    challenge_displayed_color,
    challenge_displayed_player,
    challenge_is_viewable,
    challenge_viewer_role,
    ChallengeActionFlags,
    ChallengeViewerRole,
};
pub use chat_link::{chat_link_href, external_link_destination, validated_link_url};
pub use client_message::{ChatSendRequest, ClientRequest, SubscriptionAttempt, TvInterest};
pub use config_options::{CurrentConfirm, MoveConfirm, TileDesign, TileDots, TileRotation};
pub(crate) use cookie_options::SECURE_COOKIES;
#[cfg(feature = "ssr")]
pub(crate) use date_time::format_utc_datetime;
pub(crate) use date_time::{format_local_datetime, format_tournament_datetime};
pub(crate) use download::download_blob;
pub(crate) use fitted_pagination::use_fitted_pagination;
pub(crate) use focus::focus_after_render;
pub use game_action::GameAction;
pub use game_display::{
    color_choice_label,
    conclusion_label,
    display_username,
    format_clock_time_remaining,
    format_game_rating,
    format_game_result,
    game_result_label,
    game_speed_label,
    game_speed_label_for_locale,
    game_tournament_link,
    result_marker_label,
    tournament_game_result_label,
    TournamentLink,
};
pub use game_reaction::GameReaction;
pub use locale::parse_locale;
#[cfg(feature = "ssr")]
pub(crate) use locale::request_locale;
pub use markdown::markdown_to_html;
pub use move_info::MoveInfo;
pub use overlay_paint::OverlayPaint;
pub(crate) use password::{
    password_error_message,
    password_requirements_message,
    validate_password,
};
pub use piece_paint::{resolve_piece_paint, BugHref, DotsHref, PiecePaint, ShadowHref, TileHref};
pub use piece_type::PieceType;
pub use project_links::{
    CONTRIBUTING_URL,
    GEN42_URL,
    GITHUB_ISSUES_URL,
    GITHUB_URL,
    HIVEGAME_DISCORD_URL,
    KOFI_URL,
    LICENSE_URL,
    PATREON_URL,
    WORLD_HIVE_DISCORD_URL,
};
pub use rating_change_info::RatingChangeInfo;
pub(crate) use route_path::{analysis_url, tournament_path_matches};
pub use schedule_action::{parse_schedule_slot_fragment, schedule_slot_fragment, ScheduleAction};
pub use server_result::{
    ChallengeUpdate,
    ChatSendError,
    ExternalServerError,
    GameActionResponse,
    GameUpdate,
    LobbySnapshot,
    ScheduleUpdate,
    ServerMessage,
    ServerResult,
    SubscriptionError,
    TournamentUpdate,
    UserSettingsUpdate,
    UserStatus,
    UserUpdate,
};
pub use standings_criterion::StandingsCriterionChoice;
pub use standings_score::{
    game_point_presentation,
    half_point_text,
    primary_score_presentation,
    primary_value_text,
    round_robin_criterion_presentation,
    standings_count,
    swiss_criterion_presentation,
    value_text as standings_value_text,
    ScorePresentation,
    StandingsCount,
};
pub use svg_pos::{position_from_svg, SvgPos};
pub(crate) use time_control::{
    format_compact_duration,
    format_time_control,
    format_tournament_clock,
    TournamentClockStyle,
};
pub use time_signals::{TimeParams, TimeParamsStoreFields};
pub use tournament_action::{
    TournamentAction,
    TournamentAdjudicationIntent,
    TournamentCloseoutIntent,
    TournamentStartIntent,
};
pub(crate) use tournament_admission::tournament_admission_viewer;
pub use tournament_format_label::tournament_format_label;
pub(crate) use tournament_standings::display_standing_rows;
pub use ui_error::{localized_error_message, localized_error_message_for_locale, UiError};
pub use ui_utils::{render_text_prop, with_class};
pub use user_action::UserAction;

pub(crate) use tournament_display::{
    arena_player_rating,
    elimination_color_sequence,
    result_marker_color,
    rounded_percentage,
    tournament_completion_order,
    tournament_game_number,
    tournament_player_name,
    tournament_slot_opponent,
    unfinished_tournament_game_label,
};
pub use tournament_display::{parse_tournament_slot_fragment, tournament_slot_fragment};

pub use tournament_pairing_label::{TournamentPairingLabel, TournamentStageLabel};

pub use svg_assets::{
    inline_tile_sprite,
    namespace_svg,
    THREE_D_SHADOW_PATH,
    THREE_D_SHADOW_PREFIX,
};

#[cfg(feature = "ssr")]
pub use static_assets::serve_precompressed;
