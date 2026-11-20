#[cfg(any(feature = "ssr", test))]
use crate::common::UiError;
use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub flag: Option<String>,
    pub bio: String,
    pub bio_restricted: bool,
}

#[cfg(any(feature = "ssr", test))]
fn validate_bio(bio: String) -> Result<String, ServerFnError> {
    if bio.chars().count() > 300 {
        return Err(ServerFnError::new(UiError::ProfileBioTooLong.encode()));
    }
    Ok(bio.trim().to_string())
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_player_details(user_id: Uuid) -> Result<ProfileDetails, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        models::{PlayerProfile, User},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    if User::find_by_uuid(&user_id, &mut conn).await?.deleted {
        return Ok(ProfileDetails::default());
    }
    Ok(PlayerProfile::find(user_id, &mut conn)
        .await?
        .map(|p| ProfileDetails {
            flag: p.flag,
            bio: p.bio,
            bio_restricted: p.bio_restricted,
        })
        .unwrap_or_default())
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn set_profile_flag(flag: Option<String>) -> Result<Option<String>, ServerFnError> {
    use crate::{
        common::flag_name,
        functions::{auth::identity::uuid, db::pool},
    };
    use db_lib::{get_conn, models::PlayerProfile};
    let user_id = uuid().await?;
    if flag
        .as_deref()
        .is_some_and(|code| flag_name(code).is_none())
    {
        return Err(ServerFnError::new(UiError::ProfileFlagInvalid.encode()));
    }
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    PlayerProfile::save_flag(user_id, flag.as_deref(), &mut conn).await?;
    Ok(flag)
}

#[server(client = CsrfClient)]
pub async fn save_profile_bio(bio: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PlayerProfile};
    let user_id = uuid().await?;
    let bio = validate_bio(bio)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    PlayerProfile::save_bio(user_id, &bio, &mut conn)
        .await
        .map_err(|_| ServerFnError::new(UiError::RequestUnavailable.encode()))?;
    Ok(())
}

#[server(client = CsrfClient)]
pub async fn moderate_profile_bio(
    user_id: Uuid,
    operation: String,
    bio: String,
) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{BioModeration, PlayerProfile},
    };
    let actor = uuid().await?;
    let action = match operation.as_str() {
        "clear" => BioModeration::Clear,
        "restrict" => BioModeration::Restrict,
        "replace" => BioModeration::Replace(validate_bio(bio)?),
        "reinstate" => BioModeration::Reinstate,
        _ => return Err(ServerFnError::new(UiError::RequestUnavailable.encode())),
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    PlayerProfile::moderate_bio(actor, user_id, action, &mut conn)
        .await
        .map_err(|error| match error {
            db_lib::db_error::DbError::Unauthorized => {
                ServerFnError::new(UiError::Unauthorized.encode())
            }
            _ => ServerFnError::new(UiError::RequestUnavailable.encode()),
        })?;
    log::warn!("Profile bio moderation: actor={actor} target={user_id} operation={operation}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_limit_and_clearing() {
        assert_eq!(validate_bio("  ".into()).unwrap(), "");
        assert!(validate_bio("🐝".repeat(300)).is_ok());
        assert!(validate_bio("🐝".repeat(301)).is_err());
    }
}
