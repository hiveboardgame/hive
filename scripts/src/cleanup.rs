use crate::common::setup_database;
use anyhow::{Context, Result};
use db_lib::schema::{games, tournaments, tournaments_organizers, users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

/// Scoped so it cannot reach a real account. Only the email carries the marker —
/// seeded players have realistic names, so matching on those would be both
/// fragile and dangerously broad.
const EMAIL_PATTERN: &str = "tt-%@example.com";

pub async fn run(database_url: Option<String>) -> Result<()> {
    let mut conn = setup_database(database_url).await?;

    let seeded: Vec<Uuid> = users::table
        .filter(users::email.like(EMAIL_PATTERN))
        .filter(users::admin.eq(false))
        .select(users::id)
        .load(&mut conn)
        .await
        .context("could not find the seeded users")?;

    if seeded.is_empty() {
        println!("Nothing to clean up.");
        return Ok(());
    }

    // Tournaments go before games: a tournament row referencing a game that is
    // already gone would strand the tournament with no way to reach it.
    let organized: Vec<Uuid> = tournaments_organizers::table
        .filter(tournaments_organizers::organizer_id.eq_any(&seeded))
        .select(tournaments_organizers::tournament_id)
        .load(&mut conn)
        .await
        .context("could not find the seeded tournaments")?;

    let tournament_games =
        diesel::delete(games::table.filter(games::tournament_id.eq_any(&organized)))
            .execute(&mut conn)
            .await
            .context("could not delete the tournament games")?;

    let deleted_tournaments =
        diesel::delete(tournaments::table.filter(tournaments::id.eq_any(&organized)))
            .execute(&mut conn)
            .await
            .context("could not delete the tournaments")?;

    let loose_games = diesel::delete(
        games::table.filter(
            games::white_id
                .eq_any(&seeded)
                .or(games::black_id.eq_any(&seeded)),
        ),
    )
    .execute(&mut conn)
    .await
    .context("could not delete the remaining games")?;

    let deleted_users = diesel::delete(users::table.filter(users::id.eq_any(&seeded)))
        .execute(&mut conn)
        .await
        .context("could not delete the seeded users")?;

    println!(
        "Deleted {deleted_users} users, {deleted_tournaments} tournaments, \
         {} games (ratings and memberships cascaded).",
        tournament_games + loose_games
    );
    Ok(())
}
