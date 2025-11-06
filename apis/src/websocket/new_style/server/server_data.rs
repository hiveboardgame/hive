use crate::{
    common::{ServerMessage, UserStatus, UserUpdate},
    responses::UserResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        new_style::server::TabData,
        TournamentGameStart,
    },
};
use db_lib::models::Game;
use shared_types::{GameId, TournamentId};
use std::collections::HashMap;
use std::sync::RwLock;
use tokio::sync::broadcast::{channel, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default, Debug)]
struct SubscriberSet {
    map: HashMap<Uuid, CancellationToken>,
}

impl SubscriberSet {
    pub fn new(client: &TabData) -> Self {
        let mut set = Self {
            map: HashMap::default(),
        };
        set.insert(client);
        set
    }
    pub fn contains(&mut self, id: &Uuid) -> bool {
        //clean canceled first
        let _ = self.map.extract_if(|_c, v| v.is_cancelled());
        self.map.contains_key(id)
    }
    pub fn insert(&mut self, client: &TabData) {
        if client.is_cancelled() {
            return;
        }
        let (key, value) = client.as_subscriber();
        self.map.insert(key, value);
    }
    pub fn remove(&mut self, id: &Uuid) {
        self.map.remove(id);
    }
}

#[derive(Debug)]
pub struct ServerData {
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    sender: Sender<InternalServerMessage>,
    online_users: RwLock<HashMap<Uuid, (UserResponse, i32)>>,
    game_subscribers: RwLock<HashMap<GameId, SubscriberSet>>,
    tournament_subscribers: RwLock<HashMap<TournamentId, SubscriberSet>>,
    game_start: TournamentGameStart,
}
impl Default for ServerData {
    fn default() -> Self {
        //Capacity chosen arbitrarily
        let (sender, _) = channel(32);
        Self {
            sender,
            online_users: RwLock::new(HashMap::new()),
            game_subscribers: RwLock::new(HashMap::new()),
            tournament_subscribers: RwLock::new(HashMap::new()),
            game_start: TournamentGameStart::new(),
        }
    }
}
impl ServerData {
    pub fn notifications(&self) -> BroadcastStream<InternalServerMessage> {
        BroadcastStream::new(self.sender.subscribe())
    }
    pub fn send(&self, message: InternalServerMessage) -> Result<usize, String> {
        self.sender.send(message).map_err(|e| e.to_string())
    }

    fn notify_user_status(&self, user: UserResponse, status: UserStatus) {
        let message = InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::UserStatus(UserUpdate { user, status }),
        };
        self.send(message)
            .expect("Failed to send update status notification");
    }
    pub fn remove_user(&self, user: UserResponse) {
        let mut users = self.online_users.write().unwrap();
        let value = users.get(&user.uid).map(|(_, c)| *c);
        let username = &user.username;
        if let Some(count) = value {
            if count > 1 {
                println!("{username} closed a tab");
                users.insert(user.uid, (user.clone(), count - 1));
            } else {
                println!("{username} disconnected");
                users.remove(&user.uid);
                self.notify_user_status(user.clone(), UserStatus::Offline)
            }
        };
    }
    pub fn add_user(&self, user: UserResponse) {
        let mut users = self.online_users.write().unwrap();
        let (_, count) = *users
            .entry(user.uid)
            .and_modify(|(_, count)| {
                *count += 1;
            })
            .or_insert((user.clone(), 1));
        if count > 1 {
            println!("{} opened a tab", user.username);
        } else {
            println!("{} connected", user.username);
            self.notify_user_status(user.clone(), UserStatus::Online)
        };
    }
    pub fn get_online_users(&self) -> Vec<UserResponse> {
        self.online_users
            .read()
            .unwrap()
            .values()
            .map(|u| u.0.clone())
            .collect()
    }
    pub fn subscribe_client_to(&self, client: &TabData, game_id: GameId) {
        *client.subscribed_game.write().unwrap() = Some(game_id.clone());
        let mut subscribers = self.game_subscribers.write().unwrap();
        subscribers
            .entry(game_id.clone())
            .and_modify(|v| {
                v.insert(client);
            })
            .or_insert(SubscriberSet::new(client));
    }
    pub fn is_game_subscriber(&self, tab: &TabData, game_id: &GameId) -> bool {
        let (id, _) = tab.as_subscriber();
        let sub_game_id = tab.subscribed_game.read().unwrap();
        let mut game_subs = self.game_subscribers.write().unwrap();
        if sub_game_id.as_ref().is_some_and(|i| i == game_id) {
            game_subs.get_mut(game_id).is_some_and(|g| g.contains(&id))
        } else {
            if let Some(g) = game_subs.get_mut(game_id) {
                g.remove(&id)
            }
            false
        }
    }
    pub fn subscribe_to_tournament(&self, client: &TabData, tournament_id: TournamentId) {
        let mut subscribers = self.tournament_subscribers.write().unwrap();
        *client.subscribed_tournament.write().unwrap() = Some(tournament_id.clone());
        subscribers
            .entry(tournament_id.clone())
            .and_modify(|v| {
                v.insert(client);
            })
            .or_insert(SubscriberSet::new(client));
    }
    pub fn is_tournament_subscriber(&self, tab: &TabData, game_id: &TournamentId) -> bool {
        let (id, _) = tab.as_subscriber();

        let mut game_subs = self.tournament_subscribers.write().unwrap();
        let sub_t_id = tab.subscribed_tournament.read().unwrap();
        if sub_t_id.as_ref().is_some_and(|i| i == game_id) {
            game_subs.get_mut(game_id).is_some_and(|g| g.contains(&id))
        } else {
            if let Some(g) = game_subs.get_mut(game_id) {
                g.remove(&id)
            }
            false
        }
    }
    pub fn game_should_start(&self, game: &Game, user_id: Uuid) -> anyhow::Result<bool> {
        self.game_start.should_start(game, user_id)
    }
    pub fn active_games(&self) -> Vec<GameId> {
        let game_subs = self.game_subscribers.read().unwrap();
        game_subs.keys().cloned().collect()
    }
}
