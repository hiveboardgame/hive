use crate::{db_error::DbError, schema::arena_game_results, DbConn};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tournamint::{arena::ArenaGameAward, Score};
use uuid::Uuid;

#[derive(Clone, Debug, Insertable, Queryable, Selectable)]
#[diesel(table_name = arena_game_results)]
pub(crate) struct ArenaGameResult {
    pub game_id: Uuid,
    pub tournament_id: Uuid,
    white_points: i64,
    black_points: i64,
    white_doubled: bool,
    black_doubled: bool,
}

impl ArenaGameResult {
    pub(crate) async fn insert(
        tournament_id: Uuid,
        game_id: Uuid,
        award: ArenaGameAward,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::insert_into(arena_game_results::table)
            .values(Self {
                game_id,
                tournament_id,
                white_points: i64::from(award.awarded_points[0].value()),
                black_points: i64::from(award.awarded_points[1].value()),
                white_doubled: award.doubled[0],
                black_doubled: award.doubled[1],
            })
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(crate) fn award(&self) -> Result<ArenaGameAward, DbError> {
        let score = |points| {
            u32::try_from(points)
                .map(Score::new)
                .map_err(|_| DbError::InvalidPersistedTournament {
                    reason: String::from("Arena award points are outside the scoring domain"),
                })
        };
        Ok(ArenaGameAward {
            awarded_points: [score(self.white_points)?, score(self.black_points)?],
            doubled: [self.white_doubled, self.black_doubled],
        })
    }
}
