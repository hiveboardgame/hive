use serde::{Deserialize, Serialize};

use crate::GameThread;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameChatCapabilities {
    pub is_player: bool,
    pub finished: bool,
}

impl GameChatCapabilities {
    pub const fn new(is_player: bool, finished: bool) -> Self {
        Self {
            is_player,
            finished,
        }
    }

    pub const fn can_read(self, thread: GameThread) -> bool {
        match thread {
            GameThread::Players => self.is_player,
            GameThread::Spectators => !self.is_player || self.finished,
        }
    }

    pub const fn can_toggle_embedded_threads(self) -> bool {
        self.is_player
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_players_are_excluded_from_spectator_chat() {
        let player = GameChatCapabilities::new(true, false);
        let spectator = GameChatCapabilities::new(false, false);

        assert!(!player.can_read(GameThread::Spectators));
        assert!(spectator.can_read(GameThread::Spectators));
    }

    #[test]
    fn finished_players_can_use_spectator_chat() {
        let player = GameChatCapabilities::new(true, true);

        assert!(player.can_read(GameThread::Spectators));
    }
}
