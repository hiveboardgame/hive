use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserResponse {
    pub username: String,
    pub uid: Uuid,
    pub rating: u64,
    pub played: i64,
    pub win: i64,
    pub loss: i64,
    pub draw: i64,
}

#[cfg(feature = "ssr")]
use db_lib::{
    models::{rating::Rating, user::User},
    DbPool,
};
#[cfg(feature = "ssr")]
use leptos::ServerFnError;
#[cfg(feature = "ssr")]
impl UserResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self, ServerFnError> {
        let user = User::find_by_uuid(id, pool).await?;
        let rating = Rating::for_uuid(id, pool).await?;

        Ok(Self {
            username: user.username,
            uid: user.id,
            rating: rating.rating.floor() as u64,
            played: rating.played,
            win: rating.won,
            loss: rating.lost,
            draw: rating.draw,
        })
    }
}
