use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub flag: Option<String>,
    pub bio: String,
    pub member_since: Option<chrono::NaiveDate>,
}

#[derive(Debug, PartialEq)]
pub struct ValidatedProfile {
    pub flag: Option<String>,
    pub bio: String,
}

pub fn validate_profile(flag: String, bio: String) -> Result<ValidatedProfile, String> {
    let flag = flag.trim().to_string();
    let flag = (!flag.is_empty()).then_some(flag);
    if flag.as_ref().is_some_and(|flag| {
        !crate::profile_catalog::catalog()
            .countries
            .iter()
            .any(|(code, _)| code == flag)
    }) {
        return Err("Select a flag from the list.".into());
    }
    if bio.chars().count() > 300 {
        return Err("Your About text must be 300 characters or fewer.".into());
    }
    Ok(ValidatedProfile {
        flag,
        bio: bio.trim().to_string(),
    })
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_player_details(username: String) -> Result<ProfileDetails, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        models::{PlayerProfile, User},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_username(&username, &mut conn).await?;
    if user.deleted {
        return Ok(ProfileDetails::default());
    }
    let member_since = Some(user.created_at.date_naive());
    let Some(profile) = PlayerProfile::find(user.id, &mut conn).await? else {
        return Ok(ProfileDetails {
            member_since,
            ..Default::default()
        });
    };
    Ok(ProfileDetails {
        flag: profile.flag,
        bio: profile.bio,
        member_since,
    })
}

#[server(client = CsrfClient)]
pub async fn save_player_details(flag: String, bio: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{PlayerProfile, User},
    };
    // The user ID always comes from the session, never from submitted form data.
    let user_id = uuid().await?;
    let data = validate_profile(flag, bio).map_err(ServerFnError::new)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    User::find_active_by_uuid(&user_id, &mut conn).await?;
    PlayerProfile {
        user_id,
        flag: data.flag,
        bio: data.bio,
    }
    .save(&mut conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_fields_can_be_cleared() {
        let profile = validate_profile("  ".into(), "  ".into()).unwrap();
        assert_eq!(profile.flag, None);
        assert_eq!(profile.bio, "");
    }
    #[test]
    fn enforces_unicode_character_limit_and_allows_links() {
        assert!(validate_profile("".into(), "🐝".repeat(300)).is_ok());
        assert!(validate_profile("".into(), "🐝".repeat(301)).is_err());
        for bio in [
            "https://example.com",
            "www.example.com",
            "example.com",
            "hello@example.com",
        ] {
            assert!(validate_profile("GB".into(), bio.into()).is_ok());
        }
    }
    #[test]
    fn validates_selected_flag() {
        assert!(validate_profile("ZZ".into(), "Hello".into()).is_err());
        assert!(validate_profile("GB".into(), "Hello".into()).is_ok());
    }
}
