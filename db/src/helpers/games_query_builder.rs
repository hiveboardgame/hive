use crate::schema::{game_hashes, games, users};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::sql,
    expression::SqlLiteral,
    prelude::*,
    sql_types,
    BoxableExpression,
    ExpressionMethods,
    QueryDsl,
};
use hudsoni::{Color, GameResult, GameStatus, GameType};
use shared_types::{
    BatchToken,
    Conclusion,
    GameProgress,
    GameSort,
    GameSortKey,
    GameSpeed,
    GameStart,
    GamesQueryOptions,
    ResultFilter,
    SortValue,
};

type GamePredicate =
    Box<dyn BoxableExpression<games::table, diesel::pg::Pg, SqlType = sql_types::Bool>>;

pub struct GameQueryBuilder {
    query: games::BoxedQuery<'static, diesel::pg::Pg>,
}

impl Default for GameQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GameQueryBuilder {
    pub fn new() -> Self {
        Self {
            query: games::table.into_boxed(),
        }
    }

    pub fn base_query(options: &GamesQueryOptions) -> Self {
        GameQueryBuilder::new().filters(options)
    }

    pub fn batch_query(options: &GamesQueryOptions) -> Self {
        let base = GameQueryBuilder::base_query(options).sort(&options.sort);
        match &options.batch_token {
            Some(token) => base
                .keyset(&options.sort, Some(token))
                .limit(options.batch_size),
            None => base
                .offset(((options.page - 1) * options.batch_size) as i64)
                .limit(options.batch_size),
        }
    }

    /// Query used for the total count. Mirrors the row-eligibility filters the
    /// page query applies via `sort()` so the count matches the rows that can
    /// actually appear (RatingAvg sorting drops games with NULL ratings).
    pub fn count_query(options: &GamesQueryOptions) -> Self {
        let builder = GameQueryBuilder::base_query(options);
        if options.sort.key == GameSortKey::RatingAvg {
            builder.ensure_ratings_present()
        } else {
            builder
        }
    }

    pub fn filters(mut self, options: &GamesQueryOptions) -> Self {
        self = self
            .scope(options.game_progress)
            .player_filters(
                options.player1.as_deref(),
                options.player2.as_deref(),
                options.fixed_colors,
                options.exclude_bots,
            )
            .result_filter(
                &options.result_filter,
                options.player1.as_deref(),
                options.player2.as_deref(),
                options.fixed_colors,
            )
            .speeds(&options.speeds)
            .expansions(options.expansions)
            .rated_filter(options.rated)
            .rating_range(options.rating_min, options.rating_max)
            .turn_range(options.turn_min, options.turn_max)
            .date_range(options.date_start, options.date_end)
            .tournament_filter(options.only_tournament)
            .position_hash(options.position_hash);
        self
    }

    /// Restrict to games that passed through the given canonical position hash, via a subquery
    /// on `game_hashes` (one row per turn/position, FK `game_id` → `games.id`). Used by the
    /// opening explorer's "Search this position" link.
    pub fn position_hash(mut self, hash: Option<i64>) -> Self {
        if let Some(hash) = hash {
            self.query = self.query.filter(
                games::id.eq_any(
                    game_hashes::table
                        .filter(game_hashes::hash.eq(hash))
                        .select(game_hashes::game_id),
                ),
            );
        }
        self
    }

    pub fn scope(mut self, progress: GameProgress) -> Self {
        match progress {
            GameProgress::Unstarted => {
                self.query = self.query.filter(
                    games::game_status
                        .eq(GameStatus::NotStarted.to_string())
                        .and(games::game_start.eq(GameStart::Ready.to_string()))
                        .and(games::conclusion.ne(Conclusion::Committee.to_string())),
                );
            }
            GameProgress::Playing => {
                self.query = self.query.filter(
                    games::game_status
                        .eq(GameStatus::InProgress.to_string())
                        .or(games::game_status
                            .eq(GameStatus::NotStarted.to_string())
                            .and(games::game_start.ne(GameStart::Ready.to_string()))),
                );
            }
            GameProgress::Finished => {
                let finished_values = vec![
                    GameStatus::Finished(GameResult::Winner(Color::White)).to_string(),
                    GameStatus::Finished(GameResult::Winner(Color::Black)).to_string(),
                    GameStatus::Finished(GameResult::Draw).to_string(),
                ];
                let allowed_conclusions = vec![
                    Conclusion::Board.to_string(),
                    Conclusion::Resigned.to_string(),
                    Conclusion::Timeout.to_string(),
                    Conclusion::Draw.to_string(),
                    Conclusion::Repetition.to_string(),
                ];
                self.query = self.query.filter(games::finished.eq(true));
                self.query = self
                    .query
                    .filter(games::game_status.eq_any(finished_values));
                self.query = self
                    .query
                    .filter(games::conclusion.eq_any(allowed_conclusions));
            }
            GameProgress::All => {}
        }
        self
    }

    pub fn speeds(mut self, speeds: &[GameSpeed]) -> Self {
        if !speeds.is_empty() {
            let speed_strings: Vec<String> = speeds.iter().map(|s| s.to_string()).collect();
            self.query = self.query.filter(games::speed.eq_any(speed_strings));
        }
        self
    }

    pub fn expansions(mut self, expansions: Option<bool>) -> Self {
        if let Some(has_expansions) = expansions {
            let game_type_str = if has_expansions {
                GameType::MLP.to_string()
            } else {
                GameType::Base.to_string()
            };
            self.query = self.query.filter(games::game_type.eq(game_type_str));
        }
        self
    }

    pub fn rated_filter(mut self, rated_option: Option<bool>) -> Self {
        if let Some(is_rated) = rated_option {
            self.query = self.query.filter(games::rated.eq(is_rated));
        }
        self
    }

    pub fn player_filters(
        mut self,
        player1: Option<&str>,
        player2: Option<&str>,
        fixed_colors: bool,
        exclude_bots: bool,
    ) -> Self {
        match (player1, player2) {
            (None, None) => {
                if exclude_bots {
                    self = self.apply_bot_exclusion();
                }
            }
            (Some(p1), None) => {
                let condition = if fixed_colors {
                    self.player_in_color(p1, Color::White)
                } else {
                    self.player_in_any_color(p1)
                };
                self.query = self.query.filter(condition);

                if exclude_bots {
                    self = self.apply_bot_exclusion_for_player(
                        p1,
                        if fixed_colors {
                            Some(Color::White)
                        } else {
                            None
                        },
                    );
                }
            }
            (None, Some(p2)) => {
                let condition = if fixed_colors {
                    self.player_in_color(p2, Color::Black)
                } else {
                    self.player_in_any_color(p2)
                };
                self.query = self.query.filter(condition);

                if exclude_bots {
                    self = self.apply_bot_exclusion_for_player(
                        p2,
                        if fixed_colors {
                            Some(Color::Black)
                        } else {
                            None
                        },
                    );
                }
            }
            (Some(p1), Some(p2)) => {
                let condition: GamePredicate = if fixed_colors {
                    Box::new(
                        self.player_in_color(p1, Color::White)
                            .and(self.player_in_color(p2, Color::Black)),
                    )
                } else {
                    Box::new(
                        self.player_in_color(p1, Color::White)
                            .and(self.player_in_color(p2, Color::Black))
                            .or(self
                                .player_in_color(p1, Color::Black)
                                .and(self.player_in_color(p2, Color::White))),
                    )
                };
                self.query = self.query.filter(condition);
            }
        }
        self
    }

    pub fn result_filter(
        mut self,
        result_filter: &ResultFilter,
        player1: Option<&str>,
        player2: Option<&str>,
        fixed_colors: bool,
    ) -> Self {
        match result_filter {
            ResultFilter::Any => self,
            ResultFilter::ColorWins(color) => {
                let status = self.winner_status(*color);
                self.query = self.query.filter(status);
                self
            }
            ResultFilter::Draw => {
                let status = self.draw_status();
                self.query = self.query.filter(status);
                self
            }
            ResultFilter::NotDraw => {
                let white_won = self.winner_status(Color::White);
                let black_won = self.winner_status(Color::Black);
                let condition: GamePredicate = Box::new(white_won.or(black_won));
                self.query = self.query.filter(condition);
                self
            }
            ResultFilter::PlayerWins(slot) => {
                if let Some(condition) =
                    self.winner_for_players(*slot, player1, player2, fixed_colors)
                {
                    self.query = self.query.filter(condition);
                }
                self
            }
            ResultFilter::PlayerLoses(slot) => {
                if let Some(condition) =
                    self.loser_for_players(*slot, player1, player2, fixed_colors)
                {
                    self.query = self.query.filter(condition);
                }
                self
            }
        }
    }

    pub fn rating_range(mut self, min: Option<i32>, max: Option<i32>) -> Self {
        if min.is_some() || max.is_some() {
            self.query = self.query.filter(
                games::white_rating
                    .is_not_null()
                    .and(games::black_rating.is_not_null()),
            );
            if let Some(min_value) = min {
                self.query = self
                    .query
                    .filter(Self::rating_average_expr().ge(min_value as f64));
            }
            if let Some(max_value) = max {
                self.query = self
                    .query
                    .filter(Self::rating_average_expr().le(max_value as f64));
            }
        }
        self
    }

    pub fn turn_range(mut self, min: Option<i32>, max: Option<i32>) -> Self {
        if let Some(min_turn) = min {
            self.query = self.query.filter(games::turn.ge(min_turn));
        }
        if let Some(max_turn) = max {
            self.query = self.query.filter(games::turn.le(max_turn));
        }
        self
    }

    pub fn date_range(mut self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self {
        if let Some(start_date) = start {
            self.query = self.query.filter(games::updated_at.ge(start_date));
        }
        if let Some(end_date) = end {
            // date_end is parsed at 00:00:00 of the chosen day; use an
            // exclusive next-day bound so games later that day are included.
            self.query = self
                .query
                .filter(games::updated_at.lt(end_date + chrono::Duration::days(1)));
        }
        self
    }

    pub fn tournament_filter(mut self, only_tournament: bool) -> Self {
        if only_tournament {
            self.query = self.query.filter(games::tournament_id.is_not_null());
        }
        self
    }

    pub fn sort(mut self, sort: &GameSort) -> Self {
        let asc = sort.ascending;
        self.query = match sort.key {
            GameSortKey::Date => {
                if asc {
                    self.query
                        .order_by((games::updated_at.asc(), games::id.asc()))
                } else {
                    self.query
                        .order_by((games::updated_at.desc(), games::id.desc()))
                }
            }
            GameSortKey::Turns => {
                if asc {
                    self.query.order_by((
                        games::turn.asc(),
                        games::updated_at.desc(),
                        games::id.desc(),
                    ))
                } else {
                    self.query.order_by((
                        games::turn.desc(),
                        games::updated_at.desc(),
                        games::id.desc(),
                    ))
                }
            }
            GameSortKey::RatingAvg => {
                self = self.ensure_ratings_present();
                let average = Self::rating_average_expr();
                if asc {
                    self.query
                        .order_by((average.asc(), games::updated_at.desc(), games::id.desc()))
                } else {
                    self.query.order_by((
                        average.desc(),
                        games::updated_at.desc(),
                        games::id.desc(),
                    ))
                }
            }
        };
        self
    }

    pub fn keyset(mut self, sort: &GameSort, token: Option<&BatchToken>) -> Self {
        if let Some(batch) = token {
            if let Some(condition) = self.keyset_condition(sort, batch) {
                self.query = self.query.filter(condition);
            }
        }
        self
    }

    pub fn limit(mut self, size: usize) -> Self {
        self.query = self.query.limit(size as i64);
        self
    }

    pub fn offset(mut self, n: i64) -> Self {
        self.query = self.query.offset(n);
        self
    }

    pub fn build(self) -> games::BoxedQuery<'static, diesel::pg::Pg> {
        self.query
    }

    fn rating_average_expr() -> SqlLiteral<sql_types::Double> {
        sql::<sql_types::Double>("(white_rating + black_rating) / 2.0")
    }

    fn winner_status(&self, color: Color) -> GamePredicate {
        Box::new(games::game_status.eq(GameStatus::Finished(GameResult::Winner(color)).to_string()))
    }

    fn draw_status(&self) -> GamePredicate {
        Box::new(games::game_status.eq(GameStatus::Finished(GameResult::Draw).to_string()))
    }

    /// player must already be normalized (lowercase); callers use options from validate_all()
    fn player_in_color(&self, player: &str, color: Color) -> GamePredicate {
        let username = player.to_string();
        let user_ids = users::table
            .filter(users::normalized_username.eq(username))
            .select(users::id);
        match color {
            Color::White => Box::new(games::white_id.eq_any(user_ids)),
            Color::Black => Box::new(games::black_id.eq_any(user_ids)),
        }
    }

    fn player_in_any_color(&self, player: &str) -> GamePredicate {
        let white = self.player_in_color(player, Color::White);
        let black = self.player_in_color(player, Color::Black);
        Box::new(white.or(black))
    }

    fn apply_bot_exclusion(mut self) -> Self {
        let bot_subquery_white = users::table.filter(users::bot.eq(true)).select(users::id);
        let bot_subquery_black = users::table.filter(users::bot.eq(true)).select(users::id);

        self.query = self.query.filter(
            games::white_id
                .ne_all(bot_subquery_white)
                .and(games::black_id.ne_all(bot_subquery_black)),
        );
        self
    }

    fn apply_bot_exclusion_for_player(mut self, player: &str, seat: Option<Color>) -> Self {
        let username = player.to_string();
        match seat {
            Some(Color::White) => {
                let bots = users::table.filter(users::bot.eq(true)).select(users::id);
                self.query = self.query.filter(games::black_id.ne_all(bots));
            }
            Some(Color::Black) => {
                let bots = users::table.filter(users::bot.eq(true)).select(users::id);
                self.query = self.query.filter(games::white_id.ne_all(bots));
            }
            None => {
                let bot_subquery_black = users::table.filter(users::bot.eq(true)).select(users::id);
                let bot_subquery_white = users::table.filter(users::bot.eq(true)).select(users::id);
                let user_white = users::table
                    .filter(users::normalized_username.eq(username.clone()))
                    .select(users::id);
                let user_black = users::table
                    .filter(users::normalized_username.eq(username))
                    .select(users::id);

                self.query = self.query.filter(
                    (games::white_id
                        .eq_any(user_white)
                        .and(games::black_id.ne_all(bot_subquery_black)))
                    .or(games::black_id
                        .eq_any(user_black)
                        .and(games::white_id.ne_all(bot_subquery_white))),
                );
            }
        }
        self
    }

    fn ensure_ratings_present(mut self) -> Self {
        self.query = self.query.filter(
            games::white_rating
                .is_not_null()
                .and(games::black_rating.is_not_null()),
        );
        self
    }

    fn keyset_condition(&self, sort: &GameSort, token: &BatchToken) -> Option<GamePredicate> {
        match sort.key {
            GameSortKey::Date => {
                if sort.ascending {
                    Some(Box::new(
                        games::updated_at.gt(token.updated_at).or(games::updated_at
                            .eq(token.updated_at)
                            .and(games::id.gt(token.id))),
                    ))
                } else {
                    Some(Box::new(
                        games::updated_at.lt(token.updated_at).or(games::updated_at
                            .eq(token.updated_at)
                            .and(games::id.lt(token.id))),
                    ))
                }
            }
            GameSortKey::Turns => {
                let SortValue::Turns(turn_value) = token.primary_value else {
                    return None;
                };
                let secondary_desc = games::updated_at.lt(token.updated_at).or(games::updated_at
                    .eq(token.updated_at)
                    .and(games::id.lt(token.id)));

                if sort.ascending {
                    Some(Box::new(
                        games::turn
                            .gt(turn_value)
                            .or(games::turn.eq(turn_value).and(secondary_desc)),
                    ))
                } else {
                    Some(Box::new(
                        games::turn
                            .lt(turn_value)
                            .or(games::turn.eq(turn_value).and(secondary_desc)),
                    ))
                }
            }
            GameSortKey::RatingAvg => {
                let SortValue::RatingAvg(primary_value) = token.primary_value else {
                    return None;
                };
                let secondary_desc = games::updated_at.lt(token.updated_at).or(games::updated_at
                    .eq(token.updated_at)
                    .and(games::id.lt(token.id)));

                if sort.ascending {
                    Some(Box::new(
                        Self::rating_average_expr().gt(primary_value).or(
                            Self::rating_average_expr()
                                .eq(primary_value)
                                .and(secondary_desc),
                        ),
                    ))
                } else {
                    Some(Box::new(
                        Self::rating_average_expr().lt(primary_value).or(
                            Self::rating_average_expr()
                                .eq(primary_value)
                                .and(secondary_desc),
                        ),
                    ))
                }
            }
        }
    }

    fn winner_for_players(
        &self,
        slot: u8,
        player1: Option<&str>,
        player2: Option<&str>,
        fixed_colors: bool,
    ) -> Option<GamePredicate> {
        match slot {
            1 => {
                let winner = player1?;
                let winner_color = fixed_colors.then_some(Color::White);
                let predicates = self.player_win_assignments(winner, player2, winner_color);
                Self::any_of(predicates)
            }
            2 => match (player2, player1) {
                (Some(winner), opponent) => {
                    let winner_color = fixed_colors.then_some(Color::Black);
                    let predicates = self.player_win_assignments(winner, opponent, winner_color);
                    Self::any_of(predicates)
                }
                (None, Some(opponent)) => {
                    let mut predicates: Vec<GamePredicate> = vec![Box::new(
                        self.player_in_color(opponent, Color::White)
                            .and(self.winner_status(Color::Black)),
                    )];
                    if !fixed_colors {
                        predicates.push(Box::new(
                            self.player_in_color(opponent, Color::Black)
                                .and(self.winner_status(Color::White)),
                        ));
                    }
                    Self::any_of(predicates)
                }
                (None, None) => None,
            },
            _ => None,
        }
    }

    fn loser_for_players(
        &self,
        slot: u8,
        player1: Option<&str>,
        player2: Option<&str>,
        fixed_colors: bool,
    ) -> Option<GamePredicate> {
        match slot {
            1 => {
                let loser = player1?;
                let loser_color = fixed_colors.then_some(Color::White);
                let predicates = self.player_loss_assignments(loser, player2, loser_color);
                Self::any_of(predicates)
            }
            2 => match (player2, player1) {
                (Some(loser), opponent) => {
                    let loser_color = fixed_colors.then_some(Color::Black);
                    let predicates = self.player_loss_assignments(loser, opponent, loser_color);
                    Self::any_of(predicates)
                }
                (None, Some(opponent)) => {
                    let opponent_color = fixed_colors.then_some(Color::White);
                    let predicates = self.player_win_assignments(opponent, None, opponent_color);
                    Self::any_of(predicates)
                }
                (None, None) => None,
            },
            _ => None,
        }
    }

    fn player_win_assignments(
        &self,
        winner: &str,
        opponent: Option<&str>,
        fixed_color: Option<Color>,
    ) -> Vec<GamePredicate> {
        let seats = match fixed_color {
            Some(color) => vec![color],
            None => vec![Color::White, Color::Black],
        };

        seats
            .into_iter()
            .map(|winner_color| {
                let mut predicate: GamePredicate = Box::new(
                    self.player_in_color(winner, winner_color)
                        .and(self.winner_status(winner_color)),
                );
                if let Some(opponent_name) = opponent {
                    predicate =
                        Box::new(predicate.and(
                            self.player_in_color(opponent_name, winner_color.opposite_color()),
                        ));
                }
                predicate
            })
            .collect()
    }

    fn player_loss_assignments(
        &self,
        loser: &str,
        opponent: Option<&str>,
        fixed_color: Option<Color>,
    ) -> Vec<GamePredicate> {
        let seats = match fixed_color {
            Some(color) => vec![color],
            None => vec![Color::White, Color::Black],
        };

        seats
            .into_iter()
            .map(|loser_color| {
                let opponent_color = loser_color.opposite_color();
                let mut predicate: GamePredicate = Box::new(
                    self.player_in_color(loser, loser_color)
                        .and(self.winner_status(opponent_color)),
                );
                if let Some(opponent_name) = opponent {
                    predicate = Box::new(
                        predicate.and(self.player_in_color(opponent_name, opponent_color)),
                    );
                }
                predicate
            })
            .collect()
    }

    fn any_of(predicates: Vec<GamePredicate>) -> Option<GamePredicate> {
        let mut iter = predicates.into_iter();
        let first = iter.next()?;
        Some(iter.fold(first, |acc, predicate| Box::new(acc.or(predicate))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::games;
    use diesel::debug_query;
    use shared_types::{GameProgress, GamesQueryOptions, ResultFilter};

    fn finished_defaults() -> GamesQueryOptions {
        GamesQueryOptions {
            game_progress: GameProgress::Finished,
            ..GamesQueryOptions::default()
        }
    }

    #[test]
    fn finished_scope_applies_gate() {
        let prepared = finished_defaults().validate_all().unwrap();
        let query = GameQueryBuilder::base_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("finished"));
        assert!(sql.contains("game_status"));
        assert!(sql.contains("conclusion"));
    }

    #[test]
    fn position_hash_filters_via_game_hashes_subquery() {
        let options = GamesQueryOptions {
            position_hash: Some(123),
            ..finished_defaults()
        };
        let prepared = options.clone().validate_all().unwrap();
        let query = GameQueryBuilder::base_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("game_hashes"));
        assert!(sql.contains("hash"));

        // No subquery when unset.
        let none = finished_defaults().validate_all().unwrap();
        let none_sql = debug_query::<diesel::pg::Pg, _>(
            &GameQueryBuilder::base_query(&none)
                .build()
                .select(games::all_columns),
        )
        .to_string();
        assert!(!none_sql.contains("game_hashes"));
    }

    #[test]
    fn player_loss_without_color_filters_winner() {
        let options = GamesQueryOptions {
            player1: Some("player".into()),
            result_filter: ResultFilter::PlayerWins(2),
            ..finished_defaults()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::base_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("game_status"));
        assert!(sql.contains("white_id"));
        assert!(sql.contains("black_id"));
    }

    #[test]
    fn player_loses_emits_winner_status_for_opposite_color() {
        let options = GamesQueryOptions {
            player1: Some("player".into()),
            result_filter: ResultFilter::PlayerLoses(1),
            ..finished_defaults()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::base_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("game_status"));
        assert!(sql.contains("white_id"));
        assert!(sql.contains("black_id"));
    }

    #[test]
    fn rating_range_requires_present_ratings() {
        let options = GamesQueryOptions {
            rating_min: Some(1200),
            rated: Some(true),
            speeds: GameSpeed::real_time_speeds(),
            ..finished_defaults()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::new()
            .filters(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query)
            .to_string()
            .to_lowercase();
        assert!(sql.contains("white_rating"));
        assert!(sql.contains("black_rating"));
        assert!(sql.contains("not null"));
    }

    #[test]
    fn rating_avg_count_query_requires_present_ratings() {
        let options = GamesQueryOptions {
            sort: GameSort {
                key: GameSortKey::RatingAvg,
                ascending: false,
            },
            ..finished_defaults()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::count_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query)
            .to_string()
            .to_lowercase();
        assert!(sql.contains("white_rating"));
        assert!(sql.contains("black_rating"));
        assert!(sql.contains("not null"));
    }

    #[test]
    fn date_count_query_omits_rating_presence_filter() {
        // Default (Date) sort must not add the rating-presence WHERE clause;
        // white_rating still appears in the column list, so assert on the
        // NULL check that ensure_ratings_present() would introduce.
        let prepared = finished_defaults().validate_all().unwrap();
        let query = GameQueryBuilder::count_query(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query)
            .to_string()
            .to_lowercase();
        assert!(!sql.contains("not null"));
    }

    #[test]
    fn tournament_filter_limits_to_tournament_games() {
        let options = GamesQueryOptions {
            only_tournament: true,
            ..finished_defaults()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::new()
            .filters(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query)
            .to_string()
            .to_lowercase();
        assert!(sql.contains("tournament_id"));
        assert!(sql.contains("is not null"));
    }
}
