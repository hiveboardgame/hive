use serde::{Deserialize, Serialize};
use shared_types::game_speed::GameSpeed;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UserResponse {
    pub username: String,
    pub uid: Uuid,
    pub ratings: HashMap<GameSpeed, RatingResponse>,
}

impl UserResponse {
    pub fn rating_for_speed(&self, game_speed: &GameSpeed) -> u64 {
        match game_speed {
            GameSpeed::Blitz => self.blitz(),
            GameSpeed::Correspondence => self.correspondence(),
            GameSpeed::Bullet => self.bullet(),
            GameSpeed::Rapid => self.rapid(),
            GameSpeed::Classic => self.classic(),
            GameSpeed::Untimed => 0,
        }
    }

    pub fn bullet(&self) -> u64 {
        if let Some(rating_response) = self.ratings.get(&GameSpeed::Bullet) {
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

use cfg_if::cfg_if;

use super::rating::RatingResponse;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::user::User,
    DbPool,
};
use anyhow::Result;
impl UserResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self> {
        let user = User::find_by_uuid(id, pool).await?;
        Self::from_user(&user, pool).await
    }

    pub async fn from_username(username: &str, pool: &DbPool) -> Result<Self> {
        let user = User::find_by_username(username, pool).await?;
        Self::from_user(&user, pool).await
    }

    pub async fn from_user(user: &User, pool: &DbPool) -> Result<Self> {
        let mut ratings = HashMap::new();
        for game_speed in GameSpeed::all_rated().into_iter() {
            let rating = RatingResponse::from_user(user, &game_speed, pool).await?;
            ratings.insert(game_speed, rating);
        }
        Ok(Self {
            username: user.username.clone(),
            uid: user.id,
            ratings,
        })
    }
}
}}
