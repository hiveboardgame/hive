#![cfg(feature = "ssr")]

mod common;

use apis::{
    common::{ServerMessage, UiError},
    responses::RatingResponse,
    websocket::{server_handlers::challenges::accept::AcceptHandler, RealtimeGate},
};
use db_lib::{
    get_conn,
    models::{Challenge, NewChallenge, NewUser, Rating, User},
    schema::{games, ratings},
};
use diesel::{dsl::exists, prelude::*};
use diesel_async::RunQueryDsl;
use hive_lib::{ColorChoice, GameType};
use shared_types::{ChallengeDetails, ChallengeId, ChallengeVisibility, GameSpeed, TimeMode};

#[tokio::test(flavor = "multi_thread")]
async fn challenge_admission_uses_the_displayed_rating_at_both_band_boundaries() {
    let db = common::db::test_db().await;
    let mut conn = get_conn(&db.pool).await.unwrap();
    let challenger = User::create(
        NewUser::new("challenger", "password", "challenger@example.test").unwrap(),
        &mut conn,
    )
    .await
    .unwrap();
    let accepter = User::create(
        NewUser::new("accepter", "password", "accepter@example.test").unwrap(),
        &mut conn,
    )
    .await
    .unwrap();

    for (raw, displayed) in [
        (1499.6, 1500),
        (1499.4, 1499),
        (1499.5, 1500),
        (1500.4, 1500),
        (1500.5, 1501),
    ] {
        diesel::update(
            ratings::table
                .filter(ratings::user_uid.eq(accepter.id))
                .filter(ratings::speed.eq(GameSpeed::Blitz.to_string())),
        )
        .set(ratings::rating.eq(raw))
        .execute(&mut conn)
        .await
        .unwrap();
        let rating = Rating::for_uuid(&accepter.id, &GameSpeed::Blitz, &mut conn)
            .await
            .unwrap();
        assert_eq!(RatingResponse::from_rating(&rating).rating, displayed);
        let new = NewChallenge::new(
            challenger.id,
            None,
            &ChallengeDetails {
                rated: true,
                game_type: GameType::MLP,
                visibility: ChallengeVisibility::Public,
                opponent: None,
                color_choice: ColorChoice::White,
                time_mode: TimeMode::RealTime,
                time_base: Some(300),
                time_increment: Some(0),
                band_lower: Some(1500),
                band_upper: Some(1500),
            },
            &mut conn,
        )
        .await
        .unwrap();
        let challenge = Challenge::create(&new, &mut conn).await.unwrap();
        let messages =
            AcceptHandler::new(ChallengeId(challenge.nanoid.clone()), accepter.id, &db.pool)
                .handle(&RealtimeGate::default())
                .await
                .unwrap();

        let created = diesel::select(exists(
            games::table.filter(games::nanoid.eq(&challenge.nanoid)),
        ))
        .get_result::<bool>(&mut conn)
        .await
        .unwrap();
        assert_eq!(created, displayed == 1500, "raw rating: {raw}");
        let errors: Vec<_> = messages
            .iter()
            .filter_map(|message| match &message.message {
                ServerMessage::Error(error) => Some(error.clone()),
                _ => None,
            })
            .collect();
        let expected = match displayed {
            1499 => Some(UiError::RatingBelowBand {
                rating: displayed.to_string(),
                limit: "1500".to_string(),
            }),
            1501 => Some(UiError::RatingAboveBand {
                rating: displayed.to_string(),
                limit: "1500".to_string(),
            }),
            _ => None,
        };
        assert_eq!(
            errors,
            expected
                .into_iter()
                .map(|error| error.encode())
                .collect::<Vec<_>>(),
            "raw rating: {raw}",
        );
    }
}
