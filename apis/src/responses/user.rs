use super::rating::RatingResponse;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use shared_types::{GameSpeed, Takeback};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct UserResponse {
    pub username: String,
    pub uid: Uuid,
    pub patreon: bool,
    pub bot: bool,
    pub admin: bool,
    pub deleted: bool,
    pub ratings: HashMap<GameSpeed, RatingResponse>,
    pub takeback: Takeback,
    pub lang: Option<String>,
    #[serde(default)]
    pub chat_muted: bool,
}

// Temporary compatibility for pre-mute browser tabs during blue-green deploys (2026-10).
// Their decoders accept named fields but reject an extra positional field, breaking live games.
// Revisit around 2026-11: once remaining pre-mute tabs can be required to refresh,
// remove this impl and restore derived Serialize.
impl Serialize for UserResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields = serializer.serialize_map(Some(10))?;
        fields.serialize_entry("username", &self.username)?;
        fields.serialize_entry("uid", &self.uid)?;
        fields.serialize_entry("patreon", &self.patreon)?;
        fields.serialize_entry("bot", &self.bot)?;
        fields.serialize_entry("admin", &self.admin)?;
        fields.serialize_entry("deleted", &self.deleted)?;
        fields.serialize_entry("ratings", &self.ratings)?;
        fields.serialize_entry("takeback", &self.takeback)?;
        fields.serialize_entry("lang", &self.lang)?;
        fields.serialize_entry("chat_muted", &self.chat_muted)?;
        fields.end()
    }
}

impl UserResponse {
    pub fn rating_for_speed(&self, game_speed: &GameSpeed) -> u64 {
        match game_speed {
            GameSpeed::Blitz => self.blitz(),
            GameSpeed::Correspondence | GameSpeed::Untimed => self.correspondence(),
            GameSpeed::Bullet => self.bullet(),
            GameSpeed::Rapid => self.rapid(),
            GameSpeed::Classic => self.classic(),
            GameSpeed::Puzzle => self.puzzle(),
        }
    }

    pub fn bullet(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Bullet) {
            return rating_response.rating;
        }
        0
    }

    pub fn puzzle(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Puzzle) {
            return rating_response.rating;
        }
        0
    }

    pub fn blitz(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Blitz) {
            return rating_response.rating;
        }
        0
    }

    pub fn correspondence(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Correspondence) {
            return rating_response.rating;
        }
        0
    }

    pub fn classic(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Classic) {
            return rating_response.rating;
        }
        0
    }

    pub fn rapid(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Rapid) {
            return rating_response.rating;
        }
        0
    }
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Rating, User},
    DbConn,
};
use anyhow::Result;
impl UserResponse {
    pub fn from_models_with_ratings(
        users: &[User],
        ratings: &[Rating],
    ) -> Result<HashMap<Uuid, Self>> {
        let mut ratings_by_user: HashMap<Uuid, HashMap<String, Rating>> = HashMap::new();
        for rating in ratings {
            ratings_by_user
                .entry(rating.user_uid)
                .or_default()
                .insert(rating.speed.clone(), rating.clone());
        }
        users.iter().map(|user| {
            let rows = ratings_by_user.get(&user.id).ok_or_else(|| {
                anyhow::anyhow!("Ratings not found for user {}", user.id)
            })?;
            Ok((user.id, Self::from_model_with_ratings(user, rows)?))
        }).collect()
    }

    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        let user = User::find_by_uuid(id, conn).await?;
        Self::from_model(&user, conn).await
    }

    pub async fn from_uuids(ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<HashMap<Uuid, Self>> {
        let users = User::find_by_uuids(ids, conn).await?;
        Self::from_models(&users, conn).await
    }

    pub async fn from_models(users: &[User], conn: &mut DbConn<'_>) -> Result<HashMap<Uuid, Self>> {
        if users.is_empty() {
            return Ok(HashMap::new());
        }

        let user_ids: Vec<Uuid> = users.iter().map(|user| user.id).collect();
        let rating_rows = Rating::for_uuids(&user_ids, conn).await?;
        Self::from_models_with_ratings(users, &rating_rows)
    }

    fn from_model_with_ratings(
        user: &User,
        user_rating_rows: &HashMap<String, Rating>,
    ) -> Result<Self> {
        let mut ratings = HashMap::new();
        for game_speed in GameSpeed::all_rated().into_iter() {
            let rating = user_rating_rows
                .get(&game_speed.to_string())
                .ok_or_else(|| {
                    anyhow::anyhow!("{} rating not found for user {}", game_speed, user.id)
                })?;
            let rating = RatingResponse::from_rating(rating);
            ratings.insert(game_speed, rating);
        }
        Ok(UserResponse {
            username: user.username.clone(),
            uid: user.id,
            patreon: user.patreon,
            bot: user.bot,
            chat_muted: user.chat_muted,
            admin: user.admin,
            deleted: user.deleted,
            takeback: Takeback::from_str_or_default(&user.takeback),
            ratings,
            lang: user.lang.clone(),
        })
    }

    pub async fn from_username(username: &str, conn: &mut DbConn<'_>) -> Result<Self> {
        let user = User::find_by_username(username, conn).await?;
        Self::from_model(&user, conn).await
    }

    pub async fn from_model(user: &User, conn: &mut DbConn<'_>) -> Result<Self> {
        Self::from_models(std::slice::from_ref(user), conn)
            .await?
            .remove(&user.id)
            .ok_or_else(|| anyhow::anyhow!("User response not assembled for {}", user.id))
    }
    pub async fn search_usernames(pattern: &str, excluded: &[String], conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
        let users = User::search_usernames(pattern, excluded, conn).await?;
        let mut by_id = Self::from_models(&users, conn).await?;
        users
            .into_iter()
            .map(|user| {
                by_id.remove(&user.id).ok_or_else(|| {
                    anyhow::anyhow!("User response not assembled for {}", user.id)
                })
            })
            .collect()
    }
}
}}

#[cfg(test)]
mod tests {
    use super::*;
    use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
    use shared_types::Certainty;

    // The positional layout in browser bundles shipped before admin chat mutes.
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct PreMuteUserResponse {
        username: String,
        uid: Uuid,
        patreon: bool,
        bot: bool,
        admin: bool,
        deleted: bool,
        ratings: HashMap<GameSpeed, RatingResponse>,
        takeback: Takeback,
        lang: Option<String>,
    }

    fn pre_mute_user() -> PreMuteUserResponse {
        let uid = Uuid::from_u128(1);
        PreMuteUserResponse {
            username: "Nash102".to_string(),
            uid,
            patreon: true,
            bot: false,
            admin: false,
            deleted: false,
            ratings: HashMap::from([(
                GameSpeed::Blitz,
                RatingResponse {
                    speed: GameSpeed::Blitz,
                    rating: 1600,
                    played: 10,
                    win: 6,
                    loss: 3,
                    draw: 1,
                    certainty: Certainty::Rankable,
                    user_uid: uid,
                },
            )]),
            takeback: Takeback::CasualOnly,
            lang: Some("en".to_string()),
        }
    }

    fn user(chat_muted: bool) -> UserResponse {
        let previous = pre_mute_user();
        UserResponse {
            username: previous.username,
            uid: previous.uid,
            patreon: previous.patreon,
            bot: previous.bot,
            admin: previous.admin,
            deleted: previous.deleted,
            ratings: previous.ratings,
            takeback: previous.takeback,
            lang: previous.lang,
            chat_muted,
        }
    }

    #[test]
    fn pre_mute_clients_decode_user_responses_with_either_mute_status() {
        for chat_muted in [false, true] {
            let bytes = MsgpackSerdeCodec::encode(&user(chat_muted)).unwrap();
            let decoded: PreMuteUserResponse = MsgpackSerdeCodec::decode(&bytes)
                .expect("old tabs must keep decoding users embedded in game updates and snapshots");
            assert_eq!(decoded, pre_mute_user());
        }
    }

    #[test]
    fn new_clients_decode_pre_mute_positional_user_responses() {
        let bytes = MsgpackSerdeCodec::encode(&pre_mute_user()).unwrap();
        let decoded: UserResponse = MsgpackSerdeCodec::decode(&bytes)
            .expect("new tabs must accept responses from the previous server during rollback");
        assert_eq!(decoded, user(false));
    }

    #[test]
    fn new_clients_preserve_mute_status_in_named_and_positional_user_responses() {
        for chat_muted in [false, true] {
            let expected = user(chat_muted);
            let named = MsgpackSerdeCodec::encode(&expected).unwrap();
            let positional = MsgpackSerdeCodec::encode(&(
                &expected.username,
                expected.uid,
                expected.patreon,
                expected.bot,
                expected.admin,
                expected.deleted,
                &expected.ratings,
                &expected.takeback,
                &expected.lang,
                expected.chat_muted,
            ))
            .unwrap();
            for bytes in [named, positional] {
                let decoded: UserResponse = MsgpackSerdeCodec::decode(&bytes).unwrap();
                assert_eq!(decoded, expected);
            }
        }
    }
}
