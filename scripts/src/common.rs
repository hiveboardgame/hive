use anyhow::{Context, Result};
use db_lib::{
    get_conn,
    get_pool,
    models::{NewUser, User},
    schema::ratings,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use dotenvy::dotenv;
use shared_types::GameSpeed;

/// Every seeded account shares this, matching the convention for hive test
/// accounts. Stored as an Argon2 hash, never as the plain string.
pub const PASSWORD: &str = "hivegame";

/// Large enough for a bracket that is not a neat power of two — 55 players means
/// nine byes in the first round, which is where a diagram stops being symmetric.
pub const PLAYER_COUNT: usize = 56;
pub const ORGANIZER: &str = "tournament_director";

/// Deliberately varied in length, up to the 20-character maximum, because a
/// cross-table's column widths and a standings row's truncation only misbehave
/// with real names — sixteen copies of `tt-04` prove nothing.
const PLAYER_NAMES: [&str; 16] = [
    "hive_mind_collective",
    "GrasshopperGandalf",
    "MantisShrimpMaximus",
    "beetle_juice_2026",
    "TheAmazingBeetleman",
    "PillbugPhilomena",
    "MosquitoMarauder",
    "SpiderSenseSteve",
    "DrosophilaDynamo",
    "GrandmasterGrub",
    "wasp_warrior_99",
    "Ladybug_Larry",
    "queenbee",
    "Cicada",
    "Bee",
    "ant",
];

/// Leaks the pool so the connection can be `'static`, which is harmless in a
/// one-shot binary and saves threading a lifetime through every command.
pub async fn setup_database(database_url: Option<String>) -> Result<DbConn<'static>> {
    dotenv().ok();

    let database_url = database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .context("DATABASE_URL must be set, or --database-url provided")?;

    let pool = get_pool(&database_url)
        .await
        .context("could not build a connection pool")?;

    let static_pool = Box::leak(Box::new(pool));
    get_conn(static_pool)
        .await
        .context("could not take a connection from the pool")
}

/// `NewUser::new` stores whatever it is handed, so hashing has to happen here.
///
/// The hashing in `apis` is unreachable — libraries must never depend on it —
/// so this mirrors `apis/src/functions/auth/password.rs` against `argon2`
/// directly. A raw password would not parse as a PHC string, and
/// `verify_password` fails before comparing anything, so the account could
/// never log in.
fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use rand::Rng;

    // `SaltString::generate` needs `OsRng`, which is compiled out unless
    // rand_core's `getrandom` feature happens to be on. The salt is drawn from
    // `rand` instead so this does not depend on another crate's feature flags.
    let mut salt_bytes = [0_u8; 16];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|error| anyhow::anyhow!("could not encode a password salt: {error}"))?;

    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("could not hash the seed password: {error}"))?
        .to_string())
}

/// The pool: `tt-01`..`tt-16` plus `tt-org`, created on first run and reused
/// afterwards so the script can be run repeatedly.
///
/// Ratings descend by 10 from 2000 so seeding is deterministic and the seed
/// order is legible in the UI — users are created at a flat 1500, which would
/// leave seeds to fall out of uuid order.
pub async fn ensure_pool(conn: &mut DbConn<'_>) -> Result<(User, Vec<User>)> {
    // Slot 0 is the organizer's; players take 1 upwards.
    let organizer = ensure_user(ORGANIZER, 0, 2500.0, conn).await?;

    let mut players = Vec::with_capacity(PLAYER_COUNT);
    for index in 0..PLAYER_COUNT {
        // The curated names cover the small fields, where they are actually read;
        // past that the point is bracket size, so they are generated.
        let username = match PLAYER_NAMES.get(index) {
            Some(name) => (*name).to_owned(),
            None => format!("swarm_member_{:02}", index + 1),
        };
        let rating = 2000.0 - (index as f64) * 10.0;
        players.push(ensure_user(&username, index + 1, rating, conn).await?);
    }

    Ok((organizer, players))
}

/// Emails carry the seed marker rather than usernames, so `cleanup` stays exact
/// while the names themselves can be anything.
fn seed_email(slot: usize) -> String {
    format!("tt-{slot:02}@example.com")
}

async fn ensure_user(
    username: &str,
    slot: usize,
    rating: f64,
    conn: &mut DbConn<'_>,
) -> Result<User> {
    if let Ok(user) = User::find_by_username(username, conn).await {
        return Ok(user);
    }

    let hashed = hash_password(PASSWORD)?;
    let new_user = NewUser::new(username, &hashed, &seed_email(slot))
        .with_context(|| format!("invalid seed user {username}"))?;
    let user = User::create(new_user, conn)
        .await
        .with_context(|| format!("could not create {username}"))?;

    // Tournaments seed from the Bullet rating, because every fixture is 60+0.
    diesel::update(
        ratings::table.filter(
            ratings::user_uid
                .eq(user.id)
                .and(ratings::speed.eq(GameSpeed::Bullet.to_string())),
        ),
    )
    .set(ratings::rating.eq(rating))
    .execute(conn)
    .await
    .with_context(|| format!("could not set {username}'s rating"))?;

    tracing::info!(username, rating, "created seed user");
    Ok(user)
}
