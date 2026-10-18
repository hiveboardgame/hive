use super::tournament::{
    create_rr_tournament_with_configuration,
    create_user,
    fixed_instant,
    insert_unfrozen_memberships,
};
use chrono::{Duration, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    models::{Game, Tournament},
    schema::{tournaments, tournaments_users},
    tournaments::arena,
    DbConn,
};
use diesel::{prelude::*, update};
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameControl};
use shared_types::{
    tournament::{arena::Config as ArenaConfig, BotAdmission, Config, FormatConfig, RealtimeClock},
    TournamentStatus,
};
use std::num::NonZeroU32;
use tournamint::arena::ArenaConfig as NativeArenaConfig;
use uuid::Uuid;

pub async fn start_arena_fixture(
    prefix: &str,
    player_count: usize,
    conn: &mut DbConn<'_>,
) -> (Tournament, Vec<Uuid>, Vec<Game>) {
    start_arena_fixture_with_rules(
        prefix,
        player_count,
        ArenaConfig::new(
            NonZeroU32::new(3600).unwrap(),
            RealtimeClock {
                base_seconds: NonZeroU32::new(180).unwrap(),
                increment_seconds: 1,
            },
        )
        .rules,
        conn,
    )
    .await
}

pub async fn start_arena_fixture_with_rules(
    prefix: &str,
    player_count: usize,
    rules: NativeArenaConfig,
    conn: &mut DbConn<'_>,
) -> (Tournament, Vec<Uuid>, Vec<Game>) {
    let organizer = create_user(&format!("{prefix}_org"), conn).await;
    let mut players = Vec::with_capacity(player_count);
    for index in 0..player_count {
        players.push(create_user(&format!("{prefix}_{index}"), conn).await.id);
    }
    let mut configuration = Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Arena(ArenaConfig::new(
            NonZeroU32::new(3_600).unwrap(),
            RealtimeClock {
                base_seconds: NonZeroU32::new(180).unwrap(),
                increment_seconds: 1,
            },
        )),
    };
    let tournament = create_rr_tournament_with_configuration(
        organizer.id,
        prefix,
        TournamentStatus::NotStarted,
        Some(Utc::now() + Duration::hours(1)),
        configuration.clone(),
        conn,
    )
    .await;
    // Stored rules may predate the creation preset; exercise reload behavior
    // without permitting clients to create arbitrary policies.
    let FormatConfig::Arena(arena_config) = &mut configuration.format else {
        unreachable!()
    };
    arena_config.rules = rules;
    update(tournaments::table.find(tournament.id))
        .set((
            tournaments::configuration.eq(serde_json::to_value(&configuration).unwrap()),
            tournaments::starts_at.eq(Some(Utc::now() - Duration::seconds(1))),
        ))
        .execute(conn)
        .await
        .expect("make Arena tournament due");
    insert_unfrozen_memberships(tournament.id, &players, fixed_instant(), conn).await;
    let present = players.clone();
    let started = arena::start_scheduled_with_presence(
        tournament.id,
        move |_| present.iter().copied().collect(),
        conn,
    )
    .await
    .expect("start Arena fixture");
    (started.tournament, players, started.new_games)
}

pub async fn elapse_rematch_wait(tournament_id: Uuid, conn: &mut DbConn<'_>) {
    update(
        tournaments_users::table
            .filter(tournaments_users::tournament_id.eq(tournament_id))
            .filter(tournaments_users::arena_waiting_since.is_not_null()),
    )
    .set(tournaments_users::arena_waiting_since.eq(Some(Utc::now() - Duration::seconds(50))))
    .execute(conn)
    .await
    .unwrap();
}

pub async fn finish_by_resignation(game: &Game, conn: &mut DbConn<'_>) -> Game {
    finish_by_resignation_with(game, conn, execute).await
}

pub async fn finish_by_resignation_with(
    game: &Game,
    conn: &mut DbConn<'_>,
    execute: impl AsyncFnOnce(Uuid, Command, &mut DbConn<'_>) -> Result<Outcome, DbError>,
) -> Game {
    let outcome = execute(
        game.id,
        Command::Control {
            user_id: game.white_id,
            control: GameControl::Resign(Color::White),
        },
        conn,
    )
    .await
    .expect("finish Arena Game by resignation");
    let Outcome::Applied { game, .. } = outcome else {
        panic!("Arena resignation updates the Game")
    };
    game
}
