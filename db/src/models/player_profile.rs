use crate::{
    db_error::DbError,
    models::User,
    schema::player_profiles::{self, bio, bio_restricted, flag, table as profiles, user_id},
    DbConn,
};
use diesel::{
    insert_into,
    query_dsl::methods::FilterDsl,
    upsert::excluded,
    ExpressionMethods,
    OptionalExtension,
    QueryDsl,
    Queryable,
    Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use shared_types::BioModeration;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = player_profiles)]
pub struct PlayerProfile {
    pub user_id: Uuid,
    pub flag: Option<String>,
    pub bio: String,
    pub bio_restricted: bool,
}

impl PlayerProfile {
    pub async fn find(owner_id: Uuid, conn: &mut DbConn<'_>) -> Result<Option<Self>, DbError> {
        Ok(profiles
            .find(owner_id)
            .select(Self::as_select())
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn save_flag(
        owner_id: Uuid,
        new_flag: Option<&str>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        User::find_active_by_uuid(&owner_id, conn).await?;
        insert_into(profiles)
            .values((user_id.eq(owner_id), flag.eq(new_flag)))
            .on_conflict(user_id)
            .do_update()
            .set(flag.eq(excluded(flag)))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn save_bio(
        owner_id: Uuid,
        new_bio: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        User::find_active_by_uuid(&owner_id, conn).await?;
        let rows = insert_into(profiles)
            .values((user_id.eq(owner_id), bio.eq(new_bio)))
            .on_conflict(user_id)
            .do_update()
            .set(bio.eq(excluded(bio)))
            .filter(bio_restricted.eq(false))
            .execute(conn)
            .await?;
        if rows == 0 {
            return Err(DbError::ProfileBioRestricted);
        }
        Ok(())
    }

    pub async fn moderate_bio(
        actor_id: Uuid,
        owner_id: Uuid,
        action: BioModeration,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        if !User::find_active_by_uuid(&actor_id, conn).await?.admin {
            return Err(DbError::Unauthorized);
        }
        User::find_active_by_uuid(&owner_id, conn).await?;
        let (operation, new_bio, restriction) = match &action {
            BioModeration::Clear => ("clear", Some(""), None),
            BioModeration::Restrict => ("restrict", Some(""), Some(true)),
            BioModeration::Replace(text) => ("replace", Some(text.as_str()), None),
            BioModeration::Reinstate => ("reinstate", None, Some(false)),
        };
        let changes = (
            new_bio.map(|text| bio.eq(text)),
            restriction.map(|restricted| bio_restricted.eq(restricted)),
        );
        insert_into(profiles)
            .values((user_id.eq(owner_id), changes))
            .on_conflict(user_id)
            .do_update()
            .set(changes)
            .execute(conn)
            .await?;
        log::warn!(
            "Profile bio moderation: actor={actor_id} target={owner_id} operation={operation}"
        );
        Ok(())
    }
}
