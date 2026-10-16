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
    OngoingGames,
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
                let games = user.get_games_with_notifications(&mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Urgent(
                    GameResponse::from_games_batch(games, &mut conn).await?,
                )))
            }
            BotRead::OngoingGames => {
                let user = User::find_active_by_uuid(&self.user_id, &mut conn).await?;
                let games = user.get_ongoing_games(&mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Ongoing(
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
        messages::{SocketFormat, SocketTx},
        test_db::test_db,
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

    /// A realtime game with `to_move` on the clock. A deadline in the past is a game whose
    /// clock ran out while nothing was watching: the row still says `finished = false`,
    /// which is exactly the state a sweep tick has not yet reached.
    async fn realtime_game(
        white: &User,
        black: &User,
        to_move: &User,
        timeout_at: chrono::DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Game {
        let started = Utc::now() - chrono::Duration::minutes(10);
        let one_minute = Some(60_000_000_000_i64);
        let (history, turn) = if to_move.id == white.id {
            ("wQ;bQ wQ-;".to_string(), 2)
        } else {
            ("wQ;".to_string(), 1)
        };
        Game::create(
            NewGame {
                nanoid: Uuid::new_v4().simple().to_string()[..12].to_string(),
                current_player_id: to_move.id,
                black_id: black.id,
                finished: false,
                game_status: GameStatus::InProgress.to_string(),
                game_type: GameType::MLP.to_string(),
                history,
                game_control_history: String::new(),
                rated: true,
                tournament_queen_rule: false,
                turn,
                white_id: white.id,
                white_rating: None,
                black_rating: None,
                white_rating_change: None,
                black_rating_change: None,
                created_at: started,
                updated_at: started,
                time_mode: TimeMode::RealTime.to_string(),
                time_base: Some(60),
                time_increment: Some(0),
                last_interaction: Some(started),
                black_time_left: one_minute,
                white_time_left: one_minute,
                speed: GameSpeed::Bullet.to_string(),
                hashes: Vec::new(),
                conclusion: Conclusion::Unknown.to_string(),
                tournament_id: None,
                tournament_game_result: TournamentGameResult::Unknown.to_string(),
                game_start: GameStart::Moves.to_string(),
                move_times: Vec::new(),
                timeout_at: Some(timeout_at),
                white_berserked: false,
                black_berserked: false,
                arena_move_due_at: None,
                tournament_slot_id: None,
                arena_ordinal: None,
            },
            conn,
        )
        .await
        .expect("the game is created")
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

    fn game_ids(games: &[GameResponse]) -> Vec<&str> {
        games.iter().map(|game| game.game_id.0.as_str()).collect()
    }

    /// A bot that believes an expired game is still its turn sits there moving into a game
    /// the server has already scored.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_pending_read_lists_live_turns_but_never_an_expired_one() {
        let db = test_db().await;
        let pool = db.pool.clone();
        let mut conn = get_conn(&pool).await.expect("test database connection");
        let opponent = user("foe", &mut conn).await;
        let bot = user("bot", &mut conn).await;
        let later = Utc::now() + chrono::Duration::minutes(1);
        let earlier = Utc::now() - chrono::Duration::minutes(1);
        let live = realtime_game(&opponent, &bot, &bot, later, &mut conn).await;
        let expired = realtime_game(&opponent, &bot, &bot, earlier, &mut conn).await;

        let GameUpdate::Urgent(games) =
            only_game_update(bot_read(BotRead::PendingGames, bot.id, &pool).await)
        else {
            panic!("a pending read answers with Urgent");
        };

        assert_eq!(game_ids(&games), vec![live.nanoid.as_str()]);
        assert!(!game_ids(&games).contains(&expired.nanoid.as_str()));
    }

    /// The games a bot waits in are the ones the lobby snapshot cannot give it back after
    /// a reconnect.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_ongoing_read_includes_games_waiting_on_the_opponent() {
        let db = test_db().await;
        let pool = db.pool.clone();
        let mut conn = get_conn(&pool).await.expect("test database connection");
        let opponent = user("foe", &mut conn).await;
        let bot = user("bot", &mut conn).await;
        let later = Utc::now() + chrono::Duration::minutes(1);
        let earlier = Utc::now() - chrono::Duration::minutes(1);
        let my_turn = realtime_game(&opponent, &bot, &bot, later, &mut conn).await;
        let their_turn = realtime_game(&opponent, &bot, &opponent, later, &mut conn).await;
        let expired = realtime_game(&opponent, &bot, &opponent, earlier, &mut conn).await;

        let GameUpdate::Ongoing(games) =
            only_game_update(bot_read(BotRead::OngoingGames, bot.id, &pool).await)
        else {
            panic!("an ongoing read answers with Ongoing");
        };

        let mut ids = game_ids(&games);
        ids.sort_unstable();
        let mut expected = vec![my_turn.nanoid.as_str(), their_turn.nanoid.as_str()];
        expected.sort_unstable();
        assert_eq!(ids, expected);
        assert!(!ids.contains(&expired.nanoid.as_str()));
    }
}
