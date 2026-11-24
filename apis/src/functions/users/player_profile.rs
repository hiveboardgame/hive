#[cfg(any(feature = "ssr", test))]
use crate::common::UiError;
use crate::security::csrf::CsrfClient;
#[cfg(feature = "ssr")]
use crate::{
    common::FLAG_CODES,
    functions::{auth::identity::uuid, db::pool},
};
#[cfg(feature = "ssr")]
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{PlayerProfile, User},
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;
use shared_types::BioModeration;
#[cfg(any(feature = "ssr", test))]
use shared_types::MAX_BIO_LENGTH;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub flag: Option<String>,
    pub bio: String,
    pub bio_restricted: bool,
}

pub type ProfileResource = LocalResource<Result<ProfileDetails, ServerFnError>>;

#[cfg(feature = "ssr")]
fn profile_error(error: impl Into<DbError>) -> ServerFnError {
    let message = match error.into() {
        DbError::Unauthorized => UiError::Unauthorized,
        DbError::ProfileBioRestricted => UiError::ProfileBioRestricted,
        error => {
            log::error!("Profile request failed: {error}");
            UiError::RequestUnavailable
        }
    };
    ServerFnError::new(message.encode())
}

#[cfg(any(feature = "ssr", test))]
fn validate_bio(bio: String) -> Result<String, ServerFnError> {
    if bio.chars().count() > MAX_BIO_LENGTH {
        return Err(ServerFnError::new(UiError::ProfileBioTooLong.encode()));
    }
    Ok(bio.trim().to_string())
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_player_details(user_id: Uuid) -> Result<ProfileDetails, ServerFnError> {
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(profile_error)?;
    if User::find_by_uuid(&user_id, &mut conn)
        .await
        .map_err(profile_error)?
        .deleted
    {
        return Ok(ProfileDetails::default());
    }
    Ok(PlayerProfile::find(user_id, &mut conn)
        .await
        .map_err(profile_error)?
        .map(|p| ProfileDetails {
            flag: p.flag,
            bio: p.bio,
            bio_restricted: p.bio_restricted,
        })
        .unwrap_or_default())
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn set_profile_flag(flag: Option<String>) -> Result<Option<String>, ServerFnError> {
    let user_id = uuid().await?;
    if flag
        .as_deref()
        .is_some_and(|code| !FLAG_CODES.contains(&code))
    {
        return Err(ServerFnError::new(UiError::ProfileFlagInvalid.encode()));
    }
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(profile_error)?;
    PlayerProfile::save_flag(user_id, flag.as_deref(), &mut conn)
        .await
        .map_err(profile_error)?;
    Ok(flag)
}

#[server(client = CsrfClient)]
pub async fn save_profile_bio(bio: String) -> Result<(), ServerFnError> {
    let user_id = uuid().await?;
    let bio = validate_bio(bio)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(profile_error)?;
    PlayerProfile::save_bio(user_id, &bio, &mut conn)
        .await
        .map_err(profile_error)
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn moderate_profile_bio(
    user_id: Uuid,
    operation: BioModeration,
) -> Result<(), ServerFnError> {
    let actor = uuid().await?;
    let operation = match operation {
        BioModeration::Replace(bio) => BioModeration::Replace(validate_bio(bio)?),
        other => other,
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(profile_error)?;
    PlayerProfile::moderate_bio(actor, user_id, operation, &mut conn)
        .await
        .map_err(profile_error)
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
