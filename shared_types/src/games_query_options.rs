use crate::{game_speed::GameSpeed, time_mode::TimeMode};
use chrono::{DateTime, Utc};
use hive_lib::Color;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

// Legacy search types (existing UI)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum ResultType {
    Win,
    Loss,
    Draw,
}

impl std::fmt::Display for ResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultType::Win => write!(f, "Win"),
            ResultType::Loss => write!(f, "Loss"),
            ResultType::Draw => write!(f, "Draw"),
        }
    }
}

impl std::str::FromStr for ResultType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Win" => Ok(ResultType::Win),
            "Loss" => Ok(ResultType::Loss),
            "Draw" => Ok(ResultType::Draw),
            _ => Err(anyhow::anyhow!("Invalid ResultType string")),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub struct BatchInfo {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerFilter {
    pub username: String,
    pub color: Option<Color>,
    pub result: Option<ResultType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GamesQueryOptions {
    pub player1: Option<PlayerFilter>,
    pub player2: Option<PlayerFilter>,
    pub speeds: Vec<GameSpeed>,
    pub current_batch: Option<BatchInfo>,
    pub batch_size: usize,
    pub game_progress: GameProgress,
    pub expansions: Option<bool>,
    pub rated: Option<bool>,
    pub exclude_bots: bool,
}

// Finished/advanced search types (parallel path)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinishedResultFilter {
    Any,
    ColorWins(Color),
    PlayerWins(u8),
    Draw,
    NotDraw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FinishedGameSortKey {
    Date,
    Turns,
    RatingAvg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinishedGameSort {
    pub key: FinishedGameSortKey,
    pub ascending: bool,
}

impl Default for FinishedGameSort {
    fn default() -> Self {
        Self {
            key: FinishedGameSortKey::Date,
            ascending: false,
        }
    }
}

impl FinishedGameSort {
    pub fn is_desc(&self) -> bool {
        !self.ascending
    }
}

impl std::str::FromStr for FinishedGameSortKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Date" => Ok(FinishedGameSortKey::Date),
            "Turns" => Ok(FinishedGameSortKey::Turns),
            "RatingAvg" => Ok(FinishedGameSortKey::RatingAvg),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for FinishedResultFilter {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "any" => Ok(FinishedResultFilter::Any),
            "white_wins" => Ok(FinishedResultFilter::ColorWins(Color::White)),
            "black_wins" => Ok(FinishedResultFilter::ColorWins(Color::Black)),
            "player1_wins" => Ok(FinishedResultFilter::PlayerWins(1)),
            "player2_wins" => Ok(FinishedResultFilter::PlayerWins(2)),
            "draw" => Ok(FinishedResultFilter::Draw),
            "not_draw" => Ok(FinishedResultFilter::NotDraw),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for FinishedResultFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            FinishedResultFilter::Any => "any",
            FinishedResultFilter::ColorWins(Color::White) => "white_wins",
            FinishedResultFilter::ColorWins(Color::Black) => "black_wins",
            FinishedResultFilter::PlayerWins(1) => "player1_wins",
            FinishedResultFilter::PlayerWins(2) => "player2_wins",
            FinishedResultFilter::Draw => "draw",
            FinishedResultFilter::NotDraw => "not_draw",
            FinishedResultFilter::PlayerWins(_) => "any",
        };
        write!(f, "{value}")
    }
}

impl std::fmt::Display for FinishedGameSortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            FinishedGameSortKey::Date => "Date",
            FinishedGameSortKey::Turns => "Turns",
            FinishedGameSortKey::RatingAvg => "RatingAvg",
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
    pub sort: FinishedGameSort,
    pub primary_value: SortValue,
    pub updated_at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinishedGamesQueryOptions {
    pub player1: Option<String>,
    pub player2: Option<String>,
    pub fixed_colors: bool,
    pub exclude_bots: bool,
    pub only_tournament: bool,
    pub rated: Option<bool>,
    pub expansions: Option<bool>,
    pub time_mode: Option<TimeMode>,
    pub speeds: Vec<GameSpeed>,
    pub rating_min: Option<i32>,
    pub rating_max: Option<i32>,
    pub turn_min: Option<i32>,
    pub turn_max: Option<i32>,
    pub date_start: Option<DateTime<Utc>>,
    pub date_end: Option<DateTime<Utc>>,
    pub result_filter: FinishedResultFilter,
    pub batch_token: Option<BatchToken>,
    pub batch_size: usize,
    pub page: usize,
    pub sort: FinishedGameSort,
    pub game_progress: GameProgress,
}

impl Default for FinishedGamesQueryOptions {
    fn default() -> Self {
        Self {
            player1: None,
            player2: None,
            fixed_colors: false,
            exclude_bots: false,
            only_tournament: false,
            rated: None,
            expansions: None,
            time_mode: None,
            speeds: GameSpeed::all_games(),
            rating_min: None,
            rating_max: None,
            turn_min: None,
            turn_max: None,
            date_start: None,
            date_end: None,
            result_filter: FinishedResultFilter::Any,
            batch_token: None,
            batch_size: 50,
            page: 1,
            sort: FinishedGameSort::default(),
            game_progress: GameProgress::Finished,
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishedGameQueryValidationError {
    #[error("player filters must not reference the same normalized username")]
    DuplicatePlayers,
    #[error("result filter for player {slot} requires that player to be provided")]
    MissingPlayerForResult { slot: u8 },
    #[error("time mode must be set when filtering by speeds")]
    TimeModeRequiredForSpeeds,
    #[error("speed filters must match the selected time mode")]
    InvalidSpeedForTimeMode,
    #[error("untimed games cannot be rated")]
    RatedNotAllowedForUntimed,
    #[error("base-only games cannot be rated")]
    RatedNotAllowedForBaseOnly,
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
    #[error("batch size must be between 1 and 100")]
    BatchSizeInvalid,
    #[error("page must be at least 1 and at most 10000")]
    PageOutOfRange,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishedGamesQueryParseError {
    #[error("invalid bool for {0}")]
    InvalidBool(&'static str),
    #[error("invalid option bool for {0}")]
    InvalidOptionBool(&'static str),
    #[error("invalid time mode")]
    InvalidTimeMode,
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
    ValidationFailedList(Vec<FinishedGameQueryValidationError>),
    #[error("parse error: {0}")]
    Generic(String),
}

impl FinishedGamesQueryOptions {
    pub fn validate_all(mut self) -> Result<Self, Vec<FinishedGameQueryValidationError>> {
        let mut errors = Vec::new();

        if self.batch_size == 0 || self.batch_size > 100 {
            errors.push(FinishedGameQueryValidationError::BatchSizeInvalid);
        }

        if self.page == 0 || self.page > 10000 {
            errors.push(FinishedGameQueryValidationError::PageOutOfRange);
        }

        let player1 = self.normalize_player(self.player1.as_ref());
        let player2 = self.normalize_player(self.player2.as_ref());

        if let (Some(p1), Some(p2)) = (&player1, &player2) {
            if p1 == p2 {
                errors.push(FinishedGameQueryValidationError::DuplicatePlayers);
            }
        }

        if matches!(self.result_filter, FinishedResultFilter::PlayerWins(1)) && player1.is_none() {
            errors.push(FinishedGameQueryValidationError::MissingPlayerForResult { slot: 1 });
        }
        if matches!(self.result_filter, FinishedResultFilter::PlayerWins(2))
            && player2.is_none()
            && player1.is_none()
        {
            errors.push(FinishedGameQueryValidationError::MissingPlayerForResult { slot: 2 });
        }

        let mut speeds = self.speeds.clone();
        speeds.sort();
        speeds.dedup();
        let realtime_speeds = GameSpeed::real_time_speeds();

        let mut rated = self.rated;

        if speeds.is_empty() {
            match self.time_mode {
                Some(TimeMode::RealTime) => speeds = realtime_speeds.clone(),
                Some(TimeMode::Correspondence) => speeds = vec![GameSpeed::Correspondence],
                Some(TimeMode::Untimed) => speeds = vec![GameSpeed::Untimed],
                None => {}
            }
        }

        if let Some(mode) = self.time_mode {
            match mode {
                TimeMode::RealTime => {
                    if speeds.iter().any(|s| !realtime_speeds.contains(s)) {
                        errors.push(FinishedGameQueryValidationError::InvalidSpeedForTimeMode);
                    }
                }
                TimeMode::Correspondence => {
                    if speeds.iter().any(|s| *s != GameSpeed::Correspondence) {
                        errors.push(FinishedGameQueryValidationError::InvalidSpeedForTimeMode);
                    }
                }
                TimeMode::Untimed => {
                    if speeds.iter().any(|s| *s != GameSpeed::Untimed) {
                        errors.push(FinishedGameQueryValidationError::InvalidSpeedForTimeMode);
                    }
                }
            }
        }

        if self.uses_rating_filter() {
            if rated != Some(true) {
                errors.push(FinishedGameQueryValidationError::RatingFiltersRequireRated);
            }
            rated = Some(true);

            let valid_bounds = self.rating_min.is_none_or(|min| (0..=3000).contains(&min))
                && self.rating_max.is_none_or(|max| (0..=3000).contains(&max));
            if !valid_bounds {
                errors.push(FinishedGameQueryValidationError::RatingOutOfRange);
            }
            if let (Some(min), Some(max)) = (self.rating_min, self.rating_max) {
                if min > max {
                    errors.push(FinishedGameQueryValidationError::RatingBoundsInvalid);
                }
            }
        }

        if matches!(rated, Some(true)) && self.time_mode == Some(TimeMode::Untimed) {
            errors.push(FinishedGameQueryValidationError::RatedNotAllowedForUntimed);
        }

        if matches!(rated, Some(true)) && self.expansions == Some(false) {
            errors.push(FinishedGameQueryValidationError::RatedNotAllowedForBaseOnly);
        }

        let has_untimed_speed = speeds.contains(&GameSpeed::Untimed);
        if rated.is_none() && (self.expansions == Some(false) || self.time_mode == Some(TimeMode::Untimed)) {
            rated = Some(false);
        }
        if matches!(rated, Some(true)) && has_untimed_speed {
            errors.push(FinishedGameQueryValidationError::RatedNotAllowedForUntimed);
        }

        if let (Some(min), Some(max)) = (self.turn_min, self.turn_max) {
            if min > max {
                errors.push(FinishedGameQueryValidationError::TurnBoundsInvalid);
            }
        }

        if let (Some(start), Some(end)) = (self.date_start, self.date_end) {
            if start > end {
                errors.push(FinishedGameQueryValidationError::DateBoundsInvalid);
            }
        }

        if let Some(token) = &self.batch_token {
            if token.sort != self.sort {
                errors.push(FinishedGameQueryValidationError::BatchTokenSortMismatch);
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

impl std::fmt::Display for FinishedGamesQueryOptions {
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
        match self.time_mode {
            Some(mode) => push("time_mode", mode.to_string()),
            None => push("time_mode", "any".into()),
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

        if self.page > 1 {
            push("page", self.page.to_string());
        }

        if parts.is_empty() {
            write!(f, "")
        } else {
            write!(f, "?{}", parts.join("&"))
        }
    }
}

impl std::str::FromStr for FinishedGamesQueryOptions {
    type Err = FinishedGamesQueryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FinishedGamesQueryOptions::parse_with_errors(s).map_err(|errs| {
            let mut validation_errs = Vec::new();
            let mut parse_errs = Vec::new();

            for err in errs {
                match err {
                    FinishedGamesQueryParseError::ValidationFailedList(v) => {
                        validation_errs.extend(v);
                    }
                    other => parse_errs.push(other),
                }
            }

            if parse_errs.is_empty() {
                return FinishedGamesQueryParseError::ValidationFailedList(validation_errs);
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

            FinishedGamesQueryParseError::Generic(messages.join("; "))
        })
    }
}

impl FinishedGamesQueryOptions {
    /// Parses from a query string, collecting all parse and validation errors.
    pub fn parse_with_errors(s: &str) -> Result<Self, Vec<FinishedGamesQueryParseError>> {
        let mut opts = FinishedGamesQueryOptions::default();
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
                "time_mode" => match parse_time_mode(&value) {
                    Ok(v) => {
                        opts.time_mode = v;
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
                                errs.push(FinishedGamesQueryParseError::InvalidSpeed(
                                    part.to_string(),
                                ));
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
                "result_filter" => match FinishedResultFilter::from_str(&value) {
                    Ok(v) => {
                        opts.result_filter = v;
                        None
                    }
                    Err(_) => Some(FinishedGamesQueryParseError::InvalidResultFilter),
                },
                "sort_key" => match FinishedGameSortKey::from_str(&value) {
                    Ok(v) => {
                        opts.sort.key = v;
                        None
                    }
                    Err(_) => Some(FinishedGamesQueryParseError::InvalidSortKey),
                },
                "sort_asc" => parse_bool(&value, "sort_asc")
                    .map(|v| opts.sort.ascending = v)
                    .err()
                    .map(|_| FinishedGamesQueryParseError::InvalidSortDirection),
                "page" => {
                    match value.trim().parse::<usize>() {
                        Ok(p) if p >= 1 && p <= 10000 => {
                            opts.page = p;
                            None
                        }
                        _ => Some(FinishedGamesQueryParseError::InvalidPage),
                    }
                }
                _ => None,
            };

            if let Some(e) = parse_err {
                errs.push(e);
            }
        }

        if !speeds_set {
            opts.speeds.clear();
        }

        let validated = opts.clone().validate_all();
        if let Err(ref validation_errs) = validated {
            errs.push(FinishedGamesQueryParseError::ValidationFailedList(
                validation_errs.to_vec(),
            ));
        }

        if errs.is_empty() {
            Ok(validated.ok().unwrap_or(opts))
        } else {
            Err(errs)
        }
    }
}

fn parse_bool(input: &str, field: &'static str) -> Result<bool, FinishedGamesQueryParseError> {
    match input {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(FinishedGamesQueryParseError::InvalidBool(field)),
    }
}

fn parse_option_bool(
    input: &str,
    field: &'static str,
) -> Result<Option<bool>, FinishedGamesQueryParseError> {
    match input {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        "any" => Ok(None),
        _ => Err(FinishedGamesQueryParseError::InvalidOptionBool(field)),
    }
}

fn parse_i32(
    input: &str,
    field: &'static str,
) -> Result<Option<i32>, FinishedGamesQueryParseError> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    input
        .trim()
        .parse::<i32>()
        .map(Some)
        .map_err(|e| FinishedGamesQueryParseError::InvalidRating {
            field,
            error: e.to_string(),
        })
}

fn parse_date(
    input: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, FinishedGamesQueryParseError> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    let date = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|e| {
        FinishedGamesQueryParseError::InvalidDate {
            field,
            error: e.to_string(),
        }
    })?;
    let dt =
        date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| FinishedGamesQueryParseError::InvalidDate {
                field,
                error: "invalid date".to_string(),
            })?;
    Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)))
}

fn parse_time_mode(input: &str) -> Result<Option<TimeMode>, FinishedGamesQueryParseError> {
    if input == "any" {
        return Ok(None);
    }
    TimeMode::from_str(input)
        .map(Some)
        .map_err(|_| FinishedGamesQueryParseError::InvalidTimeMode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn base_options() -> FinishedGamesQueryOptions {
        FinishedGamesQueryOptions {
            batch_size: 10,
            ..FinishedGamesQueryOptions::default()
        }
    }

    #[test]
    fn rejects_duplicate_players() {
        let options = FinishedGamesQueryOptions {
            player1: Some("User".to_string()),
            player2: Some("user".to_string()),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::DuplicatePlayers)
        ));
    }

    #[test]
    fn rejects_missing_player_for_result() {
        let options = FinishedGamesQueryOptions {
            result_filter: FinishedResultFilter::PlayerWins(1),
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::MissingPlayerForResult { slot: 1 })
        ));
    }

    #[test]
    fn rejects_speed_that_does_not_match_time_mode() {
        let options = FinishedGamesQueryOptions {
            time_mode: Some(TimeMode::Correspondence),
            speeds: vec![GameSpeed::Blitz],
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::InvalidSpeedForTimeMode)
        ));
    }

    #[test]
    fn rejects_non_realtime_speed_in_realtime_mode() {
        let options = FinishedGamesQueryOptions {
            time_mode: Some(TimeMode::RealTime),
            speeds: vec![GameSpeed::Correspondence],
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::InvalidSpeedForTimeMode)
        ));
    }

    #[test]
    fn fills_correspondence_speed_when_time_mode_set() {
        let options = FinishedGamesQueryOptions {
            time_mode: Some(TimeMode::Correspondence),
            speeds: vec![],
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(validated.speeds, vec![GameSpeed::Correspondence]);
    }

    #[test]
    fn from_str_allows_correspondence_without_speeds() {
        let opts = FinishedGamesQueryOptions::from_str("time_mode=Correspondence").unwrap();

        assert_eq!(opts.time_mode, Some(TimeMode::Correspondence));
        assert_eq!(opts.speeds, vec![GameSpeed::Correspondence]);
    }

    #[test]
    fn allows_any_time_mode_with_speeds() {
        let options = FinishedGamesQueryOptions {
            time_mode: None,
            speeds: vec![GameSpeed::Blitz, GameSpeed::Correspondence],
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(validated.time_mode, None);
        assert_eq!(
            validated.speeds,
            vec![GameSpeed::Blitz, GameSpeed::Correspondence]
        );
    }

    #[test]
    fn untimed_rated_is_rejected() {
        let options = FinishedGamesQueryOptions {
            time_mode: Some(TimeMode::Untimed),
            rated: Some(true),
            speeds: vec![],
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatedNotAllowedForUntimed)
        ));
    }

    #[test]
    fn rated_with_untimed_speed_is_rejected() {
        let options = FinishedGamesQueryOptions {
            time_mode: None,
            speeds: vec![GameSpeed::Untimed, GameSpeed::Blitz],
            rated: Some(true),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatedNotAllowedForUntimed)
        ));
    }

    #[test]
    fn rated_any_with_untimed_speed_is_allowed() {
        let options = FinishedGamesQueryOptions {
            time_mode: None,
            speeds: vec![GameSpeed::Untimed, GameSpeed::Blitz],
            rated: None,
            ..base_options()
        };

        let validated = options.validate_all().unwrap();
        assert_eq!(
            validated.speeds,
            vec![GameSpeed::Blitz, GameSpeed::Untimed]
        );
        assert_eq!(validated.rated, None);
    }

    #[test]
    fn parses_exclude_bots_and_fixed_colors() {
        let opts = FinishedGamesQueryOptions::from_str(
            "player1=ion&exclude_bots=true&fixed_colors=true",
        )
        .unwrap();

        assert!(opts.exclude_bots);
        assert!(opts.fixed_colors);
    }

    #[test]
    fn base_only_rated_is_rejected() {
        let options = FinishedGamesQueryOptions {
            expansions: Some(false),
            rated: Some(true),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatedNotAllowedForBaseOnly)
        ));
    }

    #[test]
    fn any_expansions_serializes_as_any() {
        let opts = FinishedGamesQueryOptions {
            expansions: None,
            rated: None,
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("expansions=any"));
        assert!(query.contains("rated=any"));
    }

    #[test]
    fn any_time_mode_serializes_as_any() {
        let opts = FinishedGamesQueryOptions {
            time_mode: None,
            speeds: vec![GameSpeed::Untimed],
            rated: Some(false),
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("time_mode=any"));
    }

    #[test]
    fn from_str_allows_any_time_mode_with_speeds() {
        let opts =
            FinishedGamesQueryOptions::from_str("time_mode=any&speeds=Blitz,Correspondence")
                .unwrap();

        assert_eq!(opts.time_mode, None);
        assert_eq!(
            opts.speeds,
            vec![GameSpeed::Blitz, GameSpeed::Correspondence]
        );
    }

    #[test]
    fn rejects_rating_bounds_conflicts() {
        let options = FinishedGamesQueryOptions {
            rating_min: Some(2500),
            rating_max: Some(2400),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatingBoundsInvalid)
        ));
    }

    #[test]
    fn rejects_rating_out_of_range() {
        let options = FinishedGamesQueryOptions {
            rating_min: Some(3500),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatingOutOfRange)
        ));
    }

    #[test]
    fn rejects_turn_bounds_conflicts() {
        let options = FinishedGamesQueryOptions {
            turn_min: Some(20),
            turn_max: Some(10),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::TurnBoundsInvalid)
        ));
    }

    #[test]
    fn rejects_batch_size_over_max() {
        let options = FinishedGamesQueryOptions {
            batch_size: 101,
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::BatchSizeInvalid)
        ));
    }

    #[test]
    fn rejects_batch_token_sort_mismatch() {
        let options = FinishedGamesQueryOptions {
            sort: FinishedGameSort {
                key: FinishedGameSortKey::Turns,
                ascending: true,
            },
            batch_token: Some(BatchToken {
                sort: FinishedGameSort::default(),
                primary_value: SortValue::Turns(0),
                updated_at: Utc::now(),
                id: Uuid::new_v4(),
            }),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::BatchTokenSortMismatch)
        ));
    }

    #[test]
    fn rejects_rating_filter_with_rated_false() {
        let options = FinishedGamesQueryOptions {
            rated: Some(false),
            rating_min: Some(1000),
            ..base_options()
        };

        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::RatingFiltersRequireRated)
        ));
    }

    #[test]
    fn rejects_player_two_wins_without_any_player() {
        let options = FinishedGamesQueryOptions {
            result_filter: FinishedResultFilter::PlayerWins(2),
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::MissingPlayerForResult { slot: 2 })
        ));
    }

    #[test]
    fn allows_player_two_wins_with_player_one_present() {
        let options = FinishedGamesQueryOptions {
            player1: Some("someone".into()),
            result_filter: FinishedResultFilter::PlayerWins(2),
            ..base_options()
        };

        assert!(options.validate_all().is_ok());
    }

    #[test]
    fn from_str_preserves_parse_error() {
        let err = FinishedGamesQueryOptions::from_str("fixed_colors=maybe").unwrap_err();

        assert!(matches!(
            err,
            FinishedGamesQueryParseError::InvalidBool("fixed_colors")
        ));
    }

    #[test]
    fn from_str_reports_multiple_parse_errors() {
        let err =
            FinishedGamesQueryOptions::from_str("fixed_colors=maybe&speeds=hyper&sort_asc=up")
                .unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("invalid bool for fixed_colors"));
        assert!(msg.contains("invalid speed hyper"));
        assert!(msg.contains("invalid sort direction"));
    }

    #[test]
    fn only_tournament_round_trips_in_query_string() {
        let opts = FinishedGamesQueryOptions {
            only_tournament: true,
            ..base_options()
        };

        let query = opts.to_string();
        assert!(query.contains("only_tournament=true"));

        let parsed = FinishedGamesQueryOptions::from_str(&query).unwrap();
        assert!(parsed.only_tournament);
    }

    #[test]
    fn from_str_allows_leading_question_mark() {
        let opts = FinishedGamesQueryOptions::from_str("?player1=someone").unwrap();

        assert_eq!(opts.player1.as_deref(), Some("someone"));
    }

    #[test]
    fn display_omits_batch_size() {
        let opts = FinishedGamesQueryOptions {
            batch_size: 42,
            ..FinishedGamesQueryOptions::default()
        };

        assert!(!opts.to_string().contains("batch_size="));
    }

    #[test]
    fn display_includes_page_when_gt_one() {
        let opts = FinishedGamesQueryOptions {
            page: 2,
            ..FinishedGamesQueryOptions::default()
        };
        assert!(opts.to_string().contains("page=2"));
    }

    #[test]
    fn display_omits_page_when_one() {
        let opts = FinishedGamesQueryOptions {
            page: 1,
            ..FinishedGamesQueryOptions::default()
        };
        assert!(!opts.to_string().contains("page=1"));
    }

    #[test]
    fn parse_page() {
        let opts = FinishedGamesQueryOptions::from_str("player1=ion&page=3").unwrap();
        assert_eq!(opts.page, 3);
    }

    #[test]
    fn rejects_page_zero() {
        let options = FinishedGamesQueryOptions {
            page: 0,
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::PageOutOfRange)
        ));
    }

    #[test]
    fn rejects_page_over_max() {
        let options = FinishedGamesQueryOptions {
            page: 10001,
            ..base_options()
        };
        assert!(matches!(
            options.validate_all(),
            Err(errs) if errs.contains(&FinishedGameQueryValidationError::PageOutOfRange)
        ));
    }
}
