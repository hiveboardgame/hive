use super::Game;
use crate::schema::schedules;
use crate::DbConn;
use crate::{db_error::DbError, schema::games};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use shared_types::GameId;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = schedules)]
pub struct NewSchedule {
    game_id: Uuid,
    tournament_id: Uuid,
    proposer_id: Uuid,
    start_t: DateTime<Utc>,
    opponent_id: Uuid,
    agreed: bool,
}

impl NewSchedule {
    pub async fn new(
        user_id: Uuid,
        game_id: &GameId,
        start_t: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        let game = Game::find_by_game_id(game_id, conn).await?;
        if !game.user_is_player(user_id) {
            return Err(DbError::Unauthorized);
        }
        if game.tournament_id.is_none() {
            return Err(DbError::InvalidInput {
                info: String::from("NewSchedule::new failed"),
                error: String::from("Tournament id was None"),
            });
        }
        let opponent_id = if game.white_id == user_id {
            game.black_id
        } else {
            game.white_id
        };
        Ok(Self {
            game_id: game.id,
            tournament_id: game.tournament_id.expect("Game has tournament_id"),
            proposer_id: user_id,
            start_t,
            opponent_id,
            agreed: false,
        })
    }
}

#[derive(
    Queryable, Identifiable, Serialize, Clone, Deserialize, Debug, AsChangeset, Selectable,
)]
#[diesel(table_name = schedules)]
#[diesel(primary_key(id))]
pub struct Schedule {
    pub id: Uuid,
    pub game_id: Uuid,
    pub tournament_id: Uuid,
    pub proposer_id: Uuid,
    pub opponent_id: Uuid,
    pub start_t: DateTime<Utc>,
    pub agreed: bool,
}

impl Schedule {
    pub async fn accept(&mut self, user_id: Uuid, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        if self.opponent_id != user_id {
            return Err(DbError::Unauthorized);
        }
        //unset all schedules for this game
        diesel::update(schedules::table.filter(schedules::game_id.eq(self.game_id)))
            .set(schedules::agreed.eq(false))
            .execute(conn)
            .await?;
        //set this schedule only
        let ret = Ok(diesel::update(schedules::table.find(self.id))
            .set(schedules::agreed.eq(true))
            .execute(conn)
            .await?);
        self.agreed = true;
        ret
    }

    pub async fn create(
        schedule: NewSchedule,
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        if schedule.proposer_id != user_id || schedule.opponent_id == user_id {
            return Err(DbError::Unauthorized);
        }
        Ok(schedule
            .insert_into(schedules::table)
            .get_result(conn)
            .await?)
    }

    pub async fn cancel(&mut self, user_id: Uuid, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        if !self.is_player(user_id) {
            return Err(DbError::Unauthorized);
        }
        Ok(diesel::delete(schedules::table.find(self.id))
            .execute(conn)
            .await?)
    }

    pub async fn get_first_agreed(game_id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(schedules::table
            .filter(
                schedules::game_id
                    .eq(game_id)
                    .and(schedules::agreed.eq(true)),
            )
            .first(conn)
            .await?)
    }

    pub async fn from_id(id: Uuid, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        Ok(schedules::table.find(id).get_result(conn).await?)
    }

    pub async fn all_from_nanoid(
        game_id: String,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(schedules::table
            .inner_join(games::table)
            .filter(games::nanoid.eq(game_id))
            .select(schedules::all_columns)
            .get_results(conn)
            .await?)
    }

    fn is_player(&self, user_id: Uuid) -> bool {
        user_id == self.proposer_id || user_id == self.opponent_id
    }
}
