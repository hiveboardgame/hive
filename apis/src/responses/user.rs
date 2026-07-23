#[cfg(feature = "ssr")]
mod ssr {
    use crate::responses::rating::RatingResponseDb;
    use anyhow::Result;
    use db_lib::{
        models::{Rating, User},
        DbConn,
    };
    use shared_types::{GameSpeed, RatingResponse, Takeback, UserResponse};
    use std::collections::HashMap;
    use uuid::Uuid;

    pub trait UserResponseDb: Sized {
        fn from_uuid(
            id: &Uuid,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_uuids(
            ids: &[Uuid],
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<HashMap<Uuid, Self>>> + Send;
        fn from_username(
            username: &str,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_model(
            user: &User,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn search_usernames(
            pattern: &str,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Vec<Self>>> + Send;
    }

    impl UserResponseDb for UserResponse {
        async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self> {
            let user = User::find_by_uuid(id, conn).await?;
            Self::from_model(&user, conn).await
        }

        async fn from_uuids(ids: &[Uuid], conn: &mut DbConn<'_>) -> Result<HashMap<Uuid, Self>> {
            let users = User::find_by_uuids(ids, conn).await?;
            if users.is_empty() {
                return Ok(HashMap::new());
            }

            let user_ids: Vec<Uuid> = users.iter().map(|user| user.id).collect();
            let rating_rows = Rating::for_uuids(&user_ids, conn).await?;
            let mut ratings_by_user: HashMap<Uuid, HashMap<String, Rating>> = HashMap::new();
            for rating in rating_rows {
                ratings_by_user
                    .entry(rating.user_uid)
                    .or_default()
                    .insert(rating.speed.clone(), rating);
            }

            let mut result = HashMap::new();
            for user in users {
                let user_rating_rows = ratings_by_user
                    .get(&user.id)
                    .ok_or_else(|| anyhow::anyhow!("Ratings not found for user {}", user.id))?;
                let user_response = from_model_with_ratings(&user, user_rating_rows)?;
                result.insert(user.id, user_response);
            }
            Ok(result)
        }

        async fn from_username(username: &str, conn: &mut DbConn<'_>) -> Result<Self> {
            let user = User::find_by_username(username, conn).await?;
            Self::from_model(&user, conn).await
        }

        async fn from_model(user: &User, conn: &mut DbConn<'_>) -> Result<Self> {
            let mut ratings = HashMap::new();
            for game_speed in GameSpeed::all_rated().into_iter() {
                let rating = RatingResponse::from_user(user, &game_speed, conn).await?;
                ratings.insert(game_speed, rating);
            }
            let response = UserResponse {
                username: user.username.clone(),
                uid: user.id,
                patreon: user.patreon,
                bot: user.bot,
                admin: user.admin,
                deleted: user.deleted,
                takeback: Takeback::from_str_or_default(&user.takeback),
                ratings,
                lang: user.lang.clone(),
            };
            Ok(response)
        }

        async fn search_usernames(pattern: &str, conn: &mut DbConn<'_>) -> Result<Vec<Self>> {
            let users = User::search_usernames(pattern, conn).await?;
            let mut responses = Vec::with_capacity(users.len());

            for user in users {
                responses.push(UserResponse::from_model(&user, conn).await?);
            }

            Ok(responses)
        }
    }

    fn from_model_with_ratings(
        user: &User,
        user_rating_rows: &HashMap<String, Rating>,
    ) -> Result<UserResponse> {
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
            admin: user.admin,
            deleted: user.deleted,
            takeback: Takeback::from_str_or_default(&user.takeback),
            ratings,
            lang: user.lang.clone(),
        })
    }
}

#[cfg(feature = "ssr")]
pub use ssr::UserResponseDb;
