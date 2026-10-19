use chrono::{Duration, Utc};
use db_lib::{
    db_error::DbError,
    game_command::{execute, Command, Outcome},
    get_conn,
    get_pool,
    models::{Challenge, Game, NewChallenge, NewGame, NewTournament, NewUser, Tournament, User},
    schema::{
        tournaments::dsl::{starts_at as tournament_starts_at, tournaments as tournaments_table},
        users::dsl::{email_verified, users as users_table},
    },
    tournaments::arena,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use hive_lib::{ColorChoice, GameType, Position, Turn};
use serde::{Deserialize, Serialize};
use shared_types::{
    tournament::{arena::Config as ArenaConfig, BotAdmission, Config, FormatConfig},
    ChallengeDetails,
    ChallengeVisibility,
    RealtimeClock,
    TimeMode,
    TournamentDetails,
};
use std::{env, error::Error, io, num::NonZeroU32, process::ExitCode};
use uuid::Uuid;

#[path = "hive_e2e/database.rs"]
mod database;

const PASSWORD: &str = "Hive-e2e-password1!";
const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$SGl2ZUUyRUZpeHR1cmVQd2Q$Eu9hFGZ7gREd6xhlajhr7WbCd8yjSSUvU1oAcB5L0xo";

#[derive(Deserialize)]
struct OpeningMove {
    piece: String,
    position: Position,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Scenario {
    Users,
    Game,
    Arena,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    scenario: Scenario,
    prefix: String,
    count: Option<usize>,
    plies: Option<usize>,
}

#[derive(Serialize)]
struct FixtureUser {
    id: Uuid,
    username: String,
    email: String,
    password: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    users: Vec<FixtureUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    game_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tournament_id: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hive-e2e: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let request: Request = serde_json::from_reader(io::stdin().lock())
        .map_err(|error| format!("expected one fixture JSON request on stdin: {error}"))?;
    if request.prefix.is_empty()
        || request.prefix.len() > 8
        || !request
            .prefix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err("prefix must contain 1 to 8 lowercase letters, digits, or underscores".into());
    }
    let count = match request.scenario {
        Scenario::Users => request.count.unwrap_or(2),
        Scenario::Game => {
            if request.count.is_some_and(|count| count != 2) {
                return Err("game fixtures require exactly two users".into());
            }
            2
        }
        Scenario::Arena => request.count.unwrap_or(4),
    };
    if !(1..=8).contains(&count) {
        return Err("count must be between 1 and 8".into());
    }
    if request.plies.is_some_and(|plies| plies > 7)
        || (!matches!(request.scenario, Scenario::Game) && request.plies.is_some())
    {
        return Err("plies is supported only for games and must be between 0 and 7".into());
    }
    let connection_string = env::var("PLAYWRIGHT_DATABASE_URL")
        .map_err(|_| "set PLAYWRIGHT_DATABASE_URL explicitly for the browser instance")?;
    database::validate(&connection_string)?;
    let pool = get_pool(&connection_string).await.map_err(|error| {
        format!("could not connect to the browser fixture database: {error}")
            .replace(&connection_string, "[browser fixture database]")
    })?;
    let mut conn = get_conn(&pool).await.map_err(|error| {
        format!("could not acquire a browser fixture database connection: {error}")
            .replace(&connection_string, "[browser fixture database]")
    })?;
    let response = conn
        .transaction::<_, DbError, _>(async move |conn| create_fixture(request, count, conn).await)
        .await?;
    serde_json::to_writer(io::stdout().lock(), &response)?;
    Ok(())
}

async fn create_fixture(
    request: Request,
    count: usize,
    conn: &mut DbConn<'_>,
) -> Result<Response, DbError> {
    let suffix = Uuid::new_v4().simple().to_string();
    let mut fixture_users = Vec::with_capacity(count);
    for index in 0..count {
        let username = format!("{}_{}{index}", request.prefix, &suffix[..10]);
        let email = format!("{username}@example.test");
        let user = User::create(NewUser::new(&username, PASSWORD_HASH, &email)?, conn).await?;
        diesel::update(users_table.find(user.id))
            .set(email_verified.eq(true))
            .execute(conn)
            .await?;
        fixture_users.push(FixtureUser {
            id: user.id,
            username,
            email,
            password: PASSWORD,
        });
    }
    let mut response = Response {
        users: fixture_users,
        game_id: None,
        tournament_id: None,
    };
    match request.scenario {
        Scenario::Users => {}
        Scenario::Game => {
            response.game_id = Some(
                create_game(&response.users, request.plies.unwrap_or(7), conn)
                    .await?
                    .nanoid,
            );
        }
        Scenario::Arena => {
            response.tournament_id = Some(
                create_arena(response.users[0].id, &request.prefix, &suffix, conn)
                    .await?
                    .nanoid,
            );
        }
    }
    Ok(response)
}

async fn create_game(
    players: &[FixtureUser],
    plies: usize,
    conn: &mut DbConn<'_>,
) -> Result<Game, DbError> {
    let details = ChallengeDetails {
        rated: false,
        game_type: GameType::MLP,
        visibility: ChallengeVisibility::Direct,
        opponent: Some(players[1].username.clone()),
        color_choice: ColorChoice::White,
        time_mode: TimeMode::RealTime,
        time_base: Some(1200),
        time_increment: Some(20),
        band_upper: None,
        band_lower: None,
    };
    let challenge = NewChallenge::new(players[0].id, Some(players[1].id), &details, conn).await?;
    let challenge = Challenge::create(&challenge, conn).await?;
    let new_game = NewGame::new(players[0].id, players[1].id, &challenge)?;
    let (mut game, _) = Game::create_and_delete_challenges(new_game, conn).await?;
    let opening: Vec<OpeningMove> = serde_json::from_str(include_str!(
        "../../../apis/end2end/assets/stacked-opening.json"
    ))
    .map_err(|error| DbError::InvalidAction {
        info: format!("invalid browser fixture opening: {error}"),
    })?;
    for (index, opening_move) in opening.into_iter().take(plies).enumerate() {
        let outcome = execute(
            game.id,
            Command::Move {
                user_id: players[index % 2].id,
                turn: Turn::Move(
                    opening_move
                        .piece
                        .parse()
                        .map_err(|error| DbError::InvalidAction {
                            info: format!("invalid browser fixture piece: {error}"),
                        })?,
                    opening_move.position,
                ),
                compensation: 0.0,
                expected_turn: Some(index),
            },
            conn,
        )
        .await?;
        game = match outcome {
            Outcome::Applied { game, .. } => game,
            _ => {
                return Err(DbError::InvalidAction {
                    info: String::from("fixture opening did not apply"),
                });
            }
        };
    }
    Ok(game)
}

async fn create_arena(
    organizer: Uuid,
    prefix: &str,
    suffix: &str,
    conn: &mut DbConn<'_>,
) -> Result<Tournament, DbError> {
    let now = Utc::now();
    let new_tournament = NewTournament::new(TournamentDetails {
        name: format!("{prefix} Arena {suffix}"),
        description: None,
        seats: None,
        min_seats: 0,
        invite_only: false,
        band_upper: None,
        band_lower: None,
        starts_at: Some(now + Duration::hours(1)),
        configuration: Config {
            bot_admission: BotAdmission::HumansOnly,
            format: FormatConfig::Arena(ArenaConfig::new(
                NonZeroU32::new(3600).expect("nonzero arena duration"),
                RealtimeClock {
                    base_seconds: NonZeroU32::new(300).expect("nonzero game clock"),
                    increment_seconds: 3,
                },
            )),
        },
    })?;
    let tournament = Tournament::create(organizer, &new_tournament, conn).await?;
    // Move the schedule into the past, then let the production start operation
    // initialize the lifecycle. No browser must wait for a wall-clock boundary.
    diesel::update(tournaments_table.find(tournament.id))
        .set(tournament_starts_at.eq(Some(now - Duration::seconds(1))))
        .execute(conn)
        .await?;
    Ok(
        arena::start_scheduled_with_presence(tournament.id, |_| Default::default(), conn)
            .await?
            .tournament,
    )
}
