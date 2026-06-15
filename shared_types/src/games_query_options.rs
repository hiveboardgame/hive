use crate::game_speed::GameSpeed;
use chrono::{DateTime, Utc};
use hive_lib::Color;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, PartialEq, Copy, Debug, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameProgress {
    Unstarted,
    #[default]
    Playing,
    Finished,
    All,
}

impl std::str::FromStr for GameProgress {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unstarted" => Ok(GameProgress::Unstarted),
            "Playing" => Ok(GameProgress::Playing),
            "Finished" => Ok(GameProgress::Finished),
            "All" => Ok(GameProgress::All),
            _ => Err(anyhow::anyhow!("Invalid GameProgress string")),
        }
    }
}

impl std::fmt::Display for GameProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view = match self {
            GameProgress::Unstarted => "Unstarted",
            GameProgress::Playing => "Playing",
            GameProgress::Finished => "Finished",
            GameProgress::All => "All",
        };
        write!(f, "{view}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResultFilter {
    Any,
    ColorWins(Color),
    PlayerWins(u8),
    PlayerLoses(u8),
    Draw,
    NotDraw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameSortKey {
    Date,
    Turns,
    RatingAvg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameSort {
    pub key: GameSortKey,
    pub ascending: bool,
}

impl Default for GameSort {
    fn default() -> Self {
        Self {
            key: GameSortKey::Date,
            ascending: false,
        }
    }
}

impl GameSort {
    pub fn is_desc(&self) -> bool {
        !self.ascending
    }
}

impl std::str::FromStr for GameSortKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Date" => Ok(GameSortKey::Date),
            "Turns" => Ok(GameSortKey::Turns),
            "RatingAvg" => Ok(GameSortKey::RatingAvg),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for ResultFilter {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "any" => Ok(ResultFilter::Any),
            "white_wins" => Ok(ResultFilter::ColorWins(Color::White)),
            "black_wins" => Ok(ResultFilter::ColorWins(Color::Black)),
            "player1_wins" => Ok(ResultFilter::PlayerWins(1)),
            "player2_wins" => Ok(ResultFilter::PlayerWins(2)),
            "player1_loses" => Ok(ResultFilter::PlayerLoses(1)),
            "player2_loses" => Ok(ResultFilter::PlayerLoses(2)),
            "draw" => Ok(ResultFilter::Draw),
            "not_draw" => Ok(ResultFilter::NotDraw),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ResultFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ResultFilter::Any => "any",
            ResultFilter::ColorWins(Color::White) => "white_wins",
            ResultFilter::ColorWins(Color::Black) => "black_wins",
            ResultFilter::PlayerWins(1) => "player1_wins",
            ResultFilter::PlayerWins(2) => "player2_wins",
            ResultFilter::PlayerLoses(1) => "player1_loses",
            ResultFilter::PlayerLoses(2) => "player2_loses",
            ResultFilter::Draw => "draw",
            ResultFilter::NotDraw => "not_draw",
            ResultFilter::PlayerWins(_) | ResultFilter::PlayerLoses(_) => "any",
        };
        write!(f, "{value}")
    }
}

impl std::fmt::Display for GameSortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            GameSortKey::Date => "Date",
            GameSortKey::Turns => "Turns",
            GameSortKey::RatingAvg => "RatingAvg",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum SortValue {
    UpdatedAt(DateTime<Utc>),
    Turns(i32),
    RatingAvg(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchToken {
    pub sort: GameSort,
    pub primary_value: SortValue,
    pub updated_at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GamesQueryOptions {
    pub player1: Option<String>,
    pub player2: Option<String>,
    pub fixed_colors: bool,
    pub exclude_bots: bool,
    pub only_tournament: bool,
    pub rated: Option<bool>,
    pub expansions: Option<bool>,
    pub speeds: Vec<GameSpeed>,
    pub rating_min: Option<i32>,
    pub rating_max: Option<i32>,
    pub turn_min: Option<i32>,
    pub turn_max: Option<i32>,
    pub date_start: Option<DateTime<Utc>>,
    pub date_end: Option<DateTime<Utc>>,
    pub result_filter: ResultFilter,
    pub batch_token: Option<BatchToken>,
    pub batch_size: usize,
    pub page: usize,
    pub sort: GameSort,
    pub game_progress: GameProgress,
    pub include_total: bool,
    /// Canonical position hash (`game_hashes.hash`) to restrict results to games that passed
    /// through that position. Set by the opening explorer's "Search this position" link.
    pub position_hash: Option<i64>,
}

impl Default for GamesQueryOptions {
    fn default() -> Self {
        Self {
            player1: None,
            player2: None,
            fixed_colors: false,
            exclude_bots: false,
            only_tournament: false,
            rated: None,
            expansions: None,
            speeds: GameSpeed::all_games(),
            rating_min: None,
            rating_max: None,
            turn_min: None,
            turn_max: None,
            date_start: None,
            date_end: None,
            result_filter: ResultFilter::Any,
            batch_token: None,
            batch_size: 10,
            page: 1,
            sort: GameSort::default(),
            game_progress: GameProgress::All,
            include_total: true,
            position_hash: None,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum GameQueryValidationError {
    #[error("player filters must not reference the same normalized username")]
    DuplicatePlayers,
    #[error("result filter for player {slot} requires that player to be provided")]
    MissingPlayerForResult { slot: u8 },
    #[error("rating filters must be between 0 and 3000")]
    RatingOutOfRange,
    #[error("rating range min must not exceed max")]
    RatingBoundsInvalid,
    #[error("rating filters require rated to be true")]
    RatingFiltersRequireRated,
    #[error("turn range min must not exceed max")]
    TurnBoundsInvalid,
    #[error("date start must be before or equal to date end")]
    DateBoundsInvalid,
    #[error("batch token sort does not match requested sort")]
    BatchTokenSortMismatch,
    #[error("batch size must be 10, 25, or 50")]
    BatchSizeInvalid,
    #[error("page must be at least 1 and at most 10000")]
    PageOutOfRange,
    #[error("sort key is only valid for finished games")]
    SortKeyRequiresFinished,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum GamesQueryParseError {
    #[error("invalid bool for {0}")]
    InvalidBool(&'static str),
    #[error("invalid option bool for {0}")]
    InvalidOptionBool(&'static str),
    #[error("invalid speed {0}")]
    InvalidSpeed(String),
    #[error("invalid rating filter {field}: {error}")]
    InvalidRating { field: &'static str, error: String },
    #[error("invalid turn filter {field}: {error}")]
    InvalidTurn { field: &'static str, error: String },
    #[error("invalid date {field}: {error}")]
    InvalidDate { field: &'static str, error: String },
    #[error("invalid result filter")]
    InvalidResultFilter,
    #[error("invalid sort key")]
    InvalidSortKey,
    #[error("invalid sort direction")]
    InvalidSortDirection,
    #[error("invalid batch size")]
    InvalidBatchSize,
    #[error("invalid page")]
    InvalidPage,
    #[error("validation failed: {0:?}")]
    ValidationFailedList(Vec<GameQueryValidationError>),
    #[error("parse error: {0}")]
    Generic(String),
}

/// Allowed page sizes for archive game search.
pub const ALLOWED_BATCH_SIZES: [usize; 3] = [10, 25, 50];

impl GamesQueryOptions {
    pub fn validate_all(mut self) -> Result<Self, Vec<GameQueryValidationError>> {
        let mut errors = Vec::new();

        if !ALLOWED_BATCH_SIZES.contains(&self.batch_size) {
            errors.push(GameQueryValidationError::BatchSizeInvalid);
        }

        if self.page == 0 || self.page > 10000 {
            errors.push(GameQueryValidationError::PageOutOfRange);
        }

        let player1 = self.normalize_player(self.player1.as_ref());
        let player2 = self.normalize_player(self.player2.as_ref());

        if let (Some(p1), Some(p2)) = (&player1, &player2) {
            if p1 == p2 {
                errors.push(GameQueryValidationError::DuplicatePlayers);
            }
        }

        if matches!(
            self.result_filter,
            ResultFilter::PlayerWins(1) | ResultFilter::PlayerLoses(1)
        ) && player1.is_none()
        {
            errors.push(GameQueryValidationError::MissingPlayerForResult { slot: 1 });
        }
        if matches!(
            self.result_filter,
            ResultFilter::PlayerWins(2) | ResultFilter::PlayerLoses(2)
        ) && player2.is_none()
            && player1.is_none()
        {
            errors.push(GameQueryValidationError::MissingPlayerForResult { slot: 2 });
        }

        if self.game_progress != GameProgress::Finished
            && matches!(self.sort.key, GameSortKey::Turns | GameSortKey::RatingAvg)
        {
            errors.push(GameQueryValidationError::SortKeyRequiresFinished);
        }

        let mut speeds = self.speeds.clone();
        speeds.sort();
        speeds.dedup();

        let mut rated = self.rated;

        if self.uses_rating_filter() {
            if rated != Some(true) {
                errors.push(GameQueryValidationError::RatingFiltersRequireRated);
            }
            rated = Some(true);

            let valid_bounds = self.rating_min.is_none_or(|min| (0..=3000).contains(&min))
                && self.rating_max.is_none_or(|max| (0..=3000).contains(&max));
            if !valid_bounds {
                errors.push(GameQueryValidationError::RatingOutOfRange);
            }
            if let (Some(min), Some(max)) = (self.rating_min, self.rating_max) {
                if min > max {
                    errors.push(GameQueryValidationError::RatingBoundsInvalid);
                }
            }
        }

        if let (Some(min), Some(max)) = (self.turn_min, self.turn_max) {
            if min > max {
                errors.push(GameQueryValidationError::TurnBoundsInvalid);
            }
        }

        if let (Some(start), Some(end)) = (self.date_start, self.date_end) {
            if start > end {
                errors.push(GameQueryValidationError::DateBoundsInvalid);
            }
        }

        if let Some(token) = &self.batch_token {
            if token.sort != self.sort {
                errors.push(GameQueryValidationError::BatchTokenSortMismatch);
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        self.player1 = player1;
        self.player2 = player2;
        self.rated = rated;
        self.speeds = speeds;

        Ok(self)
    }

    fn normalize_player(&self, player: Option<&String>) -> Option<String> {
        player.and_then(|p| {
            let original = p.trim();
            if original.is_empty() {
                return None;
            }
            Some(original.to_lowercase().to_string())
        })
    }

    fn uses_rating_filter(&self) -> bool {
        self.rating_min.is_some() || self.rating_max.is_some()
    }
}

impl std::fmt::Display for GamesQueryOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        let mut push = |key: &str, value: String| {
            if !value.is_empty() {
                parts.push(format!("{key}={value}"));
            }
        };

        if let Some(p) = &self.player1 {
            push("player1", p.clone());
        }
        if let Some(p) = &self.player2 {
            push("player2", p.clone());
        }
        if self.fixed_colors {
            push("fixed_colors", "true".into());
        }
        if self.exclude_bots {
            push("exclude_bots", "true".into());
        }
        if self.only_tournament {
            push("only_tournament", "true".into());
        }
        match self.rated {
            Some(rated) => push("rated", rated.to_string()),
            None => push("rated", "any".into()),
        }
        match self.expansions {
            Some(exp) => push("expansions", exp.to_string()),
            None => push("expansions", "any".into()),
        }
        if !self.speeds.is_empty() {
            let mut speeds = self.speeds.clone();
            speeds.sort();
            let joined = speeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            push("speeds", joined);
        }
        if let Some(min) = self.rating_min {
            push("rating_min", min.to_string());
        }
        if let Some(max) = self.rating_max {
            push("rating_max", max.to_string());
        }
        if let Some(min) = self.turn_min {
            push("turn_min", min.to_string());
        }
        if let Some(max) = self.turn_max {
            push("turn_max", max.to_string());
        }
        if let Some(start) = self.date_start {
            push("date_start", start.format("%Y-%m-%d").to_string());
        }
        if let Some(end) = self.date_end {
            push("date_end", end.format("%Y-%m-%d").to_string());
        }

        push("result_filter", self.result_filter.to_string());

        push("sort_key", self.sort.key.to_string());
        push("sort_asc", self.sort.ascending.to_string());

        if let Some(hash) = self.position_hash {
            push("position_hash", hash.to_string());
        }

        if self.page > 1 {
            push("page", self.page.to_string());
        }
        if self.batch_size != 10 {
            push("batch_size", self.batch_size.to_string());
        }

        if parts.is_empty() {
            write!(f, "")
        } else {
            write!(f, "?{}", parts.join("&"))
        }
    }
}

impl std::str::FromStr for GamesQueryOptions {
    type Err = GamesQueryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GamesQueryOptions::parse_with_errors(s).map_err(|errs| {
            let mut validation_errs = Vec::new();
            let mut parse_errs = Vec::new();

            for err in errs {
                match err {
                    GamesQueryParseError::ValidationFailedList(v) => {
                        validation_errs.extend(v);
                    }
                    other => parse_errs.push(other),
                }
            }

            if parse_errs.is_empty() {
                return GamesQueryParseError::ValidationFailedList(validation_errs);
            }

            if parse_errs.len() == 1 && validation_errs.is_empty() {
                return parse_errs.pop().unwrap();
            }

            let mut messages = parse_errs
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>();

            if !validation_errs.is_empty() {
                messages.extend(validation_errs.into_iter().map(|e| e.to_string()));
            }

            GamesQueryParseError::Generic(messages.join("; "))
        })
    }
}

impl GamesQueryOptions {
    /// Parses the query-string fields without running `validate_all`.
    ///
    /// Callers that need to override fields (e.g. the archive forces
    /// `game_progress = Finished`) should use this and validate themselves;
    /// otherwise validation would run against the default `game_progress`.
    pub fn parse_query(s: &str) -> Result<Self, Vec<GamesQueryParseError>> {
        let (opts, errs) = Self::parse_fields(s);
        if errs.is_empty() {
            Ok(opts)
        } else {
            Err(errs)
        }
    }

    /// Parses from a query string, collecting all parse and validation errors.
    pub fn parse_with_errors(s: &str) -> Result<Self, Vec<GamesQueryParseError>> {
        let (opts, mut errs) = Self::parse_fields(s);

        let validated = opts.clone().validate_all();
        if let Err(ref validation_errs) = validated {
            errs.push(GamesQueryParseError::ValidationFailedList(
                validation_errs.to_vec(),
            ));
        }

        if errs.is_empty() {
            Ok(validated.ok().unwrap_or(opts))
        } else {
            Err(errs)
        }
    }

    fn parse_fields(s: &str) -> (Self, Vec<GamesQueryParseError>) {
        let mut opts = GamesQueryOptions::default();
        let mut errs = Vec::new();
        let mut speeds_set = false;

        let trimmed = s.trim();
        let query = trimmed.strip_prefix('?').unwrap_or(trimmed);

        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let mut split = pair.splitn(2, '=');
            let key = split.next().unwrap_or_default();
            let raw_val = split.next().unwrap_or_default();
            let value = raw_val.to_string();

            let parse_err = match key {
                "player1" => {
                    opts.player1 = Some(value);
                    None
                }
                "player2" => {
                    opts.player2 = Some(value);
                    None
                }
                "fixed_colors" => match parse_bool(&value, "fixed_colors") {
                    Ok(v) => {
                        opts.fixed_colors = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "exclude_bots" => match parse_bool(&value, "exclude_bots") {
                    Ok(v) => {
                        opts.exclude_bots = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "only_tournament" => match parse_bool(&value, "only_tournament") {
                    Ok(v) => {
                        opts.only_tournament = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "rated" => match parse_option_bool(&value, "rated") {
                    Ok(v) => {
                        opts.rated = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "expansions" => match parse_option_bool(&value, "expansions") {
                    Ok(v) => {
                        opts.expansions = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "speeds" => {
                    let mut speeds = Vec::new();
                    for part in value.split(',') {
                        if part.trim().is_empty() {
                            continue;
                        }
                        match GameSpeed::from_str(part.trim()) {
                            Ok(speed) => speeds.push(speed),
                            Err(_) => {
                                errs.push(GamesQueryParseError::InvalidSpeed(part.to_string()));
                            }
                        }
                    }
                    if !speeds.is_empty() {
                        speeds.sort();
                        speeds.dedup();
                        opts.speeds = speeds;
                    }
                    speeds_set = true;
                    None
                }
                "rating_min" => match parse_i32(&value, "rating_min") {
                    Ok(v) => {
                        opts.rating_min = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "rating_max" => match parse_i32(&value, "rating_max") {
                    Ok(v) => {
                        opts.rating_max = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "turn_min" => match parse_i32(&value, "turn_min") {
                    Ok(v) => {
                        opts.turn_min = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "turn_max" => match parse_i32(&value, "turn_max") {
                    Ok(v) => {
                        opts.turn_max = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "date_start" => match parse_date(&value, "date_start") {
                    Ok(v) => {
                        opts.date_start = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "date_end" => match parse_date(&value, "date_end") {
                    Ok(v) => {
                        opts.date_end = v;
                        None
                    }
                    Err(e) => Some(e),
                },
                "result_filter" => match ResultFilter::from_str(&value) {
                    Ok(v) => {
                        opts.result_filter = v;
                        None
                    }
                    Err(_) => Some(GamesQueryParseError::InvalidResultFilter),
                },
                "sort_key" => match GameSortKey::from_str(&value) {
                    Ok(v) => {
                        opts.sort.key = v;
                        None
                    }
                    Err(_) => Some(GamesQueryParseError::InvalidSortKey),
                },
                "sort_asc" => parse_bool(&value, "sort_asc")
                    .map(|v| opts.sort.ascending = v)
                    .err()
                    .map(|_| GamesQueryParseError::InvalidSortDirection),
                "page" => match value.trim().parse::<usize>() {
                    Ok(p) if (1..=10000).contains(&p) => {
                        opts.page = p;
                        None
                    }
                    _ => Some(GamesQueryParseError::InvalidPage),
                },
                "batch_size" => match value.trim().parse::<usize>() {
                    Ok(s) if ALLOWED_BATCH_SIZES.contains(&s) => {
                        opts.batch_size = s;
                        None
                    }
                    _ => Some(GamesQueryParseError::InvalidBatchSize),
                },
                "position_hash" => match value.trim().parse::<i64>() {
                    Ok(h) => {
                        opts.position_hash = Some(h);
                        None
                    }
                    Err(e) => Some(GamesQueryParseError::Generic(format!(
                        "invalid position_hash: {e}"
                    ))),
                },
                _ => None,
            };

            if let Some(e) = parse_err {
                errs.push(e);
            }
        }

        if !speeds_set {
            opts.speeds.clear();
        }

        (opts, errs)
    }
}

fn parse_bool(input: &str, field: &'static str) -> Result<bool, GamesQueryParseError> {
    match input {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(GamesQueryParseError::InvalidBool(field)),
    }
}

fn parse_option_bool(
    input: &str,
    field: &'static str,
) -> Result<Option<bool>, GamesQueryParseError> {
    match input {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        "any" => Ok(None),
        _ => Err(GamesQueryParseError::InvalidOptionBool(field)),
    }
}

fn parse_i32(input: &str, field: &'static str) -> Result<Option<i32>, GamesQueryParseError> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    input
        .trim()
        .parse::<i32>()
        .map(Some)
        .map_err(|e| GamesQueryParseError::InvalidRating {
            field,
            error: e.to_string(),
        })
}

fn parse_date(
    input: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, GamesQueryParseError> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    let date = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|e| {
        GamesQueryParseError::InvalidDate {
            field,
            error: e.to_string(),
        }
    })?;
    let dt = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| GamesQueryParseError::InvalidDate {
            field,
            error: "invalid date".to_string(),
        })?;
    Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn base_options() -> GamesQueryOptions {
        GamesQueryOptions {
            batch_size: 10,
            game_progress: GameProgress::Finished,
            ..GamesQueryOptions::default()
        }
    }

    #[test]
    fn rejects_duplicate_players() {
        let options = GamesQueryOptions {
            player1: Some("User".to_string()),
            player2: Some("user".to_string()),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::DuplicatePlayers)
        ));
    }

    #[test]
    fn rejects_missing_player_for_result() {
        let options = GamesQueryOptions {
            result_filter: ResultFilter::PlayerWins(1),
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::MissingPlayerForResult { slot: 1 })
        ));
    }

    #[test]
    fn rejects_missing_player_for_loses() {
        let options = GamesQueryOptions {
            result_filter: ResultFilter::PlayerLoses(1),
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::MissingPlayerForResult { slot: 1 })
        ));
    }

    #[test]
    fn allows_player_loses_with_player_one_present() {
        let options = GamesQueryOptions {
            player1: Some("someone".into()),
            result_filter: ResultFilter::PlayerLoses(1),
            ..base_options()
        };
        assert!(options.validate_all().is_ok());
    }

    #[test]
    fn rated_with_untimed_speed_is_allowed() {
        // Rated is always selectable now; an untimed speed no longer forces it off.
        let options = GamesQueryOptions {
            speeds: vec![GameSpeed::Untimed, GameSpeed::Blitz],
            rated: Some(true),
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(validated.rated, Some(true));
    }

    #[test]
    fn rated_none_with_untimed_speed_stays_none() {
        let options = GamesQueryOptions {
            speeds: vec![GameSpeed::Untimed, GameSpeed::Blitz],
            rated: None,
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(validated.speeds, vec![GameSpeed::Blitz, GameSpeed::Untimed]);
        assert_eq!(validated.rated, None);
    }

    #[test]
    fn parses_exclude_bots_and_fixed_colors() {
        let opts =
            GamesQueryOptions::from_str("player1=ion&exclude_bots=true&fixed_colors=true").unwrap();

        assert!(opts.exclude_bots);
        assert!(opts.fixed_colors);
    }

    #[test]
    fn base_only_rated_is_allowed() {
        // Rated is always selectable; base-only no longer forbids it.
        let options = GamesQueryOptions {
            expansions: Some(false),
            rated: Some(true),
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(validated.rated, Some(true));
    }

    #[test]
    fn any_expansions_serializes_as_any() {
        let opts = GamesQueryOptions {
            expansions: None,
            rated: None,
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("expansions=any"));
        assert!(query.contains("rated=any"));
    }

    #[test]
    fn from_str_parses_speeds_and_ignores_unknown_keys() {
        // time_mode was removed; an old time_mode key in a URL is ignored.
        let opts =
            GamesQueryOptions::from_str("time_mode=any&speeds=Blitz,Correspondence").unwrap();

        assert_eq!(
            opts.speeds,
            vec![GameSpeed::Blitz, GameSpeed::Correspondence]
        );
    }

    #[test]
    fn rejects_rating_bounds_conflicts() {
        let options = GamesQueryOptions {
            rating_min: Some(2500),
            rating_max: Some(2400),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::RatingBoundsInvalid)
        ));
    }

    #[test]
    fn rejects_rating_out_of_range() {
        let options = GamesQueryOptions {
            rating_min: Some(3500),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::RatingOutOfRange)
        ));
    }

    #[test]
    fn rejects_turn_bounds_conflicts() {
        let options = GamesQueryOptions {
            turn_min: Some(20),
            turn_max: Some(10),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::TurnBoundsInvalid)
        ));
    }

    #[test]
    fn rejects_invalid_batch_size() {
        for invalid in [7, 20, 101] {
            let options = GamesQueryOptions {
                batch_size: invalid,
                ..base_options()
            };
            assert!(
                options.validate_all().is_err(),
                "batch_size {} should be rejected",
                invalid
            );
        }
    }

    #[test]
    fn accepts_valid_batch_sizes() {
        for valid in ALLOWED_BATCH_SIZES {
            let options = GamesQueryOptions {
                batch_size: valid,
                ..base_options()
            };
            assert!(options.validate_all().is_ok());
        }
    }

    #[test]
    fn rejects_batch_token_sort_mismatch() {
        let options = GamesQueryOptions {
            sort: GameSort {
                key: GameSortKey::Turns,
                ascending: true,
            },
            batch_token: Some(BatchToken {
                sort: GameSort::default(),
                primary_value: SortValue::Turns(0),
                updated_at: Utc::now(),
                id: Uuid::new_v4(),
            }),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::BatchTokenSortMismatch)
        ));
    }

    #[test]
    fn rejects_rating_filter_with_rated_false() {
        let options = GamesQueryOptions {
            rated: Some(false),
            rating_min: Some(1000),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::RatingFiltersRequireRated)
        ));
    }

    #[test]
    fn rejects_player_two_wins_without_any_player() {
        let options = GamesQueryOptions {
            result_filter: ResultFilter::PlayerWins(2),
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::MissingPlayerForResult { slot: 2 })
        ));
    }

    #[test]
    fn allows_player_two_wins_with_player_one_present() {
        let options = GamesQueryOptions {
            player1: Some("someone".into()),
            result_filter: ResultFilter::PlayerWins(2),
            ..base_options()
        };

        assert!(options.validate_all().is_ok());
    }

    #[test]
    fn from_str_preserves_parse_error() {
        let err = GamesQueryOptions::from_str("fixed_colors=maybe").unwrap_err();

        assert!(matches!(
            err,
            GamesQueryParseError::InvalidBool("fixed_colors")
        ));
    }

    #[test]
    fn from_str_reports_multiple_parse_errors() {
        let err =
            GamesQueryOptions::from_str("fixed_colors=maybe&speeds=hyper&sort_asc=up").unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("invalid bool for fixed_colors"));
        assert!(msg.contains("invalid speed hyper"));
        assert!(msg.contains("invalid sort direction"));
    }

    #[test]
    fn position_hash_round_trips_in_query_string() {
        let opts = GamesQueryOptions {
            position_hash: Some(-42),
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("position_hash=-42"));

        let parsed = GamesQueryOptions::from_str(&query).unwrap();
        assert_eq!(parsed.position_hash, Some(-42));
    }

    #[test]
    fn position_hash_absent_by_default() {
        assert!(!base_options().to_string().contains("position_hash"));
    }

    #[test]
    fn only_tournament_round_trips_in_query_string() {
        let opts = GamesQueryOptions {
            only_tournament: true,
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("only_tournament=true"));

        let parsed = GamesQueryOptions::from_str(&query).unwrap();
        assert!(parsed.only_tournament);
    }

    #[test]
    fn from_str_allows_leading_question_mark() {
        let opts = GamesQueryOptions::from_str("?player1=someone").unwrap();

        assert_eq!(opts.player1.as_deref(), Some("someone"));
    }

    #[test]
    fn display_omits_batch_size_when_default() {
        let opts = GamesQueryOptions {
            batch_size: 10,
            ..base_options()
        };
        assert!(!opts.to_string().contains("batch_size="));
    }

    #[test]
    fn display_includes_batch_size_when_non_default() {
        let opts = GamesQueryOptions {
            batch_size: 25,
            ..base_options()
        };
        assert!(opts.to_string().contains("batch_size=25"));
    }

    #[test]
    fn display_includes_page_when_gt_one() {
        let opts = GamesQueryOptions {
            page: 2,
            ..base_options()
        };
        assert!(opts.to_string().contains("page=2"));
    }

    #[test]
    fn display_omits_page_when_one() {
        let opts = GamesQueryOptions {
            page: 1,
            ..base_options()
        };
        assert!(!opts.to_string().contains("page=1"));
    }

    #[test]
    fn parse_page() {
        let opts = GamesQueryOptions::from_str("player1=ion&page=3").unwrap();
        assert_eq!(opts.page, 3);
    }

    #[test]
    fn parse_batch_size() {
        let opts = GamesQueryOptions::from_str("batch_size=25").unwrap();
        assert_eq!(opts.batch_size, 25);
    }

    #[test]
    fn parse_rejects_invalid_batch_size() {
        assert!(GamesQueryOptions::from_str("batch_size=7").is_err());
    }

    #[test]
    fn rejects_page_zero() {
        let options = GamesQueryOptions {
            page: 0,
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::PageOutOfRange)
        ));
    }

    #[test]
    fn rejects_page_over_max() {
        let options = GamesQueryOptions {
            page: 10001,
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::PageOutOfRange)
        ));
    }

    #[test]
    fn parse_query_defers_validation_for_finished_only_sorts() {
        // Bare parse must not reject Turns sort just because the default
        // game_progress is not Finished; the archive forces Finished after.
        let mut opts = GamesQueryOptions::parse_query("sort_key=Turns&sort_asc=false").unwrap();
        assert_eq!(opts.sort.key, GameSortKey::Turns);

        opts.game_progress = GameProgress::Finished;
        assert!(opts.validate_all().is_ok());
    }

    #[test]
    fn from_str_still_rejects_turns_sort_without_finished() {
        // from_str validates against the default game_progress (All), so the
        // finished-only sort key is still rejected on that path.
        assert!(GamesQueryOptions::from_str("sort_key=Turns").is_err());
    }

    #[test]
    fn rejects_turns_sort_for_unfinished() {
        let options = GamesQueryOptions {
            game_progress: GameProgress::Playing,
            sort: GameSort {
                key: GameSortKey::Turns,
                ascending: false,
            },
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&GameQueryValidationError::SortKeyRequiresFinished)
        ));
    }
}
