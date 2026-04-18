use serde::{Deserialize, Serialize};

use crate::GameThread;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameChatCapabilities {
    pub is_player: bool,
    pub finished: bool,
}

impl GameChatCapabilities {
    pub const fn new(is_player: bool, finished: bool) -> Self {
        Self { is_player, finished }
    }

    pub const fn can_read(self, thread: GameThread) -> bool {
        match thread {
            GameThread::Players => self.is_player,
            GameThread::Spectators => !self.is_player || self.finished,
        }
    }

    pub const fn can_send(self, thread: GameThread) -> bool {
        match thread {
            GameThread::Players => self.is_player,
            GameThread::Spectators => !self.is_player || self.finished,
        }
    }

    pub const fn can_toggle_embedded_threads(self) -> bool {
        self.is_player
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TournamentChatCapabilities {
    pub is_site_admin: bool,
    pub is_organizer: bool,
    pub is_participant: bool,
}

impl TournamentChatCapabilities {
    pub const fn new(is_site_admin: bool, is_organizer: bool, is_participant: bool) -> Self {
        Self {
            is_site_admin,
            is_organizer,
            is_participant,
        }
    }

    pub const fn can_read(self) -> bool {
        self.is_site_admin || self.is_organizer || self.is_participant
    }

    pub const fn can_send(self) -> bool {
        self.can_read()
    }
}
