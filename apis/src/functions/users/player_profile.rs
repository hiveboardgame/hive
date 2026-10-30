use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use server_fn::codec;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub flag: Option<String>,
    pub location: Option<String>,
    pub age: Option<i32>,
    pub gender: Option<String>,
    pub bio: String,
    pub member_since: Option<chrono::NaiveDate>,
    // Only populated for an authenticated request by the profile owner.
    pub owner_birth_year: Option<i32>,
}

#[derive(Debug, PartialEq)]
pub struct ValidatedProfile {
    pub flag: Option<String>,
    pub location: Option<String>,
    pub birth_year: Option<i32>,
    pub gender: Option<String>,
    pub bio: String,
}

fn optional(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

pub fn validate_profile(
    flag: String,
    location: String,
    birth_year: String,
    gender: String,
    bio: String,
    current_year: i32,
) -> Result<ValidatedProfile, String> {
    let flag = optional(flag);
    let location = optional(location);
    let gender = optional(gender);
    let birth_year = optional(birth_year)
        .map(|year| year.parse::<i32>().map_err(|_| "Enter a valid birth year."))
        .transpose()?;
    if flag.as_ref().is_some_and(|flag| {
        !crate::profile_catalog::catalog()
            .countries
            .iter()
            .any(|(code, _)| code == flag)
    }) {
        return Err("Select a flag from the list.".into());
    }
    if location.as_ref().is_some_and(|location| {
        !crate::profile_catalog::catalog()
            .locations
            .contains(location)
    }) {
        return Err("Select a city or region from the list.".into());
    }
    if birth_year.is_some_and(|year| !(1900..=current_year).contains(&year)) {
        return Err(format!(
            "Birth year must be between 1900 and {current_year}."
        ));
    }
    if gender
        .as_deref()
        .is_some_and(|gender| !["M", "F", "X", "?"].contains(&gender))
    {
        return Err("Select a gender from the list.".into());
    }
    if crate::profile_bio::bio_contains_link(&bio) {
        return Err("Links and email addresses are not allowed in your bio.".into());
    }
    if bio.chars().count() > 300 {
        return Err("Your bio must be 300 characters or fewer.".into());
    }
    Ok(ValidatedProfile {
        flag,
        location,
        birth_year,
        gender,
        bio: bio.trim().to_string(),
    })
}

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn get_player_details(username: String) -> Result<ProfileDetails, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use chrono::{Datelike, Utc};
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
    let owner = uuid().await.ok() == Some(user.id);
    let Some(profile) = PlayerProfile::find(user.id, &mut conn).await? else {
        return Ok(ProfileDetails {
            member_since,
            ..Default::default()
        });
    };
    Ok(ProfileDetails {
        flag: profile.flag,
        location: profile.location,
        age: profile.birth_year.map(|year| Utc::now().year() - year),
        gender: profile.gender,
        bio: profile.bio,
        member_since,
        owner_birth_year: if owner { profile.birth_year } else { None },
    })
}

#[server(client = CsrfClient)]
pub async fn save_player_details(
    flag: String,
    location: String,
    birth_year: String,
    gender: String,
    bio: String,
) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use chrono::{Datelike, Utc};
    use db_lib::{
        get_conn,
        models::{PlayerProfile, User},
    };
    // The user ID always comes from the session, never from submitted form data.
    let user_id = uuid().await?;
    let data = validate_profile(flag, location, birth_year, gender, bio, Utc::now().year())
        .map_err(ServerFnError::new)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    User::find_active_by_uuid(&user_id, &mut conn).await?;
    PlayerProfile {
        user_id,
        flag: data.flag,
        location: data.location,
        birth_year: data.birth_year,
        gender: data.gender,
        bio: data.bio,
    }
    .save(&mut conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn validate(year: &str, bio: &str) -> Result<ValidatedProfile, String> {
        validate_profile(
            String::new(),
            String::new(),
            year.into(),
            String::new(),
            bio.into(),
            2026,
        )
    }
    #[test]
    fn optional_fields_can_be_cleared() {
        let p = validate("  ", "  ").unwrap();
        assert_eq!(p.birth_year, None);
        assert_eq!(p.flag, None);
        assert_eq!(p.location, None);
        assert_eq!(p.gender, None);
        assert_eq!(p.bio, "");
    }
    #[test]
    fn validates_birth_year_and_unicode_character_limit() {
        assert!(validate("1990", &"🐝".repeat(300)).is_ok());
        assert!(validate("1990", &"🐝".repeat(301)).is_err());
        for year in ["1899", "2027", "banana", "1990.5"] {
            assert!(validate(year, "").is_err());
        }
    }
    #[test]
    fn rejects_bio_links_when_saving() {
        for bio in ["https://example.com", "www.example.com", "example.com"] {
            assert!(validate("1990", bio).is_err());
        }
    }

    #[test]
    fn rejects_unlisted_values() {
        assert!(validate_profile(
            "ZZ".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            2026
        )
        .is_err());
        assert!(validate_profile(
            "GB".into(),
            "my street address".into(),
            "".into(),
            "".into(),
            "".into(),
            2026
        )
        .is_err());
        assert!(validate_profile(
            "GB".into(),
            "".into(),
            "".into(),
            "invalid".into(),
            "".into(),
            2026
        )
        .is_err());
        assert!(validate_profile(
            "GB".into(),
            "London, United Kingdom".into(),
            "1990".into(),
            "?".into(),
            "Hello".into(),
            2026
        )
        .is_ok());
    }
}
