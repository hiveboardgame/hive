use anyhow::{ensure, Context, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    models::{
        Game,
        NewTournament,
        NewUser,
        ScheduleOffer,
        Tournament,
        TournamentFinalOutcome,
        TournamentSlot,
        TournamentSwissRound,
        TournamentUser,
        User,
    },
    schema::{games as games_table, tournaments},
    tournaments::{arena, fixed_field},
    DbConn,
};
use diesel::{
    prelude::*,
    sql_query,
    sql_types::{BigInt, Integer, Text, Uuid as SqlUuid},
};
use diesel_async::{AsyncConnection, RunQueryDsl, SimpleAsyncConnection};
use hive_lib::{Color, Direction, GameControl, GameStatus, Position, Turn};
use rand::{rngs::StdRng, seq::SliceRandom, RngExt, SeedableRng};
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        elimination::{
            ClinchPolicy as EliminationClinchPolicy,
            Config as EliminationConfig,
            EntrantSide as EliminationEntrantSide,
            SeriesPhase as EliminationSeriesPhase,
            SeriesPlan as EliminationSeriesPlan,
            SetLimit as EliminationSetLimit,
            Topology as EliminationTopology,
        },
        round_robin::{
            creation_tiebreakers as round_robin_creation_tiebreakers,
            Config as RoundRobinConfig,
            Criterion as RoundRobinCriterion,
            PrimaryScore as RoundRobinPrimaryScore,
        },
        swiss::{
            swiss_creation_tiebreakers,
            Config as SwissConfig,
            Criterion as SwissCriterion,
            PrimaryScore as DoubleSwissPrimaryScore,
        },
        BotAdmission,
        Clock,
        Config,
        CorrespondenceClock,
        Format,
        FormatConfig,
        RealtimeClock,
        ReleasePolicy,
        SlotKey,
        TournamentPairingNumberOrder,
        MAX_ROUND_ROBIN_REPEATS,
        MAX_ROUND_ROBIN_SEATS,
    },
    Conclusion,
    GameStart,
    ScheduleOfferStatus,
    TournamentDetails,
    TournamentGameResult,
    TournamentStatus,
};
use std::{
    collections::{HashMap, HashSet},
    num::{NonZeroU16, NonZeroU32},
};
use tournamint::swiss::SwissLeg;
use uuid::Uuid;

pub const ACCOUNT_PREFIX: &str = "zz_qa_t_";
pub const QA_PASSWORD: &str = "hivegame";
const ALLOWED_DATABASE: &str = "hive-local";
const FIXTURE_DESCRIPTION: &str = "Synthetic tournament for local manual QA.";
const QA_PLAYER_COUNT: usize = 160;
const QA_ACCOUNT_COUNT: usize = QA_PLAYER_COUNT + 2;
const MAX_ROUND_ROBIN_ENTRANTS: usize = MAX_ROUND_ROBIN_SEATS as usize;
const MAX_ROUND_ROBIN_GAMES: usize = MAX_ROUND_ROBIN_ENTRANTS * (MAX_ROUND_ROBIN_ENTRANTS - 1) / 2
    * MAX_ROUND_ROBIN_REPEATS as usize;
const MAX_ROUND_ROBIN_RESULTS_PER_PLAYER: usize =
    (MAX_ROUND_ROBIN_ENTRANTS - 1) * MAX_ROUND_ROBIN_REPEATS as usize;
const LARGE_ROUND_ROBIN_ENTRANTS: usize = MAX_ROUND_ROBIN_SEATS as usize - 1;
const MEDIUM_SWISS_ENTRANTS: usize = 33;
const MEDIUM_ELIMINATION_ENTRANTS: usize = 33;
const LARGE_ELIMINATION_ENTRANTS: usize = 63;
const FUTURE_SWISS_ENTRANTS: usize = 96;
const FUTURE_DOUBLE_SWISS_ENTRANTS: usize = 65;
const FUTURE_ELIMINATION_ENTRANTS: usize = 64;
const FUTURE_ARENA_ENTRANTS: usize = 120;
const LIVE_ARENA_ENTRANTS: usize = 40;
const FINISHED_ARENA_ENTRANTS: usize = 24;
const FAR_FUTURE_ARENA_DAYS: i64 = 365;
const DUE_FIXTURE_NAME: &str = "QA Due Undersubscribed";
const MAX_ROUND_ROBIN_FIXTURE_NAME: &str = "QA Finished Maximum Round Robin";
const MAX_ROUND_ROBIN_OUTCOME_SALT: usize = 0x16_06_90;
const DUE_TRANSITION_SECONDS: i64 = 65;
const OPERATOR_LOCK_KEY: i64 = 0x4849_5645_5141_5631;

#[derive(QueryableByName)]
struct DatabaseName {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct RemainingReference {
    #[diesel(sql_type = Text)]
    source: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct OutsideTournamentMembership {
    #[diesel(sql_type = SqlUuid)]
    user_id: Uuid,
    #[diesel(sql_type = Text)]
    username: String,
    #[diesel(sql_type = Text)]
    email: String,
    #[diesel(sql_type = SqlUuid)]
    tournament_id: Uuid,
    #[diesel(sql_type = Text)]
    tournament_nanoid: String,
    #[diesel(sql_type = Text)]
    tournament_name: String,
}

struct CleanupSummary {
    games_deleted: usize,
    tournaments_deleted: usize,
    series_deleted: usize,
    accounts_deleted: usize,
}

#[derive(QueryableByName)]
struct DueFixture {
    #[diesel(sql_type = SqlUuid)]
    id: Uuid,
    #[diesel(sql_type = Integer)]
    min_seats: i32,
    #[diesel(sql_type = BigInt)]
    entrants: i64,
}

#[derive(Clone)]
struct QaUser {
    id: Uuid,
    username: String,
}

struct Fixture {
    tournament: Tournament,
    expected: &'static str,
    checks: &'static str,
}

pub async fn seed(
    conn: &mut DbConn<'_>,
    confirmed: &str,
    app_url: &str,
    requested_seed: Option<u64>,
) -> Result<()> {
    confirm_local_database(conn, confirmed).await?;
    let app_url = app_url.trim_end_matches('/').to_owned();
    let seed = requested_seed.unwrap_or_else(rand::random);

    let (users, fixtures) = conn
        .transaction::<_, anyhow::Error, _>(async move |tc| {
            tc.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .await?;
            seed_transaction(tc, seed).await
        })
        .await
        .context("synthetic tournament seed rolled back")?;

    println!("Synthetic tournament QA seed complete.");
    println!("Random fixture seed: {seed} (reproduce with --seed {seed})");
    println!("QA password (all accounts): {QA_PASSWORD}");
    for user in users {
        println!("account: {} ({})", user.username, user.id);
    }
    for fixture in fixtures {
        let starts_at = fixture
            .tournament
            .starts_at
            .map(|starts_at| starts_at.to_rfc3339())
            .unwrap_or_else(|| String::from("organizer start"));
        println!(
            "tournament: {} | {}/tournament/{} | starts_at: {} | expected: {} | check: {}",
            fixture.tournament.id,
            app_url,
            fixture.tournament.nanoid,
            starts_at,
            fixture.expected,
            fixture.checks,
        );
    }
    println!(
        "Cleanup: cargo run -p script -- synthetic-tournaments cleanup --confirm-database-name {ALLOWED_DATABASE}"
    );
    Ok(())
}

pub async fn cleanup(conn: &mut DbConn<'_>, confirmed: &str) -> Result<()> {
    confirm_local_database(conn, confirmed).await?;
    let summary = conn
        .transaction::<_, anyhow::Error, _>(async move |tc| cleanup_transaction(tc).await)
        .await
        .context("synthetic tournament cleanup rolled back")?;
    println!(
        "Synthetic cleanup complete: removed {} cohort tournaments, {} cohort series, \
         {} linked games, and {} QA accounts; unrelated rows were preserved.",
        summary.tournaments_deleted,
        summary.series_deleted,
        summary.games_deleted,
        summary.accounts_deleted,
    );
    Ok(())
}

pub async fn arm_due(conn: &mut DbConn<'_>, confirmed: &str, app_url: &str) -> Result<()> {
    confirm_local_database(conn, confirmed).await?;
    let tournament = conn
        .transaction::<_, anyhow::Error, _>(async move |tc| arm_due_transaction(tc).await)
        .await
        .context("undersubscribed due-fixture re-arm rolled back")?;
    let starts_at = tournament
        .starts_at
        .context("re-armed due fixture has no scheduled start")?;
    println!(
        "Re-armed {DUE_FIXTURE_NAME}: {}/tournament/{} becomes due at {}. Open the page now; it will enter the due waiting state before the scheduled worker clears starts_at on its following sweep.",
        app_url.trim_end_matches('/'),
        tournament.nanoid,
        starts_at.to_rfc3339(),
    );
    Ok(())
}

async fn acquire_operator_lock(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query("select pg_advisory_xact_lock($1)")
        .bind::<BigInt, _>(OPERATOR_LOCK_KEY)
        .execute(conn)
        .await?;
    Ok(())
}

async fn synthetic_organizer(conn: &mut DbConn<'_>) -> Result<User> {
    User::find_by_username(&format!("{ACCOUNT_PREFIX}organizer"), conn)
        .await
        .context("synthetic organizer account is missing; seed the fixture cohort first")
}

async fn seed_transaction(conn: &mut DbConn<'_>, seed: u64) -> Result<(Vec<QaUser>, Vec<Fixture>)> {
    acquire_operator_lock(conn).await?;
    ensure_fixture_registry(conn).await?;
    refuse_conflicts(conn).await?;
    insert_fixture_data(conn, seed).await
}

async fn cleanup_transaction(conn: &mut DbConn<'_>) -> Result<CleanupSummary> {
    acquire_operator_lock(conn).await?;
    delete_fixture_data(conn).await
}

async fn arm_due_transaction(conn: &mut DbConn<'_>) -> Result<Tournament> {
    acquire_operator_lock(conn).await?;
    let organizer = synthetic_organizer(conn).await?;
    let candidates = sql_query(
        "select t.id, t.min_seats, count(tu.user_id)::bigint as entrants \
         from tournaments t \
         left join tournaments_users tu on tu.tournament_id = t.id \
         where exists ( \
             select 1 from tournaments_organizers tor \
             where tor.tournament_id = t.id and tor.organizer_id = $1 \
         ) \
           and t.name = $2 \
           and t.started_at is null \
           and t.finished_at is null \
         group by t.id, t.min_seats",
    )
    .bind::<SqlUuid, _>(organizer.id)
    .bind::<Text, _>(DUE_FIXTURE_NAME)
    .load::<DueFixture>(conn)
    .await?;
    ensure!(
        candidates.len() == 1,
        "expected exactly one {DUE_FIXTURE_NAME:?} for the synthetic organizer; seed or clean the fixture set first"
    );
    let fixture = &candidates[0];
    ensure!(
        fixture.entrants < i64::from(fixture.min_seats),
        "the due fixture is no longer undersubscribed"
    );
    let starts_at = Utc::now() + Duration::seconds(DUE_TRANSITION_SECONDS);
    Ok(diesel::update(tournaments::table.find(fixture.id))
        .set(tournaments::starts_at.eq(Some(starts_at)))
        .get_result(conn)
        .await?)
}

async fn delete_fixture_data(conn: &mut DbConn<'_>) -> Result<CleanupSummary> {
    create_cleanup_snapshots(conn).await?;
    preflight_cleanup(conn).await?;

    delete_chat_data(conn).await?;
    delete_email_data(conn).await?;
    let games_deleted = delete_game_data(conn).await?;
    let tournaments_deleted = delete_tournament_data(conn).await?;
    let series_deleted = delete_series_data(conn).await?;
    let accounts_deleted = delete_account_data(conn).await?;
    verify_cleanup(conn).await?;

    Ok(CleanupSummary {
        games_deleted,
        tournaments_deleted,
        series_deleted,
        accounts_deleted,
    })
}

async fn preflight_cleanup(conn: &mut DbConn<'_>) -> Result<()> {
    let outside_memberships = sql_query(
        "select u.id as user_id, u.username, u.email, \
                t.id as tournament_id, t.nanoid as tournament_nanoid, \
                t.name as tournament_name \
         from (select tournament_id, user_id from tournaments_users \
               union select tournament_id, organizer_id as user_id from tournaments_organizers) tu \
         join synthetic_cleanup_users qu on qu.id = tu.user_id \
         join users u on u.id = tu.user_id \
         join tournaments t on t.id = tu.tournament_id \
         where not exists ( \
             select 1 from synthetic_cleanup_tournaments qt \
             where qt.id = tu.tournament_id \
         ) \
         order by u.username, t.nanoid",
    )
    .load::<OutsideTournamentMembership>(conn)
    .await?;
    ensure!(
        outside_memberships.is_empty(),
        "refusing synthetic cleanup: QA account membership(s) belong to tournaments outside the captured synthetic cohort: {}",
        outside_memberships
            .iter()
            .map(|membership| format!(
                "{} <{}> ({}) -> {} ({}, {})",
                membership.username,
                membership.email,
                membership.user_id,
                membership.tournament_name,
                membership.tournament_nanoid,
                membership.tournament_id,
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let outside_series = sql_query(
        "select count(*)::bigint as count from tournament_series_organizers \
         where organizer_id in (select id from synthetic_cleanup_users) \
           and tournament_series_id not in (select id from synthetic_cleanup_series)",
    )
    .get_result::<Count>(conn)
    .await?
    .count;
    ensure!(outside_series == 0, "refusing synthetic cleanup: QA accounts organize {outside_series} series shared with non-QA accounts or tournaments; remove those organizer memberships deliberately first");
    Ok(())
}

async fn delete_chat_data(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "delete from chat_read_receipts \
         where user_id in (select id from synthetic_cleanup_users) \
            or channel_id in ( \
                select id from chat_channels \
                where direct_user_low_id in (select id from synthetic_cleanup_users) \
                   or direct_user_high_id in (select id from synthetic_cleanup_users) \
                   or game_id in (select id from synthetic_cleanup_games) \
                   or tournament_id in (select id from synthetic_cleanup_tournaments) \
            )",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from chat_messages \
         where sender_id in (select id from synthetic_cleanup_users) \
            or channel_id in ( \
                select id from chat_channels \
                where direct_user_low_id in (select id from synthetic_cleanup_users) \
                   or direct_user_high_id in (select id from synthetic_cleanup_users) \
                   or game_id in (select id from synthetic_cleanup_games) \
                   or tournament_id in (select id from synthetic_cleanup_tournaments) \
            )",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from chat_channels \
         where direct_user_low_id in (select id from synthetic_cleanup_users) \
            or direct_user_high_id in (select id from synthetic_cleanup_users) \
            or game_id in (select id from synthetic_cleanup_games) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn delete_email_data(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "delete from email_queue \
         where user_id in (select id from synthetic_cleanup_users) \
            or lower(to_address) in (select address from synthetic_cleanup_emails)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from email_request_log \
         where lower(email) in (select address from synthetic_cleanup_emails)",
    )
    .execute(conn)
    .await?;
    sql_query("delete from email_tokens where user_id in (select id from synthetic_cleanup_users)")
        .execute(conn)
        .await?;
    Ok(())
}

async fn delete_game_data(conn: &mut DbConn<'_>) -> Result<usize> {
    sql_query(
        "delete from challenges \
         where challenger_id in (select id from synthetic_cleanup_users) \
            or opponent_id in (select id from synthetic_cleanup_users)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from schedule_offers \
         where tournament_id in (select id from synthetic_cleanup_tournaments) \
            or proposer_id in (select id from synthetic_cleanup_users) \
            or resolved_by in (select id from synthetic_cleanup_users)",
    )
    .execute(conn)
    .await?;
    sql_query("delete from game_hashes where game_id in (select id from synthetic_cleanup_games)")
        .execute(conn)
        .await?;
    sql_query(
        "delete from games_users \
         where game_id in (select id from synthetic_cleanup_games) \
            or user_id in (select id from synthetic_cleanup_users)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournament_final_arena_results \
         where game_id in (select id from synthetic_cleanup_games)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from arena_game_results \
         where game_id in (select id from synthetic_cleanup_games)",
    )
    .execute(conn)
    .await?;
    let games_deleted =
        sql_query("delete from games where id in (select id from synthetic_cleanup_games)")
            .execute(conn)
            .await?;
    Ok(games_deleted)
}

async fn delete_tournament_data(conn: &mut DbConn<'_>) -> Result<usize> {
    delete_tournament_facts(conn).await?;
    delete_tournament_relationships(conn).await
}

async fn delete_tournament_facts(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "delete from tournament_final_outcomes \
         where tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournament_elimination_nodes \
         where tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournament_swiss_rounds \
         where tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournament_slots \
         where tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn delete_tournament_relationships(conn: &mut DbConn<'_>) -> Result<usize> {
    sql_query(
        "delete from tournaments_organizer_invitations \
         where invitee_id in (select id from synthetic_cleanup_users) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from user_tournament_chat_mutes \
         where user_id in (select id from synthetic_cleanup_users) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournaments_invitations \
         where invitee_id in (select id from synthetic_cleanup_users) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournaments_organizers \
         where organizer_id in (select id from synthetic_cleanup_users) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "delete from tournaments_users \
         where user_id in (select id from synthetic_cleanup_users) \
            or tournament_id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    let tournaments_deleted = sql_query(
        "delete from tournaments where id in (select id from synthetic_cleanup_tournaments)",
    )
    .execute(conn)
    .await?;
    Ok(tournaments_deleted)
}

async fn delete_series_data(conn: &mut DbConn<'_>) -> Result<usize> {
    sql_query(
        "delete from tournament_series_organizers \
         where organizer_id in (select id from synthetic_cleanup_users) \
            or tournament_series_id in (select id from synthetic_cleanup_series)",
    )
    .execute(conn)
    .await?;
    let series_deleted = sql_query(
        "delete from tournament_series where id in (select id from synthetic_cleanup_series)",
    )
    .execute(conn)
    .await?;
    Ok(series_deleted)
}

async fn delete_account_data(conn: &mut DbConn<'_>) -> Result<usize> {
    sql_query(
        "delete from notification_preferences \
         where user_id in (select id from synthetic_cleanup_users)",
    )
    .execute(conn)
    .await?;
    sql_query("delete from push_devices where user_id in (select id from synthetic_cleanup_users)")
        .execute(conn)
        .await?;
    sql_query("delete from ratings where user_uid in (select id from synthetic_cleanup_users)")
        .execute(conn)
        .await?;
    sql_query(
        "delete from user_blocks \
         where blocker_id in (select id from synthetic_cleanup_users) \
            or blocked_id in (select id from synthetic_cleanup_users)",
    )
    .execute(conn)
    .await?;
    let accounts_deleted =
        sql_query("delete from users where id in (select id from synthetic_cleanup_users)")
            .execute(conn)
            .await?;
    Ok(accounts_deleted)
}

async fn create_cleanup_snapshots(conn: &mut DbConn<'_>) -> Result<()> {
    ensure_fixture_registry(conn).await?;
    reset_cleanup_snapshot_tables(conn).await?;
    capture_cleanup_accounts(conn).await?;
    capture_cleanup_tournaments(conn).await?;
    capture_cleanup_games(conn).await?;
    Ok(())
}

// Fixture identity must survive account anonymization and organizer changes.
// Keep this operator-only metadata outside the application's public schema.
async fn ensure_fixture_registry(conn: &mut DbConn<'_>) -> Result<()> {
    conn.batch_execute(
        "create schema if not exists synthetic_tournament_qa; \
         create table if not exists synthetic_tournament_qa.accounts ( \
             id uuid primary key references public.users(id) on delete cascade \
         ); \
         create table if not exists synthetic_tournament_qa.tournaments ( \
             id uuid primary key references public.tournaments(id) on delete cascade \
         );",
    )
    .await?;
    Ok(())
}

async fn reset_cleanup_snapshot_tables(conn: &mut DbConn<'_>) -> Result<()> {
    conn.batch_execute(
        "create temporary table if not exists synthetic_cleanup_users ( \
             id uuid primary key, \
             email text not null, \
             pending_email text \
         ) on commit drop; \
         create temporary table if not exists synthetic_cleanup_emails ( \
             address text primary key \
         ) on commit drop; \
         create temporary table if not exists synthetic_cleanup_series ( \
             id uuid primary key \
         ) on commit drop; \
         create temporary table if not exists synthetic_cleanup_tournaments ( \
             id uuid primary key \
         ) on commit drop; \
         create temporary table if not exists synthetic_cleanup_games ( \
             id uuid primary key \
         ) on commit drop; \
         truncate synthetic_cleanup_users, synthetic_cleanup_emails, \
         synthetic_cleanup_series, synthetic_cleanup_tournaments, synthetic_cleanup_games;",
    )
    .await?;
    Ok(())
}

async fn capture_cleanup_accounts(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "insert into synthetic_cleanup_users (id, email, pending_email) \
         select id, email, pending_email \
         from users \
         where id in (select id from synthetic_tournament_qa.accounts)",
    )
    .execute(conn)
    .await?;
    sql_query(
        "insert into synthetic_cleanup_emails (address) \
         select lower(email) from synthetic_cleanup_users \
         union \
         select lower(pending_email) from synthetic_cleanup_users where pending_email is not null",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn capture_cleanup_tournaments(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "insert into synthetic_cleanup_series (id) \
         select ts.id \
         from tournament_series ts \
         where exists ( \
             select 1 \
             from tournament_series_organizers tso \
             join synthetic_cleanup_users qu on qu.id = tso.organizer_id \
             where tso.tournament_series_id = ts.id \
         ) and not exists ( \
             select 1 from tournament_series_organizers tso \
             where tso.tournament_series_id = ts.id \
               and tso.organizer_id not in (select id from synthetic_cleanup_users) \
         ) and not exists ( \
             select 1 from tournaments t where t.series = ts.id \
               and t.id not in (select id from synthetic_tournament_qa.tournaments) \
         )",
    )
    .execute(conn)
    .await?;
    sql_query(
        "insert into synthetic_cleanup_tournaments (id) \
         select t.id \
         from tournaments t \
         where t.id in (select id from synthetic_tournament_qa.tournaments)",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn capture_cleanup_games(conn: &mut DbConn<'_>) -> Result<()> {
    sql_query(
        "insert into synthetic_cleanup_games (id) \
         select g.id \
         from games g \
         where g.tournament_id in (select id from synthetic_cleanup_tournaments) \
            or g.current_player_id in (select id from synthetic_cleanup_users) \
            or g.white_id in (select id from synthetic_cleanup_users) \
            or g.black_id in (select id from synthetic_cleanup_users) \
            or exists ( \
                select 1 from games_users gu \
                join synthetic_cleanup_users qu on qu.id = gu.user_id \
                where gu.game_id = g.id \
            )",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn verify_cleanup(conn: &mut DbConn<'_>) -> Result<()> {
    let remaining = sql_query(
        "select source, count \
         from ( \
             select 'users'::text as source, count(*)::bigint as count from users where id in (select id from synthetic_cleanup_users) \
             union all select 'usernames', count(*)::bigint from users where left(username, length($1)) = $1 \
             union all select 'challenges', count(*)::bigint from challenges where challenger_id in (select id from synthetic_cleanup_users) or opponent_id in (select id from synthetic_cleanup_users) \
             union all select 'chat_channels', count(*)::bigint from chat_channels where direct_user_low_id in (select id from synthetic_cleanup_users) or direct_user_high_id in (select id from synthetic_cleanup_users) or game_id in (select id from synthetic_cleanup_games) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'chat_messages', count(*)::bigint from chat_messages where sender_id in (select id from synthetic_cleanup_users) \
             union all select 'chat_read_receipts', count(*)::bigint from chat_read_receipts where user_id in (select id from synthetic_cleanup_users) \
             union all select 'email_queue', count(*)::bigint from email_queue where user_id in (select id from synthetic_cleanup_users) or lower(to_address) in (select address from synthetic_cleanup_emails) \
             union all select 'email_request_log', count(*)::bigint from email_request_log where lower(email) in (select address from synthetic_cleanup_emails) \
             union all select 'email_tokens', count(*)::bigint from email_tokens where user_id in (select id from synthetic_cleanup_users) \
             union all select 'games', count(*)::bigint from games where id in (select id from synthetic_cleanup_games) or current_player_id in (select id from synthetic_cleanup_users) or white_id in (select id from synthetic_cleanup_users) or black_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'game_hashes', count(*)::bigint from game_hashes where game_id in (select id from synthetic_cleanup_games) \
             union all select 'games_users', count(*)::bigint from games_users where game_id in (select id from synthetic_cleanup_games) or user_id in (select id from synthetic_cleanup_users) \
             union all select 'notification_preferences', count(*)::bigint from notification_preferences where user_id in (select id from synthetic_cleanup_users) \
             union all select 'push_devices', count(*)::bigint from push_devices where user_id in (select id from synthetic_cleanup_users) \
             union all select 'ratings', count(*)::bigint from ratings where user_uid in (select id from synthetic_cleanup_users) \
             union all select 'schedule_offers', count(*)::bigint from schedule_offers where proposer_id in (select id from synthetic_cleanup_users) or resolved_by in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournament_elimination_nodes', count(*)::bigint from tournament_elimination_nodes where tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournament_final_outcomes', count(*)::bigint from tournament_final_outcomes where tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournament_series', count(*)::bigint from tournament_series where id in (select id from synthetic_cleanup_series) \
             union all select 'tournament_series_organizers', count(*)::bigint from tournament_series_organizers where organizer_id in (select id from synthetic_cleanup_users) or tournament_series_id in (select id from synthetic_cleanup_series) \
             union all select 'tournament_slots', count(*)::bigint from tournament_slots where tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournament_swiss_rounds', count(*)::bigint from tournament_swiss_rounds where tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournaments', count(*)::bigint from tournaments where id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournaments_invitations', count(*)::bigint from tournaments_invitations where invitee_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournaments_organizer_invitations', count(*)::bigint from tournaments_organizer_invitations where invitee_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournaments_organizers', count(*)::bigint from tournaments_organizers where organizer_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'tournaments_users', count(*)::bigint from tournaments_users where user_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
             union all select 'user_blocks', count(*)::bigint from user_blocks where blocker_id in (select id from synthetic_cleanup_users) or blocked_id in (select id from synthetic_cleanup_users) \
             union all select 'user_tournament_chat_mutes', count(*)::bigint from user_tournament_chat_mutes where user_id in (select id from synthetic_cleanup_users) or tournament_id in (select id from synthetic_cleanup_tournaments) \
         ) remaining_counts \
         where count <> 0",
    )
    .bind::<Text, _>(ACCOUNT_PREFIX)
    .load::<RemainingReference>(conn)
    .await?;
    ensure!(
        remaining.is_empty(),
        "synthetic cleanup left references behind: {}",
        remaining
            .iter()
            .map(|reference| format!("{}={}", reference.source, reference.count))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

async fn confirm_local_database(conn: &mut DbConn<'_>, confirmed: &str) -> Result<()> {
    ensure!(
        confirmed == ALLOWED_DATABASE,
        "refusing: --confirm-database-name must be exactly {ALLOWED_DATABASE:?}"
    );
    let actual = sql_query("select current_database()::text as name")
        .get_result::<DatabaseName>(conn)
        .await
        .context("could not read current_database()")?;
    ensure!(
        actual.name == ALLOWED_DATABASE && actual.name == confirmed,
        "refusing: connected database is {:?}, expected explicitly confirmed local database {:?}",
        actual.name,
        ALLOWED_DATABASE,
    );
    Ok(())
}

async fn refuse_conflicts(conn: &mut DbConn<'_>) -> Result<()> {
    let accounts = sql_query(
        "select count(*)::bigint as count from users \
         where left(username, length($1)) = $1 \
            or id in (select id from synthetic_tournament_qa.accounts)",
    )
    .bind::<Text, _>(ACCOUNT_PREFIX)
    .get_result::<Count>(conn)
    .await?
    .count;
    ensure!(
        accounts == 0,
        "refusing to seed over {accounts} existing synthetic accounts; run the cleanup command first"
    );
    Ok(())
}

async fn insert_fixture_data(
    conn: &mut DbConn<'_>,
    seed: u64,
) -> Result<(Vec<QaUser>, Vec<Fixture>)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let password_hash = Argon2::default()
        .hash_password(
            QA_PASSWORD.as_bytes(),
            &SaltString::encode_b64(b"hive-synthetic-qa-v1")
                .map_err(|error| anyhow::anyhow!("could not build QA password salt: {error}"))?,
        )
        .map_err(|error| anyhow::anyhow!("could not hash QA password: {error}"))?
        .to_string();
    let mut users = Vec::with_capacity(QA_ACCOUNT_COUNT);
    let suffixes = std::iter::once(String::from("organizer"))
        .chain((1..=QA_PLAYER_COUNT).map(|index| format!("player_{index:02}")))
        .chain(std::iter::once(String::from("delete_test")));
    for suffix in suffixes {
        let username = format!("{ACCOUNT_PREFIX}{suffix}");
        let user = User::create(
            NewUser::new(
                &username,
                &password_hash,
                &format!("{username}@example.test"),
            )?,
            conn,
        )
        .await?;
        sql_query("insert into synthetic_tournament_qa.accounts (id) values ($1)")
            .bind::<SqlUuid, _>(user.id)
            .execute(conn)
            .await?;
        users.push(QaUser {
            id: user.id,
            username,
        });
    }
    let organizer = users[0].id;
    let players = users[1..=QA_PLAYER_COUNT]
        .iter()
        .map(|user| user.id)
        .collect::<Vec<_>>();
    let invite_player = sample_players(&players, 1, &mut rng)[0];
    let future_count = 12;
    let future_capacity = 16;
    let future_players = sample_players(&players, future_count, &mut rng);
    let future_swiss_players = sample_players(&players, FUTURE_SWISS_ENTRANTS, &mut rng);
    let future_double_swiss_players =
        sample_players(&players, FUTURE_DOUBLE_SWISS_ENTRANTS, &mut rng);
    let future_single_elimination_players =
        sample_players(&players, FUTURE_ELIMINATION_ENTRANTS, &mut rng);
    let future_double_elimination_players =
        sample_players(&players, FUTURE_ELIMINATION_ENTRANTS, &mut rng);
    let future_arena_players = sample_players(&players, FUTURE_ARENA_ENTRANTS, &mut rng);
    let due_players = sample_players(&players, 2, &mut rng);
    let live_rr_players = sample_players(&players, LARGE_ROUND_ROBIN_ENTRANTS, &mut rng);
    let live_correspondence_rr_players =
        sample_players(&players, LARGE_ROUND_ROBIN_ENTRANTS, &mut rng);
    let swiss_players = sample_players(&players, QA_PLAYER_COUNT, &mut rng);
    let double_swiss_players = sample_players(&players, MEDIUM_SWISS_ENTRANTS, &mut rng);
    let single_elimination_players =
        sample_players(&players, MEDIUM_ELIMINATION_ENTRANTS, &mut rng);
    let double_elimination_players = sample_players(&players, LARGE_ELIMINATION_ENTRANTS, &mut rng);
    let live_arena_players = sample_players(&players, LIVE_ARENA_ENTRANTS, &mut rng);
    let finished_arena_players = sample_players(&players, FINISHED_ARENA_ENTRANTS, &mut rng);
    let finished_realtime_rr_players = sample_players(&players, 7, &mut rng);
    let finished_correspondence_rr_players =
        sample_players(&players, LARGE_ROUND_ROBIN_ENTRANTS, &mut rng);
    let finished_max_rr_players = players
        .iter()
        .copied()
        .take(MAX_ROUND_ROBIN_ENTRANTS)
        .collect::<Vec<_>>();
    let finished_swiss_players = sample_players(&players, 17, &mut rng);
    let finished_double_swiss_players = sample_players(&players, 21, &mut rng);
    let finished_single_elimination_players = sample_players(&players, 15, &mut rng);
    let finished_double_elimination_players = sample_players(&players, 31, &mut rng);
    let live_rr_repeats = NonZeroU32::new(rng.random_range(1..=2)).unwrap();
    let live_correspondence_rr_repeats =
        NonZeroU32::new(if live_rr_repeats.get() == 1 { 2 } else { 1 }).unwrap();
    let swiss_rounds = NonZeroU32::new(7).unwrap();
    let double_swiss_rounds = NonZeroU32::new(6).unwrap();
    let finished_swiss_rounds = NonZeroU32::new(6).unwrap();
    let finished_double_swiss_rounds = NonZeroU32::new(6).unwrap();
    let invite_clock = random_realtime_clock(&mut rng);
    let future_clock = random_correspondence_days_clock(&mut rng);
    let future_swiss_clock = random_realtime_clock(&mut rng);
    let future_double_swiss_clock = random_correspondence_days_clock(&mut rng);
    let future_single_elimination_clock = random_realtime_clock(&mut rng);
    let future_double_elimination_clock = random_correspondence_days_clock(&mut rng);
    let future_arena_clock = random_realtime_clock(&mut rng);
    let due_clock = random_realtime_clock(&mut rng);
    let live_rr_clock = random_realtime_clock(&mut rng);
    let live_correspondence_rr_clock = random_correspondence_days_clock(&mut rng);
    let swiss_clock = random_realtime_clock(&mut rng);
    let double_swiss_clock = random_realtime_clock(&mut rng);
    let single_elimination_clock = random_realtime_clock(&mut rng);
    let double_elimination_clock = random_correspondence_days_clock(&mut rng);
    let finished_realtime_rr_clock = random_realtime_clock(&mut rng);
    let finished_correspondence_rr_clock = random_correspondence_days_clock(&mut rng);
    let finished_swiss_clock = random_realtime_clock(&mut rng);
    let finished_double_swiss_clock = random_realtime_clock(&mut rng);
    let finished_single_elimination_clock = random_realtime_clock(&mut rng);
    let finished_double_elimination_clock = random_correspondence_days_clock(&mut rng);
    let mut tiebreak_rng = StdRng::seed_from_u64(seed ^ 0x71E_B4EA_C0DE);
    let round_robin_tiebreakers = round_robin_creation_tiebreakers(
        NonZeroU32::new(1).unwrap(),
        RoundRobinPrimaryScore::GamePoints,
    );
    let invite_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 2, &mut tiebreak_rng);
    let mut future_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 3, &mut tiebreak_rng);
    future_rr_tiebreakers.retain(|criterion| !criterion.is_seed());
    future_rr_tiebreakers.push(RoundRobinCriterion::TournamentPairingNumber(
        TournamentPairingNumberOrder::Descending,
    ));
    let due_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 4, &mut tiebreak_rng);
    let live_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 5, &mut tiebreak_rng);
    let live_correspondence_rr_tiebreakers = sample_creation_tiebreakers(
        &round_robin_tiebreakers,
        round_robin_tiebreakers.len(),
        &mut tiebreak_rng,
    );
    let finished_realtime_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 2, &mut tiebreak_rng);
    let finished_correspondence_rr_tiebreakers =
        sample_creation_tiebreakers(&round_robin_tiebreakers, 3, &mut tiebreak_rng);
    let swiss_tiebreakers = swiss_creation_tiebreakers(None);
    let future_swiss_tiebreakers =
        sample_creation_tiebreakers(&swiss_tiebreakers, 3, &mut tiebreak_rng);
    let live_swiss_tiebreakers =
        sample_creation_tiebreakers(&swiss_tiebreakers, 2, &mut tiebreak_rng);
    let finished_swiss_tiebreakers = sample_creation_tiebreakers(
        &swiss_tiebreakers,
        swiss_tiebreakers.len(),
        &mut tiebreak_rng,
    );
    let outcome_salt = seed as usize;
    let mut fixtures = Vec::new();

    let invite = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Organizer Invite Rating"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(4),
            min_seats: 2,
            invite_only: true,
            band_upper: Some(1600),
            band_lower: Some(1400),
            starts_at: None,
            configuration: rr_config_with_clock(
                ReleasePolicy::FullyUnlocked,
                NonZeroU32::new(1).unwrap(),
                invite_clock,
                &invite_rr_tiebreakers,
            ),
        },
        conn,
    )
    .await?;
    invite
        .create_invitation(&organizer, &invite_player, conn)
        .await?;
    invite.accept_invitation(&invite_player, conn).await?;
    fixtures.push(fixture(
        invite,
        "NotStarted; invite-only; 1400-1600 rating band",
        "Invitation acceptance and creation restrictions are visible.",
    ));

    let future = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Future Correspondence"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(future_capacity),
            min_seats: future_count as i32,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::days(7)),
            configuration: rr_config_with_clock(
                ReleasePolicy::FullyUnlocked,
                NonZeroU32::new(1).unwrap(),
                future_clock,
                &future_rr_tiebreakers,
            ),
        },
        conn,
    )
    .await?;
    join(&future, &future_players, conn).await?;
    fixtures.push(fixture(
        future,
        "future scheduled correspondence Round Robin",
        "A many-player upcoming correspondence event has a future start and no games yet.",
    ));

    let future_swiss = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Unstarted Large Swiss"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(future_swiss_players.len() as i32),
            min_seats: 5,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: swiss_config_with_tiebreakers(
                swiss_rounds,
                future_swiss_clock,
                &future_swiss_tiebreakers,
            ),
        },
        conn,
    )
    .await?;
    join(&future_swiss, &future_swiss_players, conn).await?;
    fixtures.push(fixture(
        future_swiss,
        "unstarted realtime Swiss with 96 players",
        "The large registered field has no accepted rounds or games until the organizer starts it.",
    ));

    let future_double_swiss = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Future Large Double Swiss"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(future_double_swiss_players.len() as i32),
            min_seats: 5,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::days(30)),
            configuration: double_swiss_config_with_clock(
                double_swiss_rounds,
                future_double_swiss_clock,
                DoubleSwissPrimaryScore::GamePoints,
                ReleasePolicy::FullyUnlocked,
            ),
        },
        conn,
    )
    .await?;
    join(&future_double_swiss, &future_double_swiss_players, conn).await?;
    fixtures.push(fixture(
        future_double_swiss,
        "future scheduled correspondence Double Swiss with 65 players",
        "The large odd field is registered but has no accepted reciprocal round or bye until its scheduled start.",
    ));

    let future_single_elimination = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Unstarted Maximum Single Elimination"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(future_single_elimination_players.len() as i32),
            min_seats: future_single_elimination_players.len() as i32,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: random_elimination_config(
                EliminationTopology::Single { bronze: true },
                future_single_elimination_clock,
                &mut rng,
            ),
        },
        conn,
    )
    .await?;
    join(
        &future_single_elimination,
        &future_single_elimination_players,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        future_single_elimination,
        "unstarted maximum-size realtime single elimination with 64 players",
        "The full field is registered but no bracket nodes, slots, or games exist before organizer start.",
    ));

    let future_double_elimination = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Future Maximum Double Elimination"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(future_double_elimination_players.len() as i32),
            min_seats: future_double_elimination_players.len() as i32,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::days(90)),
            configuration: random_elimination_config(
                EliminationTopology::Double,
                future_double_elimination_clock,
                &mut rng,
            ),
        },
        conn,
    )
    .await?;
    join(
        &future_double_elimination,
        &future_double_elimination_players,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        future_double_elimination,
        "future scheduled maximum-size correspondence double elimination with 64 players",
        "The full field is registered but the winners and losers brackets remain unmaterialized before the scheduled start.",
    ));

    let Clock::Realtime(future_arena_game_clock) = future_arena_clock else {
        unreachable!("synthetic future Arena clock is not realtime")
    };
    let future_arena = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Far Future Large Arena"),
            description: None,
            seats: None,
            min_seats: 0,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::days(FAR_FUTURE_ARENA_DAYS)),
            configuration: Config {
                bot_admission: BotAdmission::HumansAndBots,
                format: FormatConfig::Arena(ArenaConfig::new(
                    NonZeroU32::new(5 * 60 * 60).unwrap(),
                    future_arena_game_clock,
                )),
            },
        },
        conn,
    )
    .await?;
    join(&future_arena, &future_arena_players, conn).await?;
    fixtures.push(fixture(
        future_arena,
        "far-future scheduled five-hour Arena with 120 pre-registered players",
        "The large field remains unsampled and unpaired until its start roughly one year in the future.",
    ));

    let due = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from(DUE_FIXTURE_NAME),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(6),
            min_seats: 4,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::seconds(DUE_TRANSITION_SECONDS)),
            configuration: rr_config_with_clock(
                ReleasePolicy::FullyUnlocked,
                NonZeroU32::new(1).unwrap(),
                due_clock,
                &due_rr_tiebreakers,
            ),
        },
        conn,
    )
    .await?;
    join(&due, &due_players, conn).await?;
    let due = Tournament::find(due.id, conn).await?;
    fixtures.push(fixture(due, "undersubscribed scheduled fixed-field becoming due shortly", "Open this page immediately to watch its future boundary become due; the worker then clears starts_at without creating games. Run synthetic-tournaments arm-due to repeat the check."));

    let rr = start_fixed(
        organizer,
        "QA Live Realtime Round Robin",
        rr_config_with_clock(
            ReleasePolicy::SequentialPerMatchup,
            live_rr_repeats,
            live_rr_clock,
            &live_rr_tiebreakers,
        ),
        &live_rr_players,
        conn,
    )
    .await?;
    populate_round_robin_states(&rr, organizer, &live_rr_players, true, conn).await?;
    fixtures.push(fixture(rr, "mid-tournament realtime Round Robin with 15 players", "Eight completed matchups precede scheduled, active, administratively resolved, untouched, and withdrawn states; the field is one below the 16-player cap."));

    let correspondence_rr = start_fixed(
        organizer,
        "QA Live Correspondence Round Robin",
        rr_config_with_clock(
            ReleasePolicy::SequentialPerMatchup,
            live_correspondence_rr_repeats,
            live_correspondence_rr_clock,
            &live_correspondence_rr_tiebreakers,
        ),
        &live_correspondence_rr_players,
        conn,
    )
    .await?;
    populate_round_robin_states(
        &correspondence_rr,
        organizer,
        &live_correspondence_rr_players,
        false,
        conn,
    )
    .await?;
    fixtures.push(fixture(correspondence_rr, "mid-tournament correspondence Round Robin with 15 players", "Eight completed matchups precede active, untouched, finished-game, and withdrawn states; one live Round Robin always has repeated pairings."));

    let swiss = start_swiss(
        organizer,
        "QA Live Swiss",
        swiss_config_with_tiebreakers(swiss_rounds, swiss_clock, &live_swiss_tiebreakers),
        &swiss_players,
        conn,
    )
    .await?;
    populate_mid_swiss(
        &swiss,
        organizer,
        2,
        outcome_salt.rotate_left(2),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        swiss,
        "mid-tournament realtime Swiss with 160 players",
        "The large usability field has two completed rounds plus a mixed third round with scheduled, active, administratively resolved, and untouched pairings.",
    ));
    let double_swiss = start_swiss(
        organizer,
        "QA Live Double Swiss Bye",
        double_swiss_config_with_clock(
            double_swiss_rounds,
            double_swiss_clock,
            DoubleSwissPrimaryScore::GamePoints,
            ReleasePolicy::SequentialPerMatchup,
        ),
        &double_swiss_players,
        conn,
    )
    .await?;
    populate_mid_swiss(
        &double_swiss,
        organizer,
        2,
        outcome_salt.rotate_left(3),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        double_swiss,
        "mid-tournament realtime Double Swiss with 33 players",
        "The medium odd field has two completed reciprocal rounds plus a mixed third round, native byes, schedules, Hive game play, administrative results, and sequential second-leg release.",
    ));

    let single = start_fixed(
        organizer,
        "QA Live Realtime Single Elimination",
        random_elimination_config(
            EliminationTopology::Single {
                bronze: rng.random::<bool>(),
            },
            single_elimination_clock,
            &mut rng,
        ),
        &single_elimination_players,
        conn,
    )
    .await?;
    populate_mid_elimination(
        &single,
        organizer,
        2,
        outcome_salt.rotate_left(5),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        single,
        "mid-bracket uneven realtime single elimination with 33 players",
        "Completed series release dependent contests alongside unrelated active series; custom player placement preserves fixed byes and rating seed labels.",
    ));
    let double = start_fixed(
        organizer,
        "QA Live Correspondence Double Elimination",
        random_elimination_config(
            EliminationTopology::Double,
            double_elimination_clock,
            &mut rng,
        ),
        &double_elimination_players,
        conn,
    )
    .await?;
    populate_mid_elimination(
        &double,
        organizer,
        2,
        outcome_salt.rotate_left(7),
        false,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        double,
        "mid-bracket uneven correspondence double elimination with 63 players",
        "Winners and losers branches progress independently with active play, resolved Hive games, and untouched series in a large capped field.",
    ));

    let arena = live_arena(
        organizer,
        &live_arena_players,
        outcome_salt,
        random_realtime_clock(&mut rng),
        conn,
    )
    .await?;
    fixtures.push(fixture(
        arena,
        "mid-tournament realtime Arena with 40 players and mixed results",
        "A completed opening wave precedes a mixed later wave; wins, losses, draws, Berserks, no-starts, active and unstarted games coexist.",
    ));

    let finished_arena = finished_arena(
        organizer,
        &finished_arena_players,
        outcome_salt.rotate_left(7),
        random_realtime_clock(&mut rng),
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_arena,
        "finished realtime Arena with 24 players and five waves",
        "Every seed varies field, clock, participants, pairings and results while preserving diverse scores, played counts, no-starts, draws, wins/losses, streaks and Berserks.",
    ));

    let finished_realtime_rr = finish_fixed_diverse(
        organizer,
        "QA Finished Realtime Round Robin",
        rr_config_with_clock(
            ReleasePolicy::FullyUnlocked,
            NonZeroU32::new(1).unwrap(),
            finished_realtime_rr_clock,
            &finished_realtime_rr_tiebreakers,
        ),
        &finished_realtime_rr_players,
        outcome_salt,
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_realtime_rr,
        "finished small realtime Round Robin with 7 players and varied points",
        "The compact complete schedule mixes played wins/losses/draws with adjudications and double forfeits.",
    ));

    let finished_correspondence_rr = finish_fixed_diverse(
        organizer,
        "QA Finished Correspondence Round Robin",
        rr_config_with_clock(
            ReleasePolicy::FullyUnlocked,
            NonZeroU32::new(1).unwrap(),
            finished_correspondence_rr_clock,
            &finished_correspondence_rr_tiebreakers,
        ),
        &finished_correspondence_rr_players,
        outcome_salt.rotate_left(11),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_correspondence_rr,
        "finished large correspondence Round Robin with 15 players",
        "The large odd field is one below the format cap; days-per-move clock, participant order, and outcome order vary while played draws, White wins, Black wins, and distinct totals remain guaranteed.",
    ));

    let finished_max_rr = finish_max_round_robin(organizer, &finished_max_rr_players, conn).await?;
    fixtures.push(fixture(
        finished_max_rr,
        "finished maximum Round Robin with 16 players, six repeats, and 720 completed games",
        "Every player has a complete 90-result strip, exercising the long-sheet density and wrapping behavior.",
    ));

    let finished_swiss = finish_fixed_diverse(
        organizer,
        "QA Finished Realtime Swiss",
        swiss_config_with_tiebreakers(
            finished_swiss_rounds,
            finished_swiss_clock,
            &finished_swiss_tiebreakers,
        ),
        &finished_swiss_players,
        outcome_salt.rotate_left(17),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_swiss,
        "finished medium realtime Swiss with 17 players and all configured rounds",
        "All configured engine rounds are accepted and completed; byes and varied outcomes produce credible final standings.",
    ));

    let finished_double_swiss = finish_fixed_diverse(
        organizer,
        "QA Finished Realtime Double Swiss",
        double_swiss_config_with_clock(
            finished_double_swiss_rounds,
            finished_double_swiss_clock,
            DoubleSwissPrimaryScore::MatchPoints,
            ReleasePolicy::FullyUnlocked,
        ),
        &finished_double_swiss_players,
        outcome_salt.rotate_left(23),
        true,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_double_swiss,
        "finished medium realtime Double Swiss with 21 players and all configured rounds",
        "All configured reciprocal engine rounds are accepted and completed; native byes and varied outcomes produce credible final standings.",
    ));

    let finished_single_elimination = finish_fixed_diverse(
        organizer,
        "QA Finished Realtime Single Elimination",
        random_elimination_config(
            EliminationTopology::Single { bronze: true },
            finished_single_elimination_clock,
            &mut rng,
        ),
        &finished_single_elimination_players,
        outcome_salt.rotate_left(29),
        false,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_single_elimination,
        "finished medium uneven realtime single elimination with 15 players",
        "The non-power-of-two complete bracket retains played and administrative paths while series details remain seed-reproducible.",
    ));

    let finished_double_elimination = finish_fixed_diverse(
        organizer,
        "QA Finished Correspondence Double Elimination",
        random_elimination_config(
            EliminationTopology::Double,
            finished_double_elimination_clock,
            &mut rng,
        ),
        &finished_double_elimination_players,
        outcome_salt.rotate_left(31),
        false,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        finished_double_elimination,
        "finished medium uneven correspondence double elimination with 31 players",
        "The non-power-of-two complete bracket retains played and administrative paths while series details remain seed-reproducible.",
    ));

    let review = create_tournament(
        organizer,
        TournamentDetails {
            name: String::from("QA Bracket Review and Organizers"),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(33),
            min_seats: 2,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: random_elimination_config(
                EliminationTopology::Single { bronze: true },
                Clock::Realtime(RealtimeClock {
                    base_seconds: NonZeroU32::new(300).unwrap(),
                    increment_seconds: 3,
                }),
                &mut rng,
            ),
        },
        conn,
    )
    .await?;
    let mut review_players = players[..32].to_vec();
    review_players.push(users.last().expect("dedicated deletion account exists").id);
    join(&review, &review_players, conn).await?;
    review.invite_organizer(organizer, players[0], conn).await?;
    review.accept_organizer_invitation(players[0], conn).await?;
    review.invite_organizer(organizer, players[1], conn).await?;
    review.invite_organizer(organizer, players[2], conn).await?;
    review.accept_organizer_invitation(players[2], conn).await?;
    review.leave_organizers(players[2], conn).await?;
    let review = fixed_field::prepare_elimination_start_in_transaction(
        review.id,
        organizer,
        review_players,
        conn,
    )
    .await?;
    fixtures.push(fixture(
        review,
        "33-player bracket setup with two organizers and one pending organizer invitation",
        "Only zz_qa_t_organizer controls the 15-minute setup; player_01 is a co-organizer and player_02 has a pending organizer invite. Swap preliminary and direct round-of-32 entrants; byes stay fixed. Delete zz_qa_t_delete_test during setup: its entry must survive cancellation/expiry and remain removable by an organizer after unlocking.",
    ));

    Ok((users, fixtures))
}

fn fixture(tournament: Tournament, expected: &'static str, checks: &'static str) -> Fixture {
    Fixture {
        tournament,
        expected,
        checks,
    }
}

fn random_realtime_clock(rng: &mut StdRng) -> Clock {
    let (base_seconds, increment_seconds) =
        [(60, 0), (180, 2), (300, 3), (600, 5), (900, 10)][rng.random_range(0..5)];
    Clock::Realtime(RealtimeClock {
        base_seconds: NonZeroU32::new(base_seconds).unwrap(),
        increment_seconds,
    })
}

fn random_correspondence_days_clock(rng: &mut StdRng) -> Clock {
    let days = rng.random_range(1..=4);
    Clock::Correspondence(CorrespondenceClock::DaysPerMove {
        seconds_per_move: NonZeroU32::new(days * 86_400).unwrap(),
    })
}

#[cfg(test)]
fn random_correspondence_total_clock(rng: &mut StdRng) -> Clock {
    let days = rng.random_range(5..=14);
    Clock::Correspondence(CorrespondenceClock::TotalTimeEach {
        seconds_each: NonZeroU32::new(days * 86_400).unwrap(),
    })
}

fn sample_players(players: &[Uuid], count: usize, rng: &mut StdRng) -> Vec<Uuid> {
    let mut sampled = players.to_vec();
    sampled.shuffle(rng);
    sampled.truncate(count);
    sampled
}

trait QaTiebreak: Copy {
    fn is_seed(self) -> bool;
}

impl QaTiebreak for RoundRobinCriterion {
    fn is_seed(self) -> bool {
        matches!(self, Self::TournamentPairingNumber(_))
    }
}

impl QaTiebreak for SwissCriterion {
    fn is_seed(self) -> bool {
        matches!(self, Self::TournamentPairingNumber(_))
    }
}

fn sample_creation_tiebreakers<T: QaTiebreak>(
    available: &[T],
    count: usize,
    rng: &mut StdRng,
) -> Vec<T> {
    assert!(
        (2..=available.len()).contains(&count),
        "synthetic standings require between two and every user-exposed tiebreak"
    );
    let mut sampled = available.to_vec();
    sampled.shuffle(rng);
    let mut seed_seen = false;
    sampled.retain(|criterion| !criterion.is_seed() || !std::mem::replace(&mut seed_seen, true));
    sampled.truncate(count);
    sampled
}

fn random_elimination_phase(
    clock: Clock,
    set_limit: EliminationSetLimit,
    rng: &mut StdRng,
) -> EliminationSeriesPhase {
    let games_per_set = if rng.random::<bool>() { 2 } else { 4 };
    let mut color_order = vec![EliminationEntrantSide::First; games_per_set / 2];
    color_order.extend(vec![EliminationEntrantSide::Second; games_per_set / 2]);
    color_order.shuffle(rng);
    EliminationSeriesPhase {
        games_per_set: games_per_set as u16,
        color_order,
        set_limit,
        clinch: if rng.random::<bool>() {
            EliminationClinchPolicy::PlayAll
        } else {
            EliminationClinchPolicy::EarlyClinch
        },
        clock,
    }
}

fn random_elimination_plan(clock: Clock, rng: &mut StdRng) -> EliminationSeriesPlan {
    let mut phases = Vec::new();
    if rng.random::<bool>() {
        let set_limit =
            EliminationSetLimit::AtMost(NonZeroU16::new(rng.random_range(1..=2)).unwrap());
        phases.push(random_elimination_phase(clock, set_limit, rng));
    }
    phases.push(random_elimination_phase(
        clock,
        EliminationSetLimit::UntilDecisive,
        rng,
    ));
    EliminationSeriesPlan { phases }
}

fn random_elimination_config(
    topology: EliminationTopology,
    clock: Clock,
    rng: &mut StdRng,
) -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Elimination(EliminationConfig {
            topology,
            default_plan: random_elimination_plan(clock, rng),
            stage_overrides: Vec::new(),
        }),
    }
}

fn rr_config_with_clock(
    release_policy: ReleasePolicy,
    repeats: NonZeroU32,
    clock: Clock,
    tiebreakers: &[RoundRobinCriterion],
) -> Config {
    let mut round_robin = RoundRobinConfig::standard(repeats, clock);
    round_robin.release_policy = release_policy;
    round_robin.standings = std::iter::once(RoundRobinCriterion::PrimaryScore)
        .chain(tiebreakers.iter().copied())
        .collect();
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::RoundRobin(round_robin),
    }
}

fn max_round_robin_config() -> Config {
    rr_config_with_clock(
        ReleasePolicy::FullyUnlocked,
        NonZeroU32::new(MAX_ROUND_ROBIN_REPEATS).unwrap(),
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(300).unwrap(),
            increment_seconds: 3,
        }),
        &round_robin_creation_tiebreakers(
            NonZeroU32::new(MAX_ROUND_ROBIN_REPEATS).unwrap(),
            RoundRobinPrimaryScore::GamePoints,
        )
        .into_iter()
        .filter(|criterion| {
            !matches!(
                criterion,
                RoundRobinCriterion::TournamentPairingNumber(
                    TournamentPairingNumberOrder::Descending
                )
            )
        })
        .collect::<Vec<_>>(),
    )
}

fn swiss_config_with_clock(rounds: NonZeroU32, clock: Clock) -> Config {
    let extra_rounds = rounds.get().saturating_sub(3).min(3) as i32;
    let swiss = SwissConfig::automatic_swiss(extra_rounds, clock);
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(swiss),
    }
}

fn swiss_config_with_tiebreakers(
    rounds: NonZeroU32,
    clock: Clock,
    tiebreakers: &[SwissCriterion],
) -> Config {
    let mut configuration = swiss_config_with_clock(rounds, clock);
    let FormatConfig::Swiss(swiss) = &mut configuration.format else {
        unreachable!("Swiss helper returned another format")
    };
    swiss.standings = std::iter::once(SwissCriterion::PrimaryScore)
        .chain(tiebreakers.iter().copied())
        .collect();
    configuration
}

fn double_swiss_config_with_clock(
    rounds: NonZeroU32,
    clock: Clock,
    primary_score: DoubleSwissPrimaryScore,
    release_policy: ReleasePolicy,
) -> Config {
    let extra_rounds = rounds.get().saturating_sub(3).min(3) as i32;
    let mut swiss = SwissConfig::automatic_double_swiss(extra_rounds, clock, primary_score);
    swiss.double_swiss_release_policy = release_policy;
    swiss.standings.insert(
        1,
        match primary_score {
            DoubleSwissPrimaryScore::GamePoints => SwissCriterion::MatchPoints,
            DoubleSwissPrimaryScore::MatchPoints => SwissCriterion::GamePoints,
        },
    );
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(swiss),
    }
}

async fn create_tournament(
    organizer: Uuid,
    details: TournamentDetails,
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    let new = NewTournament::new(details)?;
    let tournament = Tournament::create(organizer, &new, conn).await?;
    sql_query("insert into synthetic_tournament_qa.tournaments (id) values ($1)")
        .bind::<SqlUuid, _>(tournament.id)
        .execute(conn)
        .await?;
    Ok(tournament)
}

async fn join(tournament: &Tournament, players: &[Uuid], conn: &mut DbConn<'_>) -> Result<()> {
    for player in players {
        tournament.join(player, conn).await?;
    }
    Ok(())
}

async fn set_due(id: Uuid, conn: &mut DbConn<'_>) -> Result<()> {
    diesel::update(tournaments::table.find(id))
        .set(tournaments::starts_at.eq(Some(Utc::now() - Duration::minutes(5))))
        .execute(conn)
        .await?;
    Ok(())
}

async fn start_fixed(
    organizer: Uuid,
    name: &str,
    config: Config,
    players: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    let min_seats = if matches!(config.format(), Format::Swiss | Format::DoubleSwiss) {
        5
    } else {
        players.len() as i32
    };
    let tournament = create_tournament(
        organizer,
        TournamentDetails {
            name: name.to_owned(),
            description: Some(String::from(FIXTURE_DESCRIPTION)),
            seats: Some(players.len() as i32),
            min_seats,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: None,
            configuration: config,
        },
        conn,
    )
    .await?;
    join(&tournament, players, conn).await?;
    let mut expected_entrant_ids = TournamentUser::find_by_tournament_id(tournament.id, conn)
        .await?
        .into_iter()
        .map(|membership| membership.user_id)
        .collect::<Vec<_>>();
    expected_entrant_ids.sort_unstable();
    if matches!(
        tournament.configuration().format(),
        Format::SingleElimination | Format::DoubleElimination
    ) {
        let prepared = fixed_field::prepare_elimination_start_in_transaction(
            tournament.id,
            organizer,
            expected_entrant_ids,
            conn,
        )
        .await?;
        let setup = prepared
            .start_setup()
            .context("elimination preparation is missing")?;
        // Alternate the bracket order to cover custom placement with fixed byes.
        let mut bracket_order = setup.seeded_players.clone();
        bracket_order.swap(0, 1);
        Ok(fixed_field::confirm_elimination_start(
            tournament.id,
            organizer,
            setup.id,
            bracket_order,
            conn,
        )
        .await?
        .tournament)
    } else {
        Ok(
            fixed_field::start_by_organizer(tournament.id, organizer, expected_entrant_ids, conn)
                .await?
                .tournament,
        )
    }
}

async fn start_swiss(
    organizer: Uuid,
    name: &str,
    configuration: Config,
    players: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    let is_double_swiss = configuration.format() == Format::DoubleSwiss;
    let tournament = start_fixed(organizer, name, configuration, players, conn).await?;
    if is_double_swiss {
        let rounds = TournamentSwissRound::find_by_tournament_id(tournament.id, conn).await?;
        let accepted = rounds
            .as_slice()
            .first()
            .context("Double Swiss fixture has no accepted round")?
            .pairings();
        ensure!(
            !accepted.byes.is_empty(),
            "odd-player Double Swiss fixture has no engine-assigned bye"
        );
    }
    Ok(Tournament::find(tournament.id, conn).await?)
}

async fn start_arena_fixture(
    organizer: Uuid,
    name: &str,
    players: &[Uuid],
    game_clock: Clock,
    conn: &mut DbConn<'_>,
) -> Result<(Tournament, Vec<Game>)> {
    let Clock::Realtime(game_clock) = game_clock else {
        unreachable!("Arena fixtures require a realtime clock")
    };
    let config = Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Arena(ArenaConfig::new(NonZeroU32::new(3600).unwrap(), game_clock)),
    };
    let tournament = create_tournament(
        organizer,
        TournamentDetails {
            name: name.to_owned(),
            description: None,
            seats: None,
            min_seats: 0,
            invite_only: false,
            band_upper: None,
            band_lower: None,
            starts_at: Some(Utc::now() + Duration::days(1)),
            configuration: config,
        },
        conn,
    )
    .await?;
    join(&tournament, players, conn).await?;
    set_due(tournament.id, conn).await?;
    let present = players.iter().copied().collect::<HashSet<_>>();
    let outcome =
        arena::start_scheduled_with_presence(tournament.id, move |_| present.clone(), conn).await?;
    ensure!(
        !outcome.new_games.is_empty(),
        "present Arena entrants did not produce an opening game"
    );
    let tournament = Tournament::find(tournament.id, conn).await?;
    ensure!(
        tournament.seats.is_none()
            && tournament.min_seats == 0
            && !tournament.invite_only
            && tournament.description.is_none(),
        "Arena fixture persisted a capacity, positive minimum, invitation gate, or description"
    );
    let featured_game_id = tournament
        .featured_game_id
        .context("opening Arena pairing wave did not persist a featured game")?;
    ensure!(
        outcome.new_games.iter().any(|game| {
            game.id == featured_game_id && game.tournament_id == Some(tournament.id)
        }),
        "opening Arena feature does not belong to its tournament pairing wave"
    );
    Ok((tournament, outcome.new_games))
}

#[derive(Clone, Copy)]
enum ArenaSyntheticOutcome {
    WhiteWin,
    BlackWin,
    Draw,
    NoStart,
}

fn arena_synthetic_outcome(
    game: &Game,
    player_ranks: &HashMap<Uuid, usize>,
    salt: usize,
) -> ArenaSyntheticOutcome {
    let ordinal = (game.arena_ordinal.unwrap_or_default() as usize).wrapping_add(salt);
    if ordinal.is_multiple_of(11) {
        ArenaSyntheticOutcome::NoStart
    } else if ordinal.is_multiple_of(5) {
        ArenaSyntheticOutcome::Draw
    } else if player_ranks[&game.white_id] < player_ranks[&game.black_id] {
        ArenaSyntheticOutcome::WhiteWin
    } else {
        ArenaSyntheticOutcome::BlackWin
    }
}

async fn settle_arena_game(
    game: &Game,
    outcome: ArenaSyntheticOutcome,
    berserk: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    if matches!(outcome, ArenaSyntheticOutcome::NoStart) {
        diesel::update(games_table::table.find(game.id))
            .set(games_table::arena_move_due_at.eq(Some(Utc::now() - Duration::milliseconds(1))))
            .execute(conn)
            .await?;
        let settled = execute(game.id, Command::SettleDeadline, conn).await?;
        ensure!(
            matches!(settled, Outcome::Applied { game, newly_terminal: true, .. } if game.finished && game.turn == 0),
            "Arena no-start fixture did not settle at its opening deadline"
        );
        return Ok(());
    }

    let winner = match outcome {
        ArenaSyntheticOutcome::WhiteWin => Some(game.white_id),
        ArenaSyntheticOutcome::BlackWin => Some(game.black_id),
        ArenaSyntheticOutcome::Draw | ArenaSyntheticOutcome::NoStart => None,
    };
    if berserk {
        if let Some(user_id) = winner {
            execute(game.id, Command::Berserk { user_id }, conn).await?;
        }
    }
    add_opening_moves(game, conn).await?;
    match outcome {
        ArenaSyntheticOutcome::WhiteWin => {
            execute(
                game.id,
                Command::Control {
                    user_id: game.black_id,
                    control: GameControl::Resign(Color::Black),
                },
                conn,
            )
            .await?;
        }
        ArenaSyntheticOutcome::BlackWin => {
            execute(
                game.id,
                Command::Control {
                    user_id: game.white_id,
                    control: GameControl::Resign(Color::White),
                },
                conn,
            )
            .await?;
        }
        ArenaSyntheticOutcome::Draw => {
            agree_draw(game, conn).await?;
        }
        ArenaSyntheticOutcome::NoStart => unreachable!(),
    }
    Ok(())
}

async fn settle_arena_round(
    games: &[Game],
    player_ranks: &HashMap<Uuid, usize>,
    salt: usize,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    for game in games {
        let ordinal = game.arena_ordinal.unwrap_or_default() as usize;
        let outcome = arena_synthetic_outcome(game, player_ranks, salt);
        settle_arena_game(
            game,
            outcome,
            ordinal.wrapping_add(salt).is_multiple_of(3),
            conn,
        )
        .await?;
    }
    Ok(())
}

async fn pair_present_arena(
    tournament_id: Uuid,
    players: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>> {
    let present = players.iter().copied().collect::<HashSet<_>>();
    let mut games =
        arena::pair_waiting_with_presence(tournament_id, move |_| present.clone(), conn)
            .await?
            .new_games;
    games.sort_by_key(|game| game.arena_ordinal);
    ensure!(
        !games.is_empty(),
        "Arena fixture produced no next-round games"
    );
    Ok(games)
}

async fn live_arena(
    organizer: Uuid,
    players: &[Uuid],
    outcome_salt: usize,
    game_clock: Clock,
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    let (&late_player, opening_players) = players
        .split_last()
        .context("live Arena fixture has no player to reserve for live Join")?;
    let (tournament, mut games) = start_arena_fixture(
        organizer,
        "QA Live Realtime Arena",
        opening_players,
        game_clock,
        conn,
    )
    .await?;
    games.sort_by_key(|game| game.arena_ordinal);
    let ranks = players
        .iter()
        .copied()
        .enumerate()
        .map(|(index, user_id)| (user_id, index))
        .collect::<HashMap<_, _>>();
    let first_feature_id = Tournament::find(tournament.id, conn)
        .await?
        .featured_game_id
        .context("live Arena lost its opening persisted feature")?;
    let first_feature = games
        .iter()
        .find(|game| game.id == first_feature_id)
        .context("live Arena opening feature is not one of its games")?;
    let first_feature_ordinal = first_feature.arena_ordinal.unwrap_or_default() as usize;
    settle_arena_game(
        first_feature,
        arena_synthetic_outcome(first_feature, &ranks, outcome_salt),
        first_feature_ordinal
            .wrapping_add(outcome_salt)
            .is_multiple_of(3),
        conn,
    )
    .await?;
    ensure!(
        Tournament::find(tournament.id, conn)
            .await?
            .featured_game_id
            == Some(first_feature_id),
        "finishing the live Arena feature changed it before another pairing wave"
    );
    for game in games.iter().filter(|game| game.id != first_feature_id) {
        let ordinal = game.arena_ordinal.unwrap_or_default() as usize;
        settle_arena_game(
            game,
            arena_synthetic_outcome(game, &ranks, outcome_salt),
            ordinal.wrapping_add(outcome_salt).is_multiple_of(3),
            conn,
        )
        .await?;
    }
    ensure!(
        Tournament::find(tournament.id, conn)
            .await?
            .featured_game_id
            == Some(first_feature_id),
        "live Arena feature changed without a non-empty pairing wave"
    );

    let present = players.iter().copied().collect::<HashSet<_>>();
    let join_presence = present.clone();
    let joined = match arena::join_with_presence(
        tournament.id,
        late_player,
        move |_| join_presence.clone(),
        conn,
    )
    .await
    {
        Ok(joined) => joined,
        Err(DbError::TournamentFull) => {
            return Err(anyhow::anyhow!(
                "capacityless live Arena incorrectly rejected its reserved entrant as full"
            ));
        }
        Err(error) => return Err(error.into()),
    };
    ensure!(
        joined.joined_now,
        "reserved Arena player did not join after start"
    );
    let pairing =
        arena::pair_waiting_with_presence(tournament.id, move |_| present.clone(), conn).await?;
    ensure!(
        !pairing.new_games.is_empty(),
        "live Arena Join did not produce a later pairing wave"
    );
    ensure!(
        Tournament::find(tournament.id, conn)
            .await?
            .number_of_players(conn)
            .await?
            == players.len() as i64,
        "live Arena did not retain its full deterministic cohort after live Join"
    );
    let replacement_feature_id = Tournament::find(tournament.id, conn)
        .await?
        .featured_game_id
        .context("later live Arena pairing wave cleared the feature")?;
    ensure!(
        replacement_feature_id != first_feature_id
            && pairing.new_games.iter().any(|game| {
                game.id == replacement_feature_id && game.tournament_id == Some(tournament.id)
            }),
        "later non-empty Arena pairing wave did not replace the finished feature"
    );
    let replacement_feature = pairing
        .new_games
        .iter()
        .find(|game| game.id == replacement_feature_id)
        .context("replacement Arena feature is not in its pairing wave")?;
    add_opening_moves(replacement_feature, conn).await?;

    for game in pairing
        .new_games
        .iter()
        .filter(|game| game.id != replacement_feature_id)
        .take(6)
    {
        let ordinal = game.arena_ordinal.unwrap_or_default() as usize;
        settle_arena_game(
            game,
            arena_synthetic_outcome(game, &ranks, outcome_salt.rotate_left(5)),
            ordinal.is_multiple_of(3),
            conn,
        )
        .await?;
    }
    for game in pairing
        .new_games
        .iter()
        .filter(|game| game.id != replacement_feature_id && !game.finished)
        .skip(6)
        .take(1)
    {
        add_opening_moves(game, conn).await?;
    }

    let all_games = tournament.games(conn).await?;
    ensure!(
        all_games.iter().any(|game| game.finished)
            && all_games
                .iter()
                .any(|game| !game.finished && game.turn >= 2)
            && all_games
                .iter()
                .any(|game| !game.finished && game.turn == 0)
            && all_games.iter().any(|game| {
                game.id == replacement_feature_id && !game.finished && game.turn >= 2
            }),
        "live Arena fixture lacks its finished, active, unstarted, or deliberately preserved featured game"
    );
    let tournament = Tournament::find(tournament.id, conn).await?;
    ensure!(
        tournament.featured_game_id == Some(replacement_feature_id),
        "live Arena feature changed while other games were populated"
    );
    Ok(tournament)
}

async fn finished_arena(
    organizer: Uuid,
    players: &[Uuid],
    outcome_salt: usize,
    game_clock: Clock,
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    ensure!(
        players.len() >= 20,
        "finished Arena fixture requires at least 20 players"
    );
    let (tournament, mut games) = start_arena_fixture(
        organizer,
        "QA Finished Realtime Arena",
        players,
        game_clock,
        conn,
    )
    .await?;
    games.sort_by_key(|game| game.arena_ordinal);
    let ranks = players
        .iter()
        .copied()
        .enumerate()
        .map(|(index, user_id)| (user_id, index))
        .collect::<HashMap<_, _>>();

    for round in 0..5 {
        let round_salt = outcome_salt.wrapping_add(round);
        settle_arena_round(&games, &ranks, round_salt, conn).await?;
        if round < 4 {
            let next_round_players = if round == 1 {
                let stable_absent = games
                    .iter()
                    .find(|game| {
                        !matches!(
                            arena_synthetic_outcome(game, &ranks, round_salt),
                            ArenaSyntheticOutcome::NoStart
                        )
                    })
                    .map(|game| [game.white_id, game.black_id])
                    .context("finished Arena second round has no played game to hold absent")?;
                players
                    .iter()
                    .copied()
                    .filter(|user_id| !stable_absent.contains(user_id))
                    .collect::<Vec<_>>()
            } else {
                players.to_vec()
            };
            games = pair_present_arena(tournament.id, &next_round_players, conn).await?;
        }
    }

    let scoring_games = tournament
        .games(conn)
        .await?
        .into_iter()
        .filter(|game| game.finished)
        .collect::<Vec<_>>();
    let latest_finished_at = scoring_games
        .iter()
        .filter_map(|game| game.finished_at)
        .max()
        .context("finished Arena fixture has no terminal timestamp")?;
    let cutoff = std::cmp::max(Utc::now(), latest_finished_at + Duration::microseconds(1));
    diesel::update(tournaments::table.find(tournament.id))
        .set(tournaments::starts_at.eq(Some(cutoff - Duration::seconds(3_600))))
        .execute(conn)
        .await?;
    let historical_feature_id = Tournament::find(tournament.id, conn)
        .await?
        .featured_game_id
        .context("finished Arena fixture lost its historical feature before cutoff")?;
    arena::finalize_due_in_transaction(tournament.id, conn).await?;

    let tournament = Tournament::find(tournament.id, conn).await?;
    ensure!(
        tournament.status() == TournamentStatus::Finished,
        "finished Arena fixture remained in progress"
    );
    ensure!(
        tournament.featured_game_id == Some(historical_feature_id),
        "finishing the Arena unexpectedly erased or replaced its historical feature"
    );
    let games = tournament.games(conn).await?;
    let conclusions = games
        .iter()
        .filter(|game| game.finished)
        .map(|game| game.conclusion.clone())
        .collect::<HashSet<_>>();
    ensure!(
        games.len() >= players.len()
            && conclusions.len() >= 3
            && games.iter().any(|game| {
                game.finished
                    && game.turn == 0
                    && game.conclusion == Conclusion::Timeout.to_string()
            })
            && games
                .iter()
                .any(|game| game.white_berserked || game.black_berserked),
        "finished Arena fixture lacks its required result diversity"
    );
    let final_outcome = TournamentFinalOutcome::load(tournament.id, conn)
        .await?
        .context("finished Arena fixture has no persisted final outcome")?;
    let rows = final_outcome
        .standings
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .collect::<Vec<_>>();
    let scores = rows
        .iter()
        .map(|row| format!("{:?}", row.primary_score))
        .collect::<HashSet<_>>();
    let played_counts = rows
        .iter()
        .map(|row| row.games_played)
        .collect::<HashSet<_>>();
    ensure!(
        rows.len() == players.len() && scores.len() >= 4 && played_counts.len() >= 2,
        "finished Arena standings lack player, score, or games-played diversity: {} players, {} rows, {} distinct scores, {} distinct played counts",
        players.len(),
        rows.len(),
        scores.len(),
        played_counts.len(),
    );
    Ok(tournament)
}

fn fixture_slot_order(key: SlotKey) -> Result<(u8, u64, u64, u8)> {
    match key {
        SlotKey::RoundRobin { slot } => Ok((
            0,
            u64::try_from(slot.value()).context("Round Robin Slot identity exceeds u64")?,
            0,
            0,
        )),
        SlotKey::Swiss { slot } => {
            let leg = match slot.leg {
                SwissLeg::Single => 0,
                SwissLeg::First => 1,
                SwissLeg::Second => 2,
            };
            Ok((
                1,
                u64::from(slot.round_index),
                u64::from(slot.pairing_index),
                leg,
            ))
        }
        SlotKey::Elimination { node, slot } => Ok((
            2,
            u64::try_from(node.value()).context("Elimination node identity exceeds u64")?,
            u64::from(slot.value()),
            0,
        )),
    }
}

async fn fixed_games_with_keys(
    tournament: &Tournament,
    conn: &mut DbConn<'_>,
) -> Result<Vec<(SlotKey, Game)>> {
    let mut key_by_slot = HashMap::new();
    for slot in TournamentSlot::find_by_tournament_id(tournament.id, conn).await? {
        let slot_id = slot.id;
        key_by_slot.insert(slot_id, slot.key);
    }

    let mut games = Vec::new();
    for game in tournament.games(conn).await? {
        let slot_id = game
            .tournament_slot_id
            .context("fixed-field game has no tournament slot")?;
        let key = key_by_slot.get(&slot_id).copied().with_context(|| {
            format!(
                "fixed-field game {} references unknown tournament slot {slot_id}",
                game.id
            )
        })?;
        games.push((fixture_slot_order(key)?, key, game));
    }
    games.sort_by_key(|(order, _, _)| *order);
    Ok(games
        .into_iter()
        .map(|(_, key, game)| (key, game))
        .collect())
}

async fn fixed_games_in_native_order(
    tournament: &Tournament,
    conn: &mut DbConn<'_>,
) -> Result<Vec<Game>> {
    Ok(fixed_games_with_keys(tournament, conn)
        .await?
        .into_iter()
        .map(|(_, game)| game)
        .collect())
}

async fn populate_round_robin_states(
    tournament: &Tournament,
    organizer: Uuid,
    players: &[Uuid],
    with_schedule: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let rows = fixed_games_in_native_order(tournament, conn).await?;
    ensure!(
        rows.len() >= 3,
        "Round Robin fixture unexpectedly produced too few games"
    );
    for (index, game) in rows.iter().skip(3).take(8).enumerate() {
        settle_fixed_game(tournament.id, organizer, game, index, true, conn).await?;
    }
    let scheduled = &rows[0];
    let slot_id = scheduled
        .tournament_slot_id
        .context("released Round Robin game has no slot")?;
    if with_schedule {
        add_accepted_and_pending_schedule_offers(tournament, scheduled, conn).await?;
    }

    add_opening_moves(&rows[1], conn).await?;
    if with_schedule {
        let adjudication_slot_id = rows[2]
            .tournament_slot_id
            .context("adjudication Round Robin game has no slot")?;
        let expected_resolution = TournamentSlot::find(tournament.id, adjudication_slot_id, conn)
            .await?
            .resolution;
        fixed_field::adjudicate_slot_atomic(
            tournament.id,
            adjudication_slot_id,
            TournamentGameResult::Winner(Color::White),
            expected_resolution,
            organizer,
            conn,
        )
        .await
        .with_context(|| {
            format!(
                "could not adjudicate Round Robin frontier for {:?}",
                tournament.name
            )
        })?;
    } else {
        add_opening_moves(&rows[2], conn).await?;
        execute(
            rows[2].id,
            Command::Control {
                user_id: rows[2].black_id,
                control: GameControl::Resign(Color::Black),
            },
            conn,
        )
        .await?;
    }
    let withdrawal = players
        .iter()
        .copied()
        .find(|player| {
            *player != scheduled.white_id && *player != scheduled.black_id
                && *player != rows[1].white_id && *player != rows[1].black_id
                && *player != rows[2].white_id && *player != rows[2].black_id
        })
        .context(
            "Round Robin fixture needs a withdrawal candidate outside scheduled, active, and adjudicated games",
        )?;
    fixed_field::withdraw_player(tournament.id, withdrawal, organizer, conn).await?;
    let offers = ScheduleOffer::find_for_tournament(tournament.id, conn).await?;
    if with_schedule {
        let accepted = offers
            .iter()
            .find(|offer| {
                offer
                    .status()
                    .is_ok_and(|status| status == ScheduleOfferStatus::Accepted)
            })
            .context("realtime Round Robin fixture lost its accepted schedule offer")?;
        let pending = offers
            .iter()
            .find(|offer| {
                offer
                    .status()
                    .is_ok_and(|status| status == ScheduleOfferStatus::Pending)
            })
            .context("realtime Round Robin fixture lost its pending schedule offer")?;
        let persisted_slot = TournamentSlot::find(tournament.id, slot_id, conn).await?;
        ensure!(
            offers.len() == 2
                && accepted.selected_time == persisted_slot.scheduled_at
                && pending.candidates()?.len() == 2,
            "realtime Round Robin fixture lost its appointment and pending reschedule offer"
        );
    } else {
        ensure!(
            offers.is_empty(),
            "correspondence Round Robin fixture unexpectedly retained schedule offers"
        );
    }
    let scheduled_game = Game::find_by_uuid(&scheduled.id, conn).await?;
    let expected_unstarted_status = if with_schedule {
        GameStatus::NotStarted
    } else {
        GameStatus::InProgress
    };
    ensure!(
        !scheduled_game.finished
            && scheduled_game.turn == 0
            && scheduled_game.game_status == expected_unstarted_status.to_string(),
        "Round Robin fixture did not retain its created but unplayed game"
    );
    let active = Game::find_by_uuid(&rows[1].id, conn).await?;
    ensure!(
        active.game_status == GameStatus::InProgress.to_string() && active.turn == 2,
        "Round Robin fixture did not retain its legal two-move in-progress game"
    );
    let adjudicated = Game::find_by_uuid(&rows[2].id, conn).await?;
    if with_schedule {
        ensure!(
            adjudicated.finished
                && adjudicated.tournament_game_result
                    == TournamentGameResult::Winner(Color::White).to_string(),
            "realtime Round Robin fixture did not retain its administrative winner adjudication"
        );
    } else {
        ensure!(
            adjudicated.finished && adjudicated.turn == 2,
            "correspondence Round Robin fixture did not retain its finished Hive game"
        );
    }
    ensure!(
        tournament
            .withdrawn_players(conn)
            .await?
            .contains(&withdrawal),
        "Round Robin fixture did not retain its withdrawal"
    );
    Ok(())
}

async fn add_opening_moves(game: &Game, conn: &mut DbConn<'_>) -> Result<()> {
    if game.game_start == GameStart::Ready.to_string() {
        execute(
            game.id,
            Command::StartReady {
                user_id: game.white_id,
            },
            conn,
        )
        .await?;
    }
    for (actor, piece, position) in [
        (game.white_id, "wA1", Position::initial_spawn_position()),
        (
            game.black_id,
            "bA1",
            Position::initial_spawn_position().to(Direction::E),
        ),
    ] {
        execute(
            game.id,
            Command::Move {
                user_id: actor,
                turn: Turn::Move(piece.parse()?, position),
                compensation: 0.0,
            },
            conn,
        )
        .await?;
    }
    Ok(())
}

async fn settle_released_frontier(
    tournament: &Tournament,
    organizer: Uuid,
    outcome_salt: usize,
    allow_adjudication: bool,
    conn: &mut DbConn<'_>,
) -> Result<usize> {
    let frontier = fixed_games_with_keys(tournament, conn)
        .await?
        .into_iter()
        .filter(|(_, game)| !game.finished)
        .map(|(key, _)| fixture_slot_order(key).map(|(format, unit, _, _)| (format, unit)))
        .collect::<Result<HashSet<_>>>()?;
    let mut settled = 0usize;
    loop {
        let game = fixed_games_with_keys(tournament, conn)
            .await?
            .into_iter()
            .find(|(key, game)| {
                !game.finished
                    && fixture_slot_order(*key)
                        .is_ok_and(|(format, unit, _, _)| frontier.contains(&(format, unit)))
            });
        let Some((_, game)) = game else {
            break;
        };
        settle_fixed_game(
            tournament.id,
            organizer,
            &game,
            outcome_salt.wrapping_add(settled),
            allow_adjudication,
            conn,
        )
        .await?;
        settled += 1;
        ensure!(
            settled <= 512,
            "synthetic frontier did not settle within its safety bound"
        );
    }
    ensure!(
        settled > 0,
        "synthetic frontier contained no released games"
    );
    Ok(settled)
}

async fn add_accepted_and_pending_schedule_offers(
    tournament: &Tournament,
    game: &Game,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let slot_id = game
        .tournament_slot_id
        .context("scheduled fixed-field game has no slot")?;
    let now = Utc::now();
    let proposed_time = now + Duration::hours(2);
    let accepted_offer = ScheduleOffer::propose(
        game.white_id,
        tournament.id,
        slot_id,
        vec![
            proposed_time,
            now + Duration::minutes(150),
            now + Duration::hours(3),
        ],
        conn,
    )
    .await?
    .pop()
    .context("schedule offer creation returned no inserted offer")?;
    let selected_time = accepted_offer
        .candidates()?
        .into_iter()
        .next()
        .context("accepted schedule offer lost its first candidate")?;
    ScheduleOffer::accept(accepted_offer.id, game.black_id, selected_time, conn).await?;
    ScheduleOffer::propose(
        game.black_id,
        tournament.id,
        slot_id,
        vec![now + Duration::hours(4), now + Duration::hours(5)],
        conn,
    )
    .await?;
    Ok(())
}

async fn populate_active_frontier(
    tournament: &Tournament,
    organizer: Uuid,
    with_schedule: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let games = fixed_games_in_native_order(tournament, conn)
        .await?
        .into_iter()
        .filter(|game| !game.finished)
        .collect::<Vec<_>>();
    let mut matchup_keys = HashSet::new();
    let games = games
        .into_iter()
        .filter(|game| {
            let key = if game.white_id < game.black_id {
                (game.white_id, game.black_id)
            } else {
                (game.black_id, game.white_id)
            };
            matchup_keys.insert(key)
        })
        .take(4)
        .collect::<Vec<_>>();
    ensure!(
        games.len() >= 4,
        "mid-tournament frontier needs four distinct matchups for scheduled, active, administrative, and untouched states"
    );
    if with_schedule {
        add_accepted_and_pending_schedule_offers(tournament, &games[0], conn).await?;
    }
    add_opening_moves(&games[1], conn).await?;
    if with_schedule {
        let adjudication_slot_id = games[2]
            .tournament_slot_id
            .context("frontier adjudication game has no slot")?;
        let expected_resolution = TournamentSlot::find(tournament.id, adjudication_slot_id, conn)
            .await?
            .resolution;
        fixed_field::adjudicate_slot_atomic(
            tournament.id,
            adjudication_slot_id,
            TournamentGameResult::Winner(Color::White),
            expected_resolution,
            organizer,
            conn,
        )
        .await
        .with_context(|| {
            format!(
                "could not adjudicate active frontier for {:?}",
                tournament.name
            )
        })?;
    } else {
        add_opening_moves(&games[2], conn).await?;
        execute(
            games[2].id,
            Command::Control {
                user_id: games[2].black_id,
                control: GameControl::Resign(Color::Black),
            },
            conn,
        )
        .await?;
    }

    fixed_field::reconcile(tournament.id, conn).await?;
    let refreshed = tournament.games(conn).await?;
    ensure!(
        refreshed.iter().any(|game| game.finished)
            && refreshed
                .iter()
                .any(|game| !game.finished && game.turn >= 2)
            && refreshed
                .iter()
                .any(|game| !game.finished && game.turn == 0),
        "mid-tournament frontier lost its completed, active, or planned state"
    );
    Ok(())
}

async fn populate_mid_swiss(
    tournament: &Tournament,
    organizer: Uuid,
    completed_rounds: usize,
    outcome_salt: usize,
    with_schedule: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    for round_id in 0..completed_rounds {
        settle_released_frontier(
            tournament,
            organizer,
            outcome_salt.wrapping_add(round_id * 131),
            false,
            conn,
        )
        .await?;
        let accepted_rounds =
            TournamentSwissRound::find_by_tournament_id(tournament.id, conn).await?;
        ensure!(
            accepted_rounds.len() >= round_id + 2,
            "mid-Swiss fixture failed to advance automatically to round {}",
            round_id + 2
        );
    }
    let rounds = TournamentSwissRound::find_by_tournament_id(tournament.id, conn).await?;
    ensure!(
        rounds.len() == completed_rounds + 1,
        "mid-Swiss fixture has {} accepted rounds instead of {}",
        rounds.len(),
        completed_rounds + 1
    );
    assert_swiss_round_depth(tournament.id, completed_rounds, false, conn).await?;
    populate_active_frontier(tournament, organizer, with_schedule, conn).await
}

async fn assert_swiss_round_depth(
    tournament_id: Uuid,
    required_resolved_rounds: usize,
    require_all_accepted_rounds_resolved: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let rounds = TournamentSwissRound::find_by_tournament_id(tournament_id, conn).await?;
    ensure!(
        rounds.len() >= required_resolved_rounds,
        "Swiss fixture has {} accepted rounds but requires at least {required_resolved_rounds}",
        rounds.len()
    );
    let slots = TournamentSlot::find_by_tournament_id(tournament_id, conn).await?;
    if require_all_accepted_rounds_resolved {
        ensure!(
            !slots.is_empty() && slots.iter().all(|slot| slot.resolution.is_some()),
            "finished Swiss fixture retains an unresolved slot"
        );
    } else {
        ensure!(
            slots.iter().any(|slot| slot.resolution.is_some()),
            "mid-Swiss fixture has accepted history but no resolved slots"
        );
    }
    Ok(())
}

async fn populate_mid_elimination(
    tournament: &Tournament,
    organizer: Uuid,
    completed_frontiers: usize,
    outcome_salt: usize,
    with_schedule: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    for completed_frontier in 0..completed_frontiers {
        settle_released_frontier(
            tournament,
            organizer,
            outcome_salt.wrapping_add((completed_frontier + 1) * 137),
            true,
            conn,
        )
        .await?;
        ensure!(
            tournament
                .games(conn)
                .await?
                .iter()
                .any(|game| !game.finished),
            "mid-elimination fixture failed to release successors after frontier {}",
            completed_frontier + 1,
        );
    }
    populate_active_frontier(tournament, organizer, with_schedule, conn).await
}

async fn settle_fixed_game(
    tournament_id: Uuid,
    organizer: Uuid,
    game: &Game,
    outcome_index: usize,
    allow_adjudication: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let outcome = if allow_adjudication && game.game_start == GameStart::Ready.to_string() {
        outcome_index % 5
    } else {
        outcome_index % 3
    };
    match outcome {
        0 => {
            add_opening_moves(game, conn).await?;
            agree_draw(game, conn).await?;
        }
        1 | 2 => {
            add_opening_moves(game, conn).await?;
            let (user_id, color) = if outcome == 1 {
                (game.black_id, Color::Black)
            } else {
                (game.white_id, Color::White)
            };
            execute(
                game.id,
                Command::Control {
                    user_id,
                    control: GameControl::Resign(color),
                },
                conn,
            )
            .await?;
        }
        3 => {
            let slot_id = game
                .tournament_slot_id
                .context("adjudication game has no slot")?;
            let expected_resolution = TournamentSlot::find(tournament_id, slot_id, conn)
                .await?
                .resolution;
            fixed_field::adjudicate_slot_atomic(
                tournament_id,
                slot_id,
                TournamentGameResult::Winner(Color::White),
                expected_resolution,
                organizer,
                conn,
            )
            .await
            .with_context(|| {
                format!(
                    "could not adjudicate game {} in tournament {tournament_id}",
                    game.id
                )
            })?;
        }
        4 => {
            let slot_id = game
                .tournament_slot_id
                .context("double-forfeit game has no slot")?;
            let expected_resolution = TournamentSlot::find(tournament_id, slot_id, conn)
                .await?
                .resolution;
            fixed_field::adjudicate_slot_atomic(
                tournament_id,
                slot_id,
                TournamentGameResult::DoubleForfeit,
                expected_resolution,
                organizer,
                conn,
            )
            .await
            .with_context(|| {
                format!(
                    "could not double-forfeit game {} in tournament {tournament_id}",
                    game.id
                )
            })?;
        }
        _ => unreachable!(),
    }
    // The script has no WebSocket handler to reconcile a newly terminal Hive game.
    fixed_field::reconcile(tournament_id, conn).await?;
    Ok(())
}

async fn settle_finished_elimination_game(
    game: &Game,
    key: SlotKey,
    winners: &mut HashMap<usize, Uuid>,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let SlotKey::Elimination {
        node,
        slot: series_game,
    } = key
    else {
        anyhow::bail!("released elimination game has a non-elimination native key")
    };

    if series_game.value() == 0 {
        add_opening_moves(game, conn).await?;
        agree_draw(game, conn).await?;
        return Ok(());
    }

    let node_id = node.value();
    let winner_id = if let Some(winner_id) = winners.get(&node_id) {
        *winner_id
    } else {
        let winner_id = if winners.len().is_multiple_of(2) {
            game.white_id
        } else {
            game.black_id
        };
        winners.insert(node_id, winner_id);
        winner_id
    };
    let (loser_id, loser_color) = if winner_id == game.white_id {
        (game.black_id, Color::Black)
    } else {
        ensure!(
            winner_id == game.black_id,
            "elimination series winner is absent from a released game"
        );
        (game.white_id, Color::White)
    };
    add_opening_moves(game, conn).await?;
    execute(
        game.id,
        Command::Control {
            user_id: loser_id,
            control: GameControl::Resign(loser_color),
        },
        conn,
    )
    .await?;
    Ok(())
}

async fn finish_max_round_robin(
    organizer: Uuid,
    players: &[Uuid],
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    ensure!(
        players.len() == MAX_ROUND_ROBIN_ENTRANTS,
        "maximum Round Robin fixture requires {MAX_ROUND_ROBIN_ENTRANTS} players"
    );
    let tournament = start_fixed(
        organizer,
        MAX_ROUND_ROBIN_FIXTURE_NAME,
        max_round_robin_config(),
        players,
        conn,
    )
    .await?;
    let games = fixed_games_in_native_order(&tournament, conn).await?;
    ensure!(
        games.len() == MAX_ROUND_ROBIN_GAMES,
        "maximum Round Robin generated {} games instead of {MAX_ROUND_ROBIN_GAMES}",
        games.len()
    );

    for (index, game) in games.iter().enumerate() {
        if index < 3 {
            settle_fixed_game(
                tournament.id,
                organizer,
                game,
                MAX_ROUND_ROBIN_OUTCOME_SALT.wrapping_add(index),
                false,
                conn,
            )
            .await?;
            continue;
        }
        let result = match index % 4 {
            0 => TournamentGameResult::Winner(Color::White),
            1 => TournamentGameResult::Winner(Color::Black),
            2 => TournamentGameResult::Draw,
            3 => TournamentGameResult::DoubleForfeit,
            _ => unreachable!(),
        };
        let slot_id = game
            .tournament_slot_id
            .context("maximum Round Robin game has no slot")?;
        let expected_resolution = TournamentSlot::find(tournament.id, slot_id, conn)
            .await?
            .resolution;
        fixed_field::adjudicate_slot_atomic(
            tournament.id,
            slot_id,
            result,
            expected_resolution,
            organizer,
            conn,
        )
        .await
        .with_context(|| {
            format!(
                "could not settle maximum Round Robin game {} of {MAX_ROUND_ROBIN_GAMES}",
                index + 1
            )
        })?;
    }

    let tournament = Tournament::find(tournament.id, conn).await?;
    assert_max_round_robin_fixture(&tournament, conn).await?;
    Ok(tournament)
}

async fn assert_max_round_robin_fixture(
    tournament: &Tournament,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    ensure!(
        tournament.name == MAX_ROUND_ROBIN_FIXTURE_NAME,
        "unexpected maximum Round Robin fixture name {:?}",
        tournament.name
    );
    ensure!(
        tournament.status() == TournamentStatus::Finished,
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} is not finished"
    );
    ensure!(
        tournament.number_of_players(conn).await? == MAX_ROUND_ROBIN_ENTRANTS as i64,
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} does not have {MAX_ROUND_ROBIN_ENTRANTS} players"
    );
    let configuration = tournament.configuration();
    let FormatConfig::RoundRobin(round_robin) = &configuration.format else {
        anyhow::bail!("{MAX_ROUND_ROBIN_FIXTURE_NAME} is not a Round Robin")
    };
    ensure!(
        round_robin.repeats.get() == MAX_ROUND_ROBIN_REPEATS,
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} does not use {MAX_ROUND_ROBIN_REPEATS} repeats"
    );
    let slots = TournamentSlot::find_by_tournament_id(tournament.id, conn).await?;
    ensure!(
        slots.len() == MAX_ROUND_ROBIN_GAMES,
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} has {} slots instead of {MAX_ROUND_ROBIN_GAMES}",
        slots.len()
    );
    let games = tournament.games(conn).await?;
    ensure!(
        games.len() == MAX_ROUND_ROBIN_GAMES && games.iter().all(|game| game.finished),
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} does not have {MAX_ROUND_ROBIN_GAMES} completed games"
    );
    let final_outcome = TournamentFinalOutcome::load(tournament.id, conn)
        .await?
        .context("maximum Round Robin fixture has no final outcome")?;
    let rows = final_outcome
        .standings
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .collect::<Vec<_>>();
    ensure!(
        rows.len() == MAX_ROUND_ROBIN_ENTRANTS
            && rows
                .iter()
                .all(|row| {
                    row.wins + row.draws + row.losses
                        == MAX_ROUND_ROBIN_RESULTS_PER_PLAYER as u32
                }),
        "{MAX_ROUND_ROBIN_FIXTURE_NAME} does not give every player a {MAX_ROUND_ROBIN_RESULTS_PER_PLAYER}-result sheet"
    );
    Ok(())
}

async fn finish_fixed_diverse(
    organizer: Uuid,
    name: &str,
    config: Config,
    players: &[Uuid],
    outcome_salt: usize,
    expect_point_diversity: bool,
    conn: &mut DbConn<'_>,
) -> Result<Tournament> {
    let format = config.format();
    let swiss_format = matches!(format, Format::Swiss | Format::DoubleSwiss);
    let elimination_format = matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    );
    let tournament = start_fixed(organizer, name, config, players, conn).await?;
    let mut settled = 0usize;
    let mut elimination_winners = HashMap::new();
    while settled < 512 {
        let current = Tournament::find(tournament.id, conn).await?;
        if current.status() == TournamentStatus::Finished {
            break;
        }
        let games = fixed_games_with_keys(&current, conn).await?;
        let (key, game) = games
            .into_iter()
            .find(|(_, game)| !game.finished)
            .context("in-progress fixed-field fixture has no unfinished game")?;
        if elimination_format {
            settle_finished_elimination_game(&game, key, &mut elimination_winners, conn).await?;
            fixed_field::reconcile(tournament.id, conn).await?;
        } else {
            settle_fixed_game(
                tournament.id,
                organizer,
                &game,
                outcome_salt.wrapping_add(settled),
                !swiss_format,
                conn,
            )
            .await?;
        }
        settled += 1;
    }

    let tournament = Tournament::find(tournament.id, conn).await?;
    ensure!(
        tournament.status() == TournamentStatus::Finished,
        "{name} did not finish within its {settled}-game safety bound"
    );
    if matches!(
        tournament.configuration().format(),
        Format::Swiss | Format::DoubleSwiss
    ) {
        let configuration = tournament.configuration();
        let FormatConfig::Swiss(swiss) = &configuration.format else {
            unreachable!("Swiss format contains another configuration")
        };
        let configured_rounds = swiss
            .rounds
            .resolved_rounds()
            .context("finished Swiss fixture retained an unresolved round count")?
            .get() as usize;
        let accepted_rounds =
            TournamentSwissRound::find_by_tournament_id(tournament.id, conn).await?;
        ensure!(
            accepted_rounds.len() == configured_rounds,
            "{name} shallow-finished after {} accepted rounds instead of all {configured_rounds} configured rounds",
            accepted_rounds.len()
        );
        assert_swiss_round_depth(tournament.id, configured_rounds, true, conn).await?;
    }
    let final_outcome = TournamentFinalOutcome::load(tournament.id, conn)
        .await?
        .context("diverse fixed-field fixture has no persisted final outcome")?;
    let games = tournament.games(conn).await?;
    let result_kinds = games
        .iter()
        .filter(|game| game.finished)
        .map(|game| (game.conclusion.clone(), game.tournament_game_result.clone()))
        .collect::<HashSet<_>>();
    ensure!(
        games.iter().any(|game| game.finished && game.turn >= 2) && result_kinds.len() >= 3,
        "{name} lacks played-game and scored-result diversity ({} result kinds)",
        result_kinds.len()
    );
    if expect_point_diversity {
        let rows = final_outcome
            .standings
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .collect::<Vec<_>>();
        let scores = rows
            .iter()
            .map(|row| format!("{:?}", row.primary_score))
            .collect::<HashSet<_>>();
        ensure!(scores.len() >= 2, "{name} standings lack score diversity");
    }
    Ok(tournament)
}

async fn agree_draw(game: &Game, conn: &mut DbConn<'_>) -> Result<()> {
    for (user_id, control) in [
        (game.white_id, GameControl::DrawOffer(Color::White)),
        (game.black_id, GameControl::DrawAccept(Color::Black)),
    ] {
        execute(game.id, Command::Control { user_id, control }, conn).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use db_lib::{
        get_conn,
        schema::{ratings, tournaments_organizers, users},
    };

    mod test_database {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../db/tests/common/test_db_support.rs"
        ));
    }

    #[allow(dead_code)]
    mod fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../db/tests/common/tournament.rs"
        ));
    }

    async fn register_account(id: Uuid, conn: &mut DbConn<'_>) {
        sql_query("insert into synthetic_tournament_qa.accounts (id) values ($1)")
            .bind::<SqlUuid, _>(id)
            .execute(conn)
            .await
            .unwrap();
    }

    async fn register_tournament(id: Uuid, conn: &mut DbConn<'_>) {
        sql_query("insert into synthetic_tournament_qa.tournaments (id) values ($1)")
            .bind::<SqlUuid, _>(id)
            .execute(conn)
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cleanup_registry_retains_anonymized_account_identity() {
        let db = test_database::test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        ensure_fixture_registry(&mut conn).await.unwrap();
        let account = fixture::create_user(&format!("{ACCOUNT_PREFIX}deleted"), &mut conn).await;
        register_account(account.id, &mut conn).await;
        account
            .soft_delete("replacement-password-hash", &mut conn)
            .await
            .unwrap();
        let anonymized = User::find_by_uuid(&account.id, &mut conn).await.unwrap();
        assert!(anonymized.deleted);
        assert!(!anonymized.username.starts_with(ACCOUNT_PREFIX));
        assert_ne!(anonymized.email, account.email);
        assert_eq!(sql_query("select count(*)::bigint as count from synthetic_tournament_qa.accounts where id = $1")
            .bind::<SqlUuid, _>(account.id).get_result::<Count>(&mut conn).await.unwrap().count, 1);
        assert!(
            refuse_conflicts(&mut conn).await.is_err(),
            "anonymization must not allow reseeding over the registered account"
        );

        let summary = conn
            .transaction::<_, anyhow::Error, _>(async |tc| cleanup_transaction(tc).await)
            .await
            .unwrap();
        assert_eq!(summary.accounts_deleted, 1);
        assert!(users::table
            .find(account.id)
            .first::<User>(&mut conn)
            .await
            .optional()
            .unwrap()
            .is_none());
        assert_eq!(
            sql_query("select count(*)::bigint as count from synthetic_tournament_qa.accounts")
                .get_result::<Count>(&mut conn)
                .await
                .unwrap()
                .count,
            0
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cleanup_refuses_qa_coorganizer_on_an_unregistered_tournament_atomically() {
        let db = test_database::test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        ensure_fixture_registry(&mut conn).await.unwrap();
        let qa = fixture::create_user(&format!("{ACCOUNT_PREFIX}coorg"), &mut conn).await;
        let real = fixture::create_user("real_coorganizer", &mut conn).await;
        register_account(qa.id, &mut conn).await;
        let qa_tournament = fixture::create_rr_tournament(
            qa.id,
            "Registered cleanup cohort",
            TournamentStatus::NotStarted,
            &mut conn,
        )
        .await;
        register_tournament(qa_tournament.id, &mut conn).await;
        let real_tournament = fixture::create_rr_tournament(
            real.id,
            "Unrelated shared tournament",
            TournamentStatus::NotStarted,
            &mut conn,
        )
        .await;
        diesel::insert_into(tournaments_organizers::table)
            .values((
                tournaments_organizers::tournament_id.eq(real_tournament.id),
                tournaments_organizers::organizer_id.eq(qa.id),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        let error = conn
            .transaction::<_, anyhow::Error, _>(async |tc| cleanup_transaction(tc).await)
            .await
            .err()
            .expect("QA co-organizing an unrelated tournament must block cleanup");
        let reason = error.to_string();
        assert!(
            reason.contains("refusing synthetic cleanup")
                && reason.contains(&real_tournament.id.to_string()),
            "unexpected refusal: {reason}"
        );
        assert!(Tournament::find(real_tournament.id, &mut conn)
            .await
            .is_ok());
        assert!(Tournament::find(qa_tournament.id, &mut conn).await.is_ok());
        assert!(User::find_by_uuid(&qa.id, &mut conn).await.is_ok());
        assert_eq!(
            tournaments_organizers::table
                .filter(tournaments_organizers::tournament_id.eq(real_tournament.id))
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sql_query("select count(*)::bigint as count from synthetic_tournament_qa.tournaments")
                .get_result::<Count>(&mut conn)
                .await
                .unwrap()
                .count,
            1
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cleanup_removes_registered_games_and_preserves_unrelated_rows() {
        let db = test_database::test_db().await;
        let mut conn = get_conn(&db.pool).await.unwrap();
        ensure_fixture_registry(&mut conn).await.unwrap();
        let qa_owner = fixture::create_user(&format!("{ACCOUNT_PREFIX}owner"), &mut conn).await;
        let qa_player = fixture::create_user(&format!("{ACCOUNT_PREFIX}player"), &mut conn).await;
        let real_owner = fixture::create_user("real_owner", &mut conn).await;
        let real_player = fixture::create_user("real_player", &mut conn).await;
        for user in [&qa_owner, &qa_player] {
            register_account(user.id, &mut conn).await;
        }
        let qa_tournament = fixture::create_rr_tournament(
            qa_owner.id,
            "Cleanup fixture",
            TournamentStatus::NotStarted,
            &mut conn,
        )
        .await;
        register_tournament(qa_tournament.id, &mut conn).await;
        let real_tournament = fixture::create_rr_tournament(
            real_owner.id,
            "Unrelated cleanup control",
            TournamentStatus::NotStarted,
            &mut conn,
        )
        .await;
        let mut real_game_ids = Vec::new();
        for (tournament, owner, players) in [
            (&qa_tournament, qa_owner.id, vec![qa_owner.id, qa_player.id]),
            (
                &real_tournament,
                real_owner.id,
                vec![real_owner.id, real_player.id],
            ),
        ] {
            fixture::insert_unfrozen_memberships(
                tournament.id,
                &players,
                fixture::fixed_instant(),
                &mut conn,
            )
            .await;
            let started = fixed_field::start_by_organizer(tournament.id, owner, players, &mut conn)
                .await
                .unwrap();
            assert_eq!(started.games.len(), 1);
            if tournament.id == real_tournament.id {
                real_game_ids = started.games.into_iter().map(|game| game.id).collect();
            }
        }
        let mut real_users = vec![real_owner.id, real_player.id];
        real_users.sort_unstable();
        let real_ratings = ratings::table
            .filter(ratings::user_uid.eq_any(&real_users))
            .select((ratings::user_uid, ratings::speed, ratings::rating))
            .order((ratings::user_uid, ratings::speed))
            .load::<(Uuid, String, f64)>(&mut conn)
            .await
            .unwrap();
        let real_memberships = TournamentUser::find_by_tournament_id(real_tournament.id, &mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.user_id, row.pairing_number))
            .collect::<Vec<_>>();

        let summary = conn
            .transaction::<_, anyhow::Error, _>(async |tc| cleanup_transaction(tc).await)
            .await
            .unwrap();
        assert_eq!(summary.accounts_deleted, 2);
        assert_eq!(summary.tournaments_deleted, 1);
        assert_eq!(summary.games_deleted, 1);
        assert_eq!(summary.series_deleted, 0);
        assert_eq!(
            users::table
                .select(users::id)
                .order(users::id)
                .load::<Uuid>(&mut conn)
                .await
                .unwrap(),
            real_users
        );
        assert_eq!(
            tournaments::table
                .select(tournaments::id)
                .load::<Uuid>(&mut conn)
                .await
                .unwrap(),
            vec![real_tournament.id]
        );
        assert_eq!(
            games_table::table
                .select(games_table::id)
                .load::<Uuid>(&mut conn)
                .await
                .unwrap(),
            real_game_ids
        );
        assert_eq!(
            ratings::table
                .select((ratings::user_uid, ratings::speed, ratings::rating))
                .order((ratings::user_uid, ratings::speed))
                .load::<(Uuid, String, f64)>(&mut conn)
                .await
                .unwrap(),
            real_ratings
        );
        assert_eq!(
            TournamentUser::find_by_tournament_id(real_tournament.id, &mut conn)
                .await
                .unwrap()
                .into_iter()
                .map(|row| (row.user_id, row.pairing_number))
                .collect::<Vec<_>>(),
            real_memberships
        );
        assert_eq!(
            Tournament::find(real_tournament.id, &mut conn)
                .await
                .unwrap()
                .status(),
            TournamentStatus::InProgress
        );
    }

    #[test]
    fn account_namespace_fits_persisted_usernames() {
        assert!(ACCOUNT_PREFIX.starts_with("zz_qa_"));
        assert!(format!("{ACCOUNT_PREFIX}organizer").chars().count() <= 20);
        assert!(format!("{ACCOUNT_PREFIX}player_160").chars().count() <= 20);
        assert!(format!("{ACCOUNT_PREFIX}delete_test").chars().count() <= 20);
    }

    #[test]
    fn randomized_fixture_configurations_validate_across_seeds() {
        for seed in [0, 1, 7, 42, 0x51_7E_ED, u64::MAX] {
            let mut rng = StdRng::seed_from_u64(seed);
            let round_robin_available = round_robin_creation_tiebreakers(
                NonZeroU32::new(1).unwrap(),
                RoundRobinPrimaryScore::GamePoints,
            );
            for count in 2..=round_robin_available.len() {
                let selected = sample_creation_tiebreakers(&round_robin_available, count, &mut rng);
                assert_eq!(selected.len(), count.min(round_robin_available.len() - 1));
                assert!(choices_are_unique(&selected));
                assert!(
                    selected
                        .iter()
                        .filter(|criterion| criterion.is_seed())
                        .count()
                        <= 1
                );
                assert!(selected
                    .iter()
                    .all(|criterion| round_robin_available.contains(criterion)));
            }
            let swiss_available = swiss_creation_tiebreakers(None);
            for count in [2, swiss_available.len()] {
                let selected = sample_creation_tiebreakers(&swiss_available, count, &mut rng);
                assert_eq!(selected.len(), count.min(swiss_available.len() - 1));
                assert!(choices_are_unique(&selected));
                assert!(
                    selected
                        .iter()
                        .filter(|criterion| criterion.is_seed())
                        .count()
                        <= 1
                );
                assert!(selected
                    .iter()
                    .all(|criterion| swiss_available.contains(criterion)));
            }
            for _ in 0..32 {
                for clock in [
                    random_realtime_clock(&mut rng),
                    random_correspondence_days_clock(&mut rng),
                    random_correspondence_total_clock(&mut rng),
                ] {
                    assert!(random_elimination_config(
                        EliminationTopology::Single {
                            bronze: rng.random::<bool>(),
                        },
                        clock,
                        &mut rng,
                    )
                    .validate_for_creation()
                    .is_ok());
                }

                let rounds = NonZeroU32::new(rng.random_range(2..=7)).unwrap();
                assert!(
                    swiss_config_with_clock(rounds, random_realtime_clock(&mut rng))
                        .validate_for_creation()
                        .is_ok()
                );
            }
        }
    }

    fn choices_are_unique<T: PartialEq>(choices: &[T]) -> bool {
        choices
            .iter()
            .enumerate()
            .all(|(index, choice)| !choices[..index].contains(choice))
    }
}
