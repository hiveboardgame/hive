#[cfg(feature = "ssr")]
mod ssr {
    use anyhow::Result;
    use db_lib::{
        models::{Rating, User},
        DbConn,
    };
    use shared_types::{Certainty, GameSpeed, RatingResponse};
    use std::str::FromStr;
    use uuid::Uuid;

    pub trait RatingResponseDb: Sized {
        fn from_uuid(
            id: &Uuid,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_user(
            user: &User,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_username(
            username: &str,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> impl std::future::Future<Output = Result<Self>> + Send;
        fn from_rating(rating: &Rating) -> Self;
    }

    impl RatingResponseDb for RatingResponse {
        async fn from_uuid(
            id: &Uuid,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> Result<Self> {
            let rating = Rating::for_uuid(id, game_speed, conn).await?;
            Ok(Self::from_rating(&rating))
        }

        async fn from_user(
            user: &User,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> Result<Self> {
            let rating = Rating::for_uuid(&user.id, game_speed, conn).await?;
            Ok(Self::from_rating(&rating))
        }

        async fn from_username(
            username: &str,
            game_speed: &GameSpeed,
            conn: &mut DbConn<'_>,
        ) -> Result<Self> {
            let user = User::find_by_username(username, conn).await?;
            let rating = Rating::for_uuid(&user.id, game_speed, conn).await?;
            Ok(Self::from_rating(&rating))
        }

        fn from_rating(rating: &Rating) -> Self {
            Self {
                speed: GameSpeed::from_str(&rating.speed)
                    .expect("Rating to have a valid GameSpeed"),
                rating: rating.rating.floor() as u64,
                played: rating.played,
                win: rating.won,
                loss: rating.lost,
                draw: rating.draw,
                certainty: Certainty::from_deviation(rating.deviation),
                user_uid: rating.user_uid,
            }
        }
    }
}

#[cfg(feature = "ssr")]
pub use ssr::RatingResponseDb;
