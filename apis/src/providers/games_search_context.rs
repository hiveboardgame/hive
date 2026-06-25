use crate::{functions::games::get::GetBatchFromOptions, responses::GameResponse};
use codee::{binary::MsgpackSerdeCodec, string::Base64};
use cookie::SameSite;
use hive_lib::Color;
use leptos::{html, prelude::*};
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use serde::{Deserialize, Serialize};
use shared_types::{
    BatchToken,
    GameProgress,
    GameSort,
    GameSortKey,
    GameSpeed,
    GamesQueryOptions,
    ResultFilter,
};

const GAMES_FILTER_COOKIE: &str = "games_filter";
const CONF_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResultType {
    Win,
    Loss,
    Draw,
}

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
            expansions: None,
            rated: None,
            exclude_bots: false,
        }
    }
}

pub fn initial_profile_filters_for_tab(
    tab: GameProgress,
    saved_filters: Option<FilterState>,
) -> FilterState {
    if tab == GameProgress::Finished {
        saved_filters.unwrap_or_default()
    } else {
        FilterState::default()
    }
}

pub fn searchable_profile_filters_for_tab(filters: FilterState, tab: GameProgress) -> FilterState {
    if tab == GameProgress::Finished {
        filters
    } else {
        FilterState {
            result: None,
            ..filters
        }
    }
}

#[derive(Clone)]
pub struct GamesSearchContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub filters: RwSignal<FilterState>,
    pub pending: RwSignal<FilterState>,
    pub next_batch: ServerAction<GetBatchFromOptions>,
    pub next_batch_token: RwSignal<Option<BatchToken>>,
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
        1
    } else if container_width < 1024.0 {
        2
    } else {
        3
    };

    let card_height = if container_width < 640.0 {
        160.0
    } else {
        224.0
    };
    let rows_with_buffer = (container_height / card_height).floor() as usize + 1;
    rows_with_buffer * columns
}

/// Snap a UI-derived batch size to the closest allowed value for the backend.
fn nearest_allowed_batch_size(requested: usize) -> usize {
    use shared_types::ALLOWED_BATCH_SIZES;
    *ALLOWED_BATCH_SIZES
        .iter()
        .min_by_key(|&&allowed| allowed.abs_diff(requested))
        .expect("ALLOWED_BATCH_SIZES is non-empty")
}

/// Slot the profile user occupies for the new query builder.
/// White-colored filter → slot 1; black → slot 2; no color → slot 1 (any color).
fn profile_slot(color: Option<Color>) -> u8 {
    match color {
        Some(Color::Black) => 2,
        _ => 1,
    }
}

fn result_to_filter(result: Option<ResultType>, slot: u8) -> ResultFilter {
    match result {
        None => ResultFilter::Any,
        Some(ResultType::Win) => ResultFilter::PlayerWins(slot),
        Some(ResultType::Loss) => ResultFilter::PlayerLoses(slot),
        Some(ResultType::Draw) => ResultFilter::Draw,
    }
}

pub fn build_profile_query_options(
    filters: &FilterState,
    tab: GameProgress,
    username: &str,
    batch_token: Option<BatchToken>,
    batch_size: usize,
) -> GamesQueryOptions {
    let slot = profile_slot(filters.color);
    let (player1, player2) = match slot {
        2 => (None, Some(username.to_string())),
        _ => (Some(username.to_string()), None),
    };

    let sort = GameSort {
        key: GameSortKey::Date,
        ascending: false,
    };
    let result_filter = if tab == GameProgress::Finished {
        result_to_filter(filters.result, slot)
    } else {
        ResultFilter::Any
    };

    GamesQueryOptions {
        player1,
        player2,
        fixed_colors: filters.color.is_some(),
        exclude_bots: filters.exclude_bots,
        only_tournament: false,
        rated: filters.rated,
        expansions: filters.expansions,
        speeds: filters.speeds.clone(),
        rating_min: None,
        rating_max: None,
        turn_min: None,
        turn_max: None,
        date_start: None,
        date_end: None,
        result_filter,
        batch_token,
        batch_size: nearest_allowed_batch_size(batch_size.max(1)),
        page: 1,
        sort,
        game_progress: tab,
        include_total: false,
        position_hash: None,
    }
}

pub fn load_games(
    filters: FilterState,
    tab: GameProgress,
    username: String,
    batch_token: Option<BatchToken>,
    action: ServerAction<GetBatchFromOptions>,
    batch_size: usize,
) {
    let options = build_profile_query_options(&filters, tab, &username, batch_token, batch_size);
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
            .path("/"),
    );
    (cookie, set_cookie)
}

pub fn provide_games_search_context(
    initial_batch_size: Signal<usize>,
    infinite_scroll_batch_size: Signal<usize>,
    games_container_ref: NodeRef<html::Div>,
    current_tab: GameProgress,
) -> GamesSearchContext {
    let (cookie, set_cookie) = games_filter_cookie();
    let saved_filters = initial_profile_filters_for_tab(current_tab, cookie.get_untracked());

    let context = GamesSearchContext {
        filters: RwSignal::new(saved_filters.clone()),
        pending: RwSignal::new(saved_filters),
        games: RwSignal::new(Vec::new()),
        has_more: StoredValue::new(true),
        next_batch: ServerAction::new(),
        next_batch_token: RwSignal::new(None),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_default_only_initializes_finished_profile_filters() {
        let saved_filters = FilterState {
            color: Some(Color::Black),
            result: Some(ResultType::Win),
            speeds: vec![GameSpeed::Blitz],
            expansions: Some(true),
            rated: Some(false),
            exclude_bots: true,
        };

        assert_eq!(
            initial_profile_filters_for_tab(GameProgress::Finished, Some(saved_filters.clone())),
            saved_filters
        );
        assert_eq!(
            initial_profile_filters_for_tab(GameProgress::Playing, Some(saved_filters.clone())),
            FilterState::default()
        );
        assert_eq!(
            initial_profile_filters_for_tab(GameProgress::Unstarted, Some(saved_filters)),
            FilterState::default()
        );
    }

    #[test]
    fn non_finished_search_keeps_visible_filters_without_result_filter() {
        let filters = FilterState {
            color: Some(Color::Black),
            result: Some(ResultType::Loss),
            speeds: vec![GameSpeed::Blitz],
            expansions: Some(true),
            rated: Some(false),
            exclude_bots: true,
        };

        let searchable = searchable_profile_filters_for_tab(filters.clone(), GameProgress::Playing);

        assert_eq!(searchable.color, filters.color);
        assert_eq!(searchable.result, None);
        assert_eq!(searchable.speeds, filters.speeds);
        assert_eq!(searchable.expansions, filters.expansions);
        assert_eq!(searchable.rated, filters.rated);
        assert_eq!(searchable.exclude_bots, filters.exclude_bots);

        let query =
            build_profile_query_options(&filters, GameProgress::Playing, "player", None, 10);

        assert_eq!(query.player2, Some("player".to_string()));
        assert!(query.fixed_colors);
        assert!(query.exclude_bots);
        assert_eq!(query.rated, Some(false));
        assert_eq!(query.expansions, Some(true));
        assert_eq!(query.speeds, vec![GameSpeed::Blitz]);
        assert_eq!(query.result_filter, ResultFilter::Any);
    }
}
