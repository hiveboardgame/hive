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
    pub bio_blocked: bool,
}

fn validate_bio(bio: &str) -> Result<(), DbError> {
    if bio.chars().count() > 300 {
        return Err(DbError::InvalidInput {
            info: "About text exceeds 300 characters".into(),
            error: String::new(),
        });
    }
    Ok(())
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

    /// Owner writes can update the flag, but never the moderation state or a blocked bio.
    /// The conditional UPSERT locks the row and is atomic with concurrent moderation.
    pub async fn save_owner(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        validate_bio(&self.bio)?;
        diesel::sql_query("INSERT INTO player_profiles (user_id, flag, bio) VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET flag = EXCLUDED.flag,
            bio = CASE WHEN player_profiles.bio_blocked THEN player_profiles.bio ELSE EXCLUDED.bio END")
            .bind::<SqlUuid,_>(self.user_id).bind::<Nullable<Text>,_>(&self.flag).bind::<Text,_>(&self.bio)
            .execute(conn).await?;
        Ok(())
    }

    /// Called with the actor ID from the authenticated session, never submitted by the client.
    pub async fn moderate_bio(
        actor_id: Uuid,
        user_id: Uuid,
        bio: String,
        blocked: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let actor = User::find_active_by_uuid(&actor_id, conn).await?;
        if !actor.admin {
            return Err(DbError::Unauthorized);
        }
        User::find_active_by_uuid(&user_id, conn).await?;
        validate_bio(&bio)?;
        // Preserve the player's flag, including concurrent flag edits.
        diesel::sql_query("INSERT INTO player_profiles (user_id, bio, bio_blocked) VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET bio = EXCLUDED.bio, bio_blocked = EXCLUDED.bio_blocked")
            .bind::<SqlUuid,_>(user_id).bind::<Text,_>(bio).bind::<Bool,_>(blocked)
            .execute(conn).await?;
        Ok(())
    }
}
