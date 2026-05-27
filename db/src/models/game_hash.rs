use crate::{db_error::DbError, schema::game_hashes, DbConn};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = game_hashes)]
pub struct GameHash {
    pub hash: i64,
    pub game_id: Uuid,
    pub turn: i32,
    pub rating: Option<f64>,
    pub result: String,
    pub speed: String,
    pub game_type: String,
    pub rated: bool,
    pub played_at: DateTime<Utc>,
}

pub struct GameFinishContext {
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub result: String,
    pub speed: String,
    pub game_type: String,
    pub rated: bool,
    pub played_at: DateTime<Utc>,
}

impl GameHash {
    pub fn from_engine_hashes(game_id: Uuid, hashes: &[u64], ctx: &GameFinishContext) -> Vec<Self> {
        hashes
            .iter()
            .enumerate()
            .map(|(turn, &h)| Self {
                hash: h as i64,
                game_id,
                turn: turn as i32,
                rating: if turn % 2 == 0 {
                    ctx.white_rating
                } else {
                    ctx.black_rating
                },
                result: ctx.result.clone(),
                speed: ctx.speed.clone(),
                game_type: ctx.game_type.clone(),
                rated: ctx.rated,
                played_at: ctx.played_at,
            })
            .collect()
    }

    pub async fn insert_batch(entries: &[GameHash], conn: &mut DbConn<'_>) -> Result<(), DbError> {
        if entries.is_empty() {
            return Ok(());
        }
        diesel::insert_into(game_hashes::table)
            .values(entries)
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn find_by_hash(
        hash: u64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<GameHash>, DbError> {
        Ok(game_hashes::table
            .filter(game_hashes::hash.eq(hash as i64))
            .load(conn)
            .await?)
    }

    pub async fn best(
        hash: u64,
        limit: Option<i64>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<GameHash>, DbError> {
        Ok(game_hashes::table
            .filter(game_hashes::hash.eq(hash as i64))
            .filter(game_hashes::rating.is_not_null())
            .order(game_hashes::rating.desc())
            .limit(limit.unwrap_or(10))
            .load(conn)
            .await?)
    }
}
