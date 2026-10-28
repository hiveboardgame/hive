use crate::{
    db_error::DbError,
    schema::tutorial_progress::{self, lesson_id, user_id as user_id_column},
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = tutorial_progress)]
struct NewTutorialProgress<'a> {
    user_id: Uuid,
    lesson_id: &'a str,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = tutorial_progress)]
pub struct TutorialProgress {
    pub user_id: Uuid,
    pub lesson_id: String,
    pub completed_at: DateTime<Utc>,
}

impl TutorialProgress {
    pub async fn completed_lessons(
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<String>, DbError> {
        Ok(tutorial_progress::table
            .filter(user_id_column.eq(user_id))
            .select(lesson_id)
            .order(lesson_id.asc())
            .load(conn)
            .await?)
    }

    pub async fn complete(
        user_id: Uuid,
        lessons: &[String],
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        if lessons.is_empty() {
            return Ok(0);
        }
        let rows: Vec<NewTutorialProgress> = lessons
            .iter()
            .map(|lesson| NewTutorialProgress {
                user_id,
                lesson_id: lesson,
            })
            .collect();
        Ok(diesel::insert_into(tutorial_progress::table)
            .values(&rows)
            .on_conflict_do_nothing()
            .execute(conn)
            .await?)
    }
}
