use super::{
    challenge_action::ChallengeAction,
    game_action::GameAction,
    ScheduleAction,
    TournamentAction,
};
use serde::{Deserialize, Serialize};
use shared_types::{ChatMessageContainer, GameId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    Chat(ChatMessageContainer),
    Challenge(ChallengeAction),
    Game { game_id: GameId, action: GameAction },
    LinkDiscord,
    Pong(u64),
    Resync,
    Schedule(ScheduleAction),
    Tournament(TournamentAction),
    // leptos-use idle or window unfocused will send
    Away, // Online and Offline are not needed because they will be handled by the WS connection
    // being established/torn down
    // TODO: all the other things the API does right now
    // Bearer token sent as the first frame after the WS opens. Connection
    // starts anonymous; on receipt the backend re-binds the socket to the
    // decoded user. Used by cross-origin clients (HiveGame mobile) that can't
    // rely on the session cookie. SSR + hydrate same-origin clients never
    // emit this. Kept at the end of the enum so adding it doesn't shift the
    // serialized variant index of pre-existing variants.
    Auth(String),
}
