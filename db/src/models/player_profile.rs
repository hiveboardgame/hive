use crate::{db_error::DbError, models::User, schema::player_profiles, DbConn};
use diesel::{
    sql_types::{Bool, Nullable, Text, Uuid as SqlUuid},
    OptionalExtension,
    QueryDsl,
    Queryable,
    Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = player_profiles)]
pub struct PlayerProfile {
    pub user_id: Uuid,
    pub flag: Option<String>,
    pub bio: String,
    pub bio_restricted: bool,
}

pub enum BioModeration {
    Clear,
    Restrict,
    Replace(String),
    Reinstate,
}

impl PlayerProfile {
    pub async fn find(user_id: Uuid, conn: &mut DbConn<'_>) -> Result<Option<Self>, DbError> {
        Ok(player_profiles::table
            .find(user_id)
            .select(Self::as_select())
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn save_flag(
        user_id: Uuid,
        flag: Option<&str>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        User::find_active_by_uuid(&user_id, conn).await?;
        diesel::sql_query(
            "INSERT INTO player_profiles (user_id, flag) VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET flag = EXCLUDED.flag",
        )
        .bind::<SqlUuid, _>(user_id)
        .bind::<Nullable<Text>, _>(flag)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn save_bio(user_id: Uuid, bio: &str, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        User::find_active_by_uuid(&user_id, conn).await?;
        // Check the current restriction under the UPSERT's row lock, including stale forms.
        diesel::sql_query(
            "INSERT INTO player_profiles (user_id, bio) VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET bio = CASE
            WHEN player_profiles.bio_restricted THEN player_profiles.bio ELSE EXCLUDED.bio END",
        )
        .bind::<SqlUuid, _>(user_id)
        .bind::<Text, _>(bio)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn moderate_bio(
        actor_id: Uuid,
        user_id: Uuid,
        action: BioModeration,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        if !User::find_active_by_uuid(&actor_id, conn).await?.admin {
            return Err(DbError::Unauthorized);
        }
        User::find_active_by_uuid(&user_id, conn).await?;
        let (bio, restriction) = match action {
            BioModeration::Clear => (Some(String::new()), None),
            BioModeration::Restrict => (Some(String::new()), Some(true)),
            BioModeration::Replace(text) => (Some(text), None),
            BioModeration::Reinstate => (None, Some(false)),
        };
        diesel::sql_query(
            "INSERT INTO player_profiles (user_id, bio, bio_restricted)
            VALUES ($1, COALESCE($2, ''), COALESCE($3, FALSE))
            ON CONFLICT (user_id) DO UPDATE SET
            bio = COALESCE($2, player_profiles.bio),
            bio_restricted = COALESCE($3, player_profiles.bio_restricted)",
        )
        .bind::<SqlUuid, _>(user_id)
        .bind::<Nullable<Text>, _>(bio)
        .bind::<Nullable<Bool>, _>(restriction)
        .execute(conn)
        .await?;
        Ok(())
    }
}
