use super::game_action::GameAction;
use super::{challenge_action::ChallengeAction, ScheduleAction, TournamentAction};
use serde::{Deserialize, Serialize};
use shared_types::{ChatMessageContainer, GameId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Chat(ChatMessageContainer),
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    LinkDiscord,
    Pong(u64),
    Schedule(ScheduleAction),
    Tournament(TournamentAction),
    UpdateId,
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
    // being established/torn down
    // TODO: all the other things the API does right now
    DbgMsg(String),
}
