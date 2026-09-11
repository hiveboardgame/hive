use std::sync::Arc;

use crate::{
    common::{GameUpdate, ServerMessage},
    responses::{GameResponse, UserResponse},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use shared_types::GameId;
use uuid::Uuid;

pub enum BotRead {
    Game(GameId),
    PendingGames,
    User(Uuid),
    Username(String),
}

pub struct BotReadHandler {
    read: BotRead,
    received_from: SocketTx,
    user_id: Uuid,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl BotReadHandler {
    pub fn new(
        read: BotRead,
        received_from: SocketTx,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            read,
            received_from,
            user_id,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let message = match &self.read {
            // No ownership filter, matching `GameSelector::Specific`: bots read other
            // people's games.
            BotRead::Game(game_id) => {
                let game = Game::find_by_game_id(game_id, &mut conn).await?;
                let response = self.data.get_or_build_response(&game, &mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Fetched(response.clone())))
            }
            BotRead::PendingGames => {
                let user = User::find_active_by_uuid(&self.user_id, &mut conn).await?;
                let pending = user.get_games_with_notifications(&mut conn).await?;
                // Between sweep ticks a row can be past timeout but not yet finalized;
                // settling here is what keeps this the same answer as
                // `GET /api/v1/bot/games/pending`, which the bot API documents it as.
                let mut games = Vec::with_capacity(pending.len());
                for game in pending {
                    let game = game.check_time(&mut conn).await?;
                    if !game.finished {
                        games.push(game);
                    }
                }
                ServerMessage::Game(Box::new(GameUpdate::Urgent(
                    GameResponse::from_games_batch(games, &mut conn).await?,
                )))
            }
            BotRead::User(id) => {
                ServerMessage::UserProfile(UserResponse::from_uuid(id, &mut conn).await?)
            }
            BotRead::Username(name) => {
                ServerMessage::UserProfile(UserResponse::from_username(name, &mut conn).await?)
            }
        };
        Ok(HandlerOutput {
            messages: vec![InternalServerMessage {
                destination: MessageDestination::Direct(self.received_from),
                message,
            }],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::{
        live_test_pool,
        messages::{SocketFormat, SocketTx},
    };
    use chrono::Utc;
    use db_lib::{
        models::{NewGame, NewUser},
        DbConn,
    };
    use hive_lib::{GameStatus, GameType};
    use shared_types::{Conclusion, GameSpeed, GameStart, TimeMode, TournamentGameResult};
    use tokio::sync::mpsc;

    fn socket() -> SocketTx {
        let (tx, _rx) = mpsc::channel(8);
        SocketTx {
            socket_id: Uuid::new_v4(),
            format: SocketFormat::Json,
            tx,
        }
    }

    async fn user(prefix: &str, conn: &mut DbConn<'_>) -> User {
        let username = format!("{prefix}{}", &Uuid::new_v4().simple().to_string()[..8]);
        let email = format!("{username}@example.com");
        let new_user = NewUser::new(&username, "password", &email).expect("valid user");
        User::create(new_user, conn).await.expect("user is created")
    }

    /// A realtime game whose clock ran out while nothing was watching: the row still says
    /// `finished = false`, which is exactly the state a sweep tick has not yet reached.
    /// Turn 1 puts Black on the clock, so the bot is both the timed-out side and the
    /// `current_player_id` that `get_games_with_notifications` selects on.
    async fn expired_bot_game(conn: &mut DbConn<'_>) -> (User, Game) {
        let opponent = user("timeoutfoe", conn).await;
        let bot = user("timeoutbot", conn).await;
        let long_ago = Utc::now() - chrono::Duration::minutes(10);
        let one_minute = Some(60_000_000_000_i64);

        let game = Game::create(
            NewGame {
                nanoid: Uuid::new_v4().simple().to_string()[..12].to_string(),
                current_player_id: bot.id,
                black_id: bot.id,
                finished: false,
                game_status: GameStatus::InProgress.to_string(),
                game_type: GameType::MLP.to_string(),
                history: "wQ;".to_string(),
                game_control_history: String::new(),
                rated: true,
                tournament_queen_rule: false,
                turn: 1,
                white_id: opponent.id,
                white_rating: None,
                black_rating: None,
                white_rating_change: None,
                black_rating_change: None,
                created_at: long_ago,
                updated_at: long_ago,
                time_mode: TimeMode::RealTime.to_string(),
                time_base: Some(60),
                time_increment: Some(0),
                last_interaction: Some(long_ago),
                black_time_left: one_minute,
                white_time_left: one_minute,
                speed: GameSpeed::Bullet.to_string(),
                hashes: Vec::new(),
                conclusion: Conclusion::Unknown.to_string(),
                tournament_id: None,
                tournament_game_result: TournamentGameResult::Unknown.to_string(),
                game_start: GameStart::Moves.to_string(),
                move_times: Vec::new(),
                timeout_at: Some(long_ago + chrono::Duration::seconds(60)),
            },
            conn,
        )
        .await
        .expect("the expired game is created");

        (bot, game)
    }

    async fn bot_read(read: BotRead, user_id: Uuid, pool: &DbPool) -> HandlerOutput {
        BotReadHandler::new(
            read,
            socket(),
            user_id,
            Arc::new(WebsocketData::default()),
            pool,
        )
        .handle()
        .await
        .expect("the read succeeds")
    }

    fn only_game_update(output: HandlerOutput) -> GameUpdate {
        let message = output
            .messages
            .into_iter()
            .next()
            .expect("a read answers with one message");
        let ServerMessage::Game(update) = message.message else {
            panic!("a bot read answers with a game update");
        };
        *update
    }

    fn urgent_games(output: HandlerOutput) -> Vec<GameResponse> {
        let GameUpdate::Urgent(games) = only_game_update(output) else {
            panic!("a pending read answers with Urgent");
        };
        games
    }

    fn fetched_game(output: HandlerOutput) -> Arc<GameResponse> {
        let GameUpdate::Fetched(game) = only_game_update(output) else {
            panic!("a single-game read answers with Fetched");
        };
        game
    }

    /// `GET /api/v1/bot/games/pending` settles each row's clock and drops the ones that
    /// finished; `BOT_WEBSOCKET_API.md` documents `GetPendingGames` as the same request.
    /// A bot that believes an expired game is still its turn will sit there moving into a
    /// game the HTTP side already scored.
    #[actix_rt::test]
    async fn a_pending_read_never_returns_a_game_whose_clock_ran_out() {
        let Some(pool) = live_test_pool().await else {
            eprintln!("skipped: no reachable TEST_DATABASE_URL");
            return;
        };
        let mut conn = get_conn(&pool).await.expect("test database connection");
        let (bot, game) = expired_bot_game(&mut conn).await;

        let unsettled = bot
            .get_games_with_notifications(&mut conn)
            .await
            .expect("the pending query runs");
        assert!(
            unsettled.iter().any(|candidate| candidate.id == game.id),
            "test setup is wrong: the expired game is not on the raw pending list"
        );

        let games = urgent_games(bot_read(BotRead::PendingGames, bot.id, &pool).await);

        assert!(
            !games
                .iter()
                .any(|response| response.game_id.0 == game.nanoid),
            "an expired game came back as pending over the websocket; the HTTP endpoint \
             settles it with check_time and filters it out"
        );
    }

    /// The counterpart the pending read has to match: `find_by_game_id` settles the clock
    /// on the way out, so a single-game read already reports the timeout.
    #[actix_rt::test]
    async fn reading_one_game_settles_its_clock() {
        let Some(pool) = live_test_pool().await else {
            eprintln!("skipped: no reachable TEST_DATABASE_URL");
            return;
        };
        let mut conn = get_conn(&pool).await.expect("test database connection");
        let (bot, game) = expired_bot_game(&mut conn).await;

        let fetched =
            fetched_game(bot_read(BotRead::Game(GameId(game.nanoid)), bot.id, &pool).await);

        assert!(
            fetched.finished,
            "a single-game read must report the timeout the clock already reached"
        );
    }
}
