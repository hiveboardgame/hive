use crate::functions::games::get::GetBatchFromOptions;
use crate::responses::GameResponse;
use codee::{binary::MsgpackSerdeCodec, string::Base64};
use cookie::SameSite;
use hive_lib::Color;
use leptos::{html, prelude::*};
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use serde::{Deserialize, Serialize};
use shared_types::{
    BatchInfo, GameProgress, GameSpeed, GamesQueryOptions, PlayerFilter, ResultType,
};

const GAMES_FILTER_COOKIE: &str = "games_filter";
const CONF_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterState {
    pub color: Option<Color>,
    pub result: Option<ResultType>,
    pub speeds: Vec<GameSpeed>,
    pub expansions: Option<bool>,
    pub rated: Option<bool>,
    pub exclude_bots: bool,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            color: None,
            result: None,
            speeds: GameSpeed::all_games(),
            expansions: None,    // Show both Base and MLP games by default
            rated: None,         // Show both rated and unrated
            exclude_bots: false, // Don't exclude bots by default
        }
    }
}

#[derive(Clone)]
pub struct GamesSearchContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub filters: RwSignal<FilterState>,
    pub pending: RwSignal<FilterState>,
    pub next_batch: ServerAction<GetBatchFromOptions>,
    pub is_first_batch: StoredValue<bool>,
    pub has_more: StoredValue<bool>,
    pub initial_batch_size: Signal<usize>,
    pub infinite_scroll_batch_size: Signal<usize>,
    pub games_container_ref: NodeRef<html::Div>,
    pub set_filter_cookie: WriteSignal<Option<FilterState>>,
    pub get_filter_cookie: Signal<Option<FilterState>>,
}

pub fn calculate_initial_batch_size(container_height: f64, container_width: f64) -> usize {
    // Container layout: 1 column on mobile, 2 columns on sm, 3 columns on lg
    let columns = if container_width < 640.0 {
        1 // mobile
    } else if container_width < 1024.0 {
        2 // sm to lg
    } else {
        3 // lg and above
    };

    // GameRow heights: 160px (h-40) on mobile, 224px (sm:h-56) on desktop
    let card_height = if container_width < 640.0 {
        160.0
    } else {
        224.0
    };
    let rows_with_buffer = (container_height / card_height).floor() as usize + 1;
    rows_with_buffer * columns
}

pub fn load_games(
    filters: FilterState,
    tab: GameProgress,
    username: String,
    batch_info: Option<BatchInfo>,
    action: ServerAction<GetBatchFromOptions>,
    batch_size: usize,
) {
    let player1 = Some(PlayerFilter {
        username,
        color: filters.color,
        result: filters.result,
    });

    let options = GamesQueryOptions {
        player1,
        player2: None,
        speeds: filters.speeds,
        current_batch: batch_info,
        batch_size,
        game_progress: tab,
        expansions: filters.expansions,
        rated: filters.rated,
        exclude_bots: filters.exclude_bots,
    };
    action.dispatch(GetBatchFromOptions { options });
}

pub fn games_filter_cookie() -> (
    Signal<Option<FilterState>>,
    WriteSignal<Option<FilterState>>,
) {
    let (cookie, set_cookie) = use_cookie_with_options::<FilterState, Base64<MsgpackSerdeCodec>>(
        GAMES_FILTER_COOKIE,
        UseCookieOptions::<FilterState, _, _>::default()
            .same_site(SameSite::Lax)
            .secure(true)
            .max_age(CONF_MAX_AGE)
            .default_value(Some(FilterState::default()))
            .path("/"),
    );
    (cookie, set_cookie)
}

pub fn provide_games_search_context(
    initial_batch_size: Signal<usize>,
    infinite_scroll_batch_size: Signal<usize>,
    games_container_ref: NodeRef<html::Div>,
) -> GamesSearchContext {
    let (cookie, set_cookie) = games_filter_cookie();
    let saved_filters = cookie.get_untracked().unwrap_or_default();

    let context = GamesSearchContext {
        filters: RwSignal::new(saved_filters.clone()),
        pending: RwSignal::new(saved_filters),
        games: RwSignal::new(Vec::new()),
        has_more: StoredValue::new(true),
        next_batch: ServerAction::new(),
        is_first_batch: StoredValue::new(true),
        initial_batch_size,
        infinite_scroll_batch_size,
        games_container_ref,
        set_filter_cookie: set_cookie,
        get_filter_cookie: cookie,
    };

    provide_context(context.clone());
    context
}
