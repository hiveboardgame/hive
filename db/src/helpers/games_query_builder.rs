use crate::schema::{games, users};
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::expression::SqlLiteral;
use diesel::{prelude::*, sql_types, BoxableExpression, ExpressionMethods, QueryDsl};
use hive_lib::{Color, GameResult, GameStatus, GameType};
use shared_types::{
    BatchInfo, BatchToken, Conclusion, FinishedGameSort, FinishedGameSortKey,
    FinishedGamesQueryOptions, FinishedResultFilter, GameProgress, GameSpeed, GameStart,
    GamesQueryOptions, PlayerFilter, ResultType, SortValue, TimeMode,
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

    pub fn finished_base_query(options: &FinishedGamesQueryOptions) -> Self {
        GameQueryBuilder::new()
            .filters(options)
            .apply_finished_gate()
    }

    pub fn finished_batch_query(options: &FinishedGamesQueryOptions) -> Self {
        GameQueryBuilder::finished_base_query(options)
            .sort(&options.sort)
            .keyset(&options.sort, options.batch_token.as_ref())
            .limit(options.batch_size)
    }

    pub fn apply_finished_options(self, options: &FinishedGamesQueryOptions) -> Self {
        GameQueryBuilder::finished_batch_query(options)
    }

    pub fn filters(mut self, options: &FinishedGamesQueryOptions) -> Self {
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
            .time_mode(options.time_mode)
            .rating_range(options.rating_min, options.rating_max)
            .turn_range(options.turn_min, options.turn_max)
            .date_range(options.date_start, options.date_end)
            .tournament_filter(options.only_tournament);
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
                self.query = self.query.filter(games::finished.eq(true));
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

    pub fn legacy_speeds(mut self, speeds: &[GameSpeed]) -> Self {
        if !speeds.is_empty() && speeds.len() != GameSpeed::all_games().len() {
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

    pub fn time_mode(mut self, time_mode: Option<TimeMode>) -> Self {
        if let Some(mode) = time_mode {
            self.query = self.query.filter(games::time_mode.eq(mode.to_string()));
        }
        self
    }

    pub fn apply_legacy_options(mut self, options: &GamesQueryOptions) -> Self {
        self = self
            .legacy_player_filters(
                options.player1.as_ref(),
                options.player2.as_ref(),
                options.exclude_bots,
            )
            .scope(options.game_progress)
            .legacy_speeds(&options.speeds)
            .expansions(options.expansions)
            .rated_filter(options.rated)
            .paginate(options.current_batch.as_ref(), options.batch_size);
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
        result_filter: &FinishedResultFilter,
        player1: Option<&str>,
        player2: Option<&str>,
        fixed_colors: bool,
    ) -> Self {
        match result_filter {
            FinishedResultFilter::Any => self,
            FinishedResultFilter::ColorWins(color) => {
                let status = self.winner_status(*color);
                self.query = self.query.filter(status);
                self
            }
            FinishedResultFilter::Draw => {
                let status = self.draw_status();
                self.query = self.query.filter(status);
                self
            }
            FinishedResultFilter::NotDraw => {
                let white_won = self.winner_status(Color::White);
                let black_won = self.winner_status(Color::Black);
                let condition: GamePredicate = Box::new(white_won.or(black_won));
                self.query = self.query.filter(condition);
                self
            }
            FinishedResultFilter::PlayerWins(slot) => {
                if let Some(condition) =
                    self.winner_for_players(*slot, player1, player2, fixed_colors)
                {
                    self.query = self.query.filter(condition);
                }
                self
            }
        }
    }

    pub fn legacy_player_filters(
        mut self,
        player1: Option<&PlayerFilter>,
        player2: Option<&PlayerFilter>,
        exclude_bots: bool,
    ) -> Self {
        match (player1, player2) {
            (None, None) => {
                if exclude_bots {
                    self = self.legacy_apply_bot_exclusion();
                }
            }
            (Some(p1), None) | (None, Some(p1)) => {
                let condition =
                    self.legacy_build_player_condition(&p1.username, p1.color, p1.result);
                self.query = self.query.filter(condition);

                if exclude_bots {
                    self = self.legacy_apply_bot_exclusion_for_player(&p1.username, p1.color);
                }
            }
            (Some(p1), Some(p2)) => {
                let condition1 =
                    self.legacy_build_player_condition(&p1.username, p1.color, p1.result);
                let condition2 =
                    self.legacy_build_player_condition(&p2.username, p2.color, p2.result);
                self.query = self.query.filter(condition1.and(condition2));
            }
        }
        self
    }

    fn legacy_build_player_condition(
        &self,
        username: &str,
        color: Option<Color>,
        result: Option<ResultType>,
    ) -> GamePredicate {
        let user_subquery = users::table
            .filter(users::normalized_username.eq(username.to_lowercase()))
            .select(users::id);

        let white_won = games::game_status
            .eq(GameStatus::Finished(GameResult::Winner(Color::White)).to_string());
        let black_won = games::game_status
            .eq(GameStatus::Finished(GameResult::Winner(Color::Black)).to_string());
        let is_draw = games::game_status.eq(GameStatus::Finished(GameResult::Draw).to_string());

        let is_white = games::white_id.eq_any(user_subquery.clone());
        let is_black = games::black_id.eq_any(user_subquery);

        match (result, color) {
            (Some(ResultType::Win), Some(Color::White)) => Box::new(is_white.and(white_won)),
            (Some(ResultType::Win), Some(Color::Black)) => Box::new(is_black.and(black_won)),
            (Some(ResultType::Win), None) => Box::new(
                is_white
                    .clone()
                    .and(white_won)
                    .or(is_black.clone().and(black_won)),
            ),

            (Some(ResultType::Loss), Some(Color::White)) => Box::new(is_white.and(black_won)),
            (Some(ResultType::Loss), Some(Color::Black)) => Box::new(is_black.and(white_won)),
            (Some(ResultType::Loss), None) => Box::new(
                is_white
                    .clone()
                    .and(black_won)
                    .or(is_black.clone().and(white_won)),
            ),

            (Some(ResultType::Draw), Some(Color::White)) => Box::new(is_white.and(is_draw)),
            (Some(ResultType::Draw), Some(Color::Black)) => Box::new(is_black.and(is_draw)),
            (Some(ResultType::Draw), None) => Box::new(
                is_white
                    .clone()
                    .and(is_draw.clone())
                    .or(is_black.clone().and(is_draw)),
            ),

            (None, Some(Color::White)) => Box::new(is_white),
            (None, Some(Color::Black)) => Box::new(is_black),
            (None, None) => Box::new(is_white.or(is_black)),
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
            self.query = self.query.filter(games::updated_at.le(end_date));
        }
        self
    }

    pub fn tournament_filter(mut self, only_tournament: bool) -> Self {
        if only_tournament {
            self.query = self.query.filter(games::tournament_id.is_not_null());
        }
        self
    }

    pub fn sort(mut self, sort: &FinishedGameSort) -> Self {
        let asc = sort.ascending;
        self.query = match sort.key {
            FinishedGameSortKey::Date => {
                if asc {
                    self.query
                        .order_by((games::updated_at.asc(), games::id.asc()))
                } else {
                    self.query
                        .order_by((games::updated_at.desc(), games::id.desc()))
                }
            }
            FinishedGameSortKey::Turns => {
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
            FinishedGameSortKey::RatingAvg => {
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

    pub fn keyset(mut self, sort: &FinishedGameSort, token: Option<&BatchToken>) -> Self {
        if let Some(batch) = token {
            if let Some(condition) = self.keyset_condition(sort, batch) {
                self.query = self.query.filter(condition);
            }
        }
        self
    }

    pub fn paginate(mut self, batch: Option<&BatchInfo>, size: usize) -> Self {
        self.query = self
            .query
            .order_by((games::updated_at.desc(), games::id.desc()));
        if let Some(batch) = batch {
            self.query = self.query.filter(
                games::updated_at.lt(batch.timestamp).or(games::updated_at
                    .eq(batch.timestamp)
                    .and(games::id.ne(batch.id))),
            );
        }
        self.query = self.query.limit(size as i64);
        self
    }

    pub fn limit(mut self, size: usize) -> Self {
        self.query = self.query.limit(size as i64);
        self
    }

    pub fn build(self) -> games::BoxedQuery<'static, diesel::pg::Pg> {
        self.query
    }

    fn rating_average_expr() -> SqlLiteral<sql_types::Double> {
        sql::<sql_types::Double>("(white_rating + black_rating) / 2.0")
    }

    fn apply_finished_gate(mut self) -> Self {
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
        self
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

    fn legacy_apply_bot_exclusion(mut self) -> Self {
        let bot_subquery = users::table.filter(users::bot.eq(true)).select(users::id);

        self.query = self.query.filter(
            games::white_id
                .ne_all(bot_subquery)
                .and(games::black_id.ne_all(bot_subquery)),
        );
        self
    }

    fn legacy_apply_bot_exclusion_for_player(
        mut self,
        username: &str,
        color: Option<Color>,
    ) -> Self {
        let bot_subquery = users::table.filter(users::bot.eq(true)).select(users::id);

        let user_subquery = users::table
            .filter(users::normalized_username.eq(username.to_lowercase()))
            .select(users::id);

        match color {
            Some(Color::White) => {
                self.query = self.query.filter(games::black_id.ne_all(bot_subquery));
            }
            Some(Color::Black) => {
                self.query = self.query.filter(games::white_id.ne_all(bot_subquery));
            }
            None => {
                self.query = self.query.filter(
                    (games::white_id
                        .eq_any(user_subquery.clone())
                        .and(games::black_id.ne_all(bot_subquery)))
                    .or(games::black_id
                        .eq_any(user_subquery)
                        .and(games::white_id.ne_all(bot_subquery))),
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

    fn keyset_condition(
        &self,
        sort: &FinishedGameSort,
        token: &BatchToken,
    ) -> Option<GamePredicate> {
        match sort.key {
            FinishedGameSortKey::Date => {
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
            FinishedGameSortKey::Turns => {
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
            FinishedGameSortKey::RatingAvg => {
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
    use shared_types::{FinishedGamesQueryOptions, FinishedResultFilter};

    #[test]
    fn finished_scope_applies_gate() {
        let prepared = FinishedGamesQueryOptions::default().validate_all().unwrap();
        let query = GameQueryBuilder::new()
            .apply_finished_options(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("finished"));
        assert!(sql.contains("game_status"));
        assert!(sql.contains("conclusion"));
    }

    #[test]
    fn player_loss_without_color_filters_winner() {
        let options = FinishedGamesQueryOptions {
            player1: Some("player".into()),
            result_filter: FinishedResultFilter::PlayerWins(2),
            ..FinishedGamesQueryOptions::default()
        };
        let prepared = options.validate_all().unwrap();
        let query = GameQueryBuilder::new()
            .apply_finished_options(&prepared)
            .build()
            .select(games::all_columns);

        let sql = debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(sql.contains("game_status"));
        assert!(sql.contains("white_id"));
        assert!(sql.contains("black_id"));
    }

    #[test]
    fn rating_range_requires_present_ratings() {
        let options = FinishedGamesQueryOptions {
            rating_min: Some(1200),
            ..FinishedGamesQueryOptions::default()
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
    fn tournament_filter_limits_to_tournament_games() {
        let options = FinishedGamesQueryOptions {
            only_tournament: true,
            ..FinishedGamesQueryOptions::default()
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
