use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};

use shared_types::{ChatMessageContainer, GameId};
use uuid::Uuid;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct UserToUser {
    pub id: (Uuid, Uuid),
}

impl UserToUser {
    pub fn new(user_1: Uuid, user_2: Uuid) -> Self {
        if user_1 < user_2 {
            Self {
                id: (user_1, user_2),
            }
        } else {
            Self {
                id: (user_2, user_1),
            }
        }
    }
}

#[derive(Debug)]
pub struct Chats {
    pub tournament: RwLock<HashMap<String, Vec<ChatMessageContainer>>>,
    pub games_public: RwLock<HashMap<GameId, Vec<ChatMessageContainer>>>,
    pub games_private: RwLock<HashMap<GameId, Vec<ChatMessageContainer>>>,
    pub direct: RwLock<HashMap<UserToUser, Vec<ChatMessageContainer>>>,
    pub direct_lookup: RwLock<HashMap<Uuid, HashSet<Uuid>>>,
}
impl Default for Chats {
    fn default() -> Self {
        Self::new()
    }
}

impl Chats {
    pub fn new() -> Self {
        Self {
            tournament: RwLock::new(HashMap::new()),
            games_public: RwLock::new(HashMap::new()),
            games_private: RwLock::new(HashMap::new()),
            direct: RwLock::new(HashMap::new()),
            direct_lookup: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert_or_update_direct_lookup(&self, id1: Uuid, id2: Uuid) {
        let mut direct_lookup = self.direct_lookup.write().unwrap();
        direct_lookup.entry(id1).or_default().insert(id2);
        direct_lookup.entry(id2).or_default().insert(id1);
    }
}
