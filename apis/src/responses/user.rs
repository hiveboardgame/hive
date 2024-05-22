use super::rating::RatingResponse;
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UserResponse {
    pub username: String,
    pub uid: Uuid,
    pub patreon: bool,
    pub admin: bool,
    pub ratings: HashMap<GameSpeed, RatingResponse>,
}

impl UserResponse {
    pub fn for_anon(uuid: Uuid) -> Self {
        Self {
            username: uuid.to_string(),
            uid: uuid,
            patreon: false,
            admin: false,
            ratings: HashMap::new(),
        }
    }

    pub fn rating_for_speed(&self, game_speed: &GameSpeed) -> u64 {
        match game_speed {
            GameSpeed::Blitz => self.blitz(),
            GameSpeed::Correspondence => self.correspondence(),
            GameSpeed::Bullet => self.bullet(),
            GameSpeed::Rapid => self.rapid(),
            GameSpeed::Classic => self.classic(),
            GameSpeed::Puzzle => self.puzzle(),
            GameSpeed::Untimed => 0,
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
    models::User,
    DbConn,
};
use anyhow::Result;
impl UserResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
        let user = User::find_by_uuid(id, conn).await?;
        Self::from_user(&user, conn).await
    }

    pub async fn from_username(username: &str, conn: &mut DbConn<'_>) -> Result<Self> {
        let user = User::find_by_username(username, conn).await?;
        Self::from_user(&user, conn).await
    }

    pub async fn from_user(user: &User, conn: &mut DbConn<'_>) -> Result<Self> {
        let mut ratings = HashMap::new();
        for game_speed in GameSpeed::all_rated().into_iter() {
            let rating = RatingResponse::from_user(user, &game_speed, conn).await?;
            ratings.insert(game_speed, rating);
        }
        let response = UserResponse {
            username: user.username.clone(),
            uid: user.id,
            patreon: user.patreon,
            admin: user.admin,
            ratings,
        };
        Ok(response)
    }
}
}}
