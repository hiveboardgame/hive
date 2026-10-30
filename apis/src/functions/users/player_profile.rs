use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub flag: Option<String>,
    pub bio: String,
    pub bio_blocked: bool,
}

#[cfg(any(feature = "ssr", test))]
impl ProfileDetails {
    fn public(flag: Option<String>, bio: String, bio_blocked: bool) -> Self {
        Self {
            flag,
            bio: if bio_blocked { String::new() } else { bio },
            bio_blocked,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BioModerationDetails {
    pub bio: String,
    pub blocked: bool,
}

fn validate_bio(bio: String) -> Result<String, String> {
    if bio.chars().count() > 300 {
        return Err("Your About text must be 300 characters or fewer.".into());
    }
    Ok(bio.trim().to_string())
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
    Ok(ValidatedProfile {
        flag,
        bio: validate_bio(bio)?,
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
    let Some(profile) = PlayerProfile::find(user.id, &mut conn).await? else {
        return Ok(ProfileDetails::default());
    };
    // Blocked text is never sent through this public endpoint, even to its owner.
    Ok(ProfileDetails::public(
        profile.flag,
        profile.bio,
        profile.bio_blocked,
    ))
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
        bio_blocked: false,
    }
    .save_owner(&mut conn)
    .await?;
    Ok(())
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_bio_moderation(
    user_id: uuid::Uuid,
) -> Result<BioModerationDetails, ServerFnError> {
    use crate::functions::{auth::identity::ensure_admin, db::pool};
    use db_lib::{
        get_conn,
        models::{PlayerProfile, User},
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    User::find_active_by_uuid(&user_id, &mut conn).await?;
    Ok(PlayerProfile::find(user_id, &mut conn)
        .await?
        .map(|profile| BioModerationDetails {
            bio: profile.bio,
            blocked: profile.bio_blocked,
        })
        .unwrap_or_default())
}

#[server(client = CsrfClient)]
pub async fn save_bio_moderation(
    user_id: uuid::Uuid,
    bio: String,
    blocked: bool,
) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PlayerProfile};
    let actor_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    PlayerProfile::moderate_bio(
        actor_id,
        user_id,
        validate_bio(bio).map_err(ServerFnError::new)?,
        blocked,
        &mut conn,
    )
    .await?;
    log::warn!("Profile bio moderated by admin {actor_id}: target={user_id}, blocked={blocked}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_bio_is_redacted_from_public_response() {
        let hidden = ProfileDetails::public(Some("GB".into()), "Hidden text".into(), true);
        assert_eq!(hidden.bio, "");
        assert!(hidden.bio_blocked);
        assert_eq!(hidden.flag.as_deref(), Some("GB"));
        let visible = ProfileDetails::public(None, "Visible text".into(), false);
        assert_eq!(visible.bio, "Visible text");
    }
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
