use crate::schema::{games, users};
use diesel::{prelude::*, sql_types, BoxableExpression, ExpressionMethods, QueryDsl};
use hive_lib::{Color, GameResult, GameStatus, GameType};
use shared_types::{
    BatchInfo, Conclusion, GameProgress, GameSpeed, GameStart, PlayerFilter, ResultType,
};

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

    pub fn progress(mut self, progress: &GameProgress) -> Self {
        self.query = match progress {
            GameProgress::Unstarted => self.query.filter(
                games::game_status
                    .eq(GameStatus::NotStarted.to_string())
                    .and(games::game_start.eq(GameStart::Ready.to_string()))
                    .and(games::conclusion.ne(Conclusion::Committee.to_string())),
            ),
            GameProgress::Playing => self.query.filter(
                games::game_status
                    .eq(GameStatus::InProgress.to_string())
                    .or(games::game_status
                        .eq(GameStatus::NotStarted.to_string())
                        .and(games::game_start.ne(GameStart::Ready.to_string()))),
            ),
            GameProgress::Finished => self.query.filter(games::finished.eq(true)),
            GameProgress::All => self.query,
        };
        self
    }

    pub fn speeds(mut self, speeds: &[GameSpeed]) -> Self {
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

    pub fn player_filters(
        mut self,
        player1: Option<&PlayerFilter>,
        player2: Option<&PlayerFilter>,
        exclude_bots: bool,
    ) -> Self {
        match (player1, player2) {
            (None, None) => {
                if exclude_bots {
                    self = self.apply_bot_exclusion();
                }
            }
            (Some(p1), None) | (None, Some(p1)) => {
                let condition = self.build_player_condition(&p1.username, p1.color, p1.result);
                self.query = self.query.filter(condition);

                if exclude_bots {
                    self = self.apply_bot_exclusion_for_player(&p1.username, p1.color);
                }
            }
            (Some(p1), Some(p2)) => {
                let condition1 = self.build_player_condition(&p1.username, p1.color, p1.result);
                let condition2 = self.build_player_condition(&p2.username, p2.color, p2.result);
                self.query = self.query.filter(condition1.and(condition2));
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

    pub fn build(self) -> games::BoxedQuery<'static, diesel::pg::Pg> {
        self.query
    }

    fn build_player_condition(
        &self,
        username: &str,
        color: Option<Color>,
        result: Option<ResultType>,
    ) -> Box<dyn BoxableExpression<games::table, diesel::pg::Pg, SqlType = sql_types::Bool>> {
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

    fn apply_bot_exclusion(mut self) -> Self {
        let bot_subquery = users::table.filter(users::bot.eq(true)).select(users::id);

        self.query = self.query.filter(
            games::white_id
                .ne_all(bot_subquery)
                .and(games::black_id.ne_all(bot_subquery)),
        );
        self
    }

    fn apply_bot_exclusion_for_player(mut self, username: &str, color: Option<Color>) -> Self {
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
}
