use crate::{GameSpeed, RatingResponse, Takeback};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
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
