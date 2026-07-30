use serde::{Deserialize, Serialize};
use shared_types::{ArenaEventKind, GameId, TournamentDetails, TournamentGameResult, TournamentId};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TournamentResponseDepth {
    Full,
    Abstract,
}

/// Not `Eq`: `TournamentDetails` carries the tournament's point values as
/// `f64`, and a tournament is configurable down to half a point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TournamentAction {
    Abandon(TournamentId),
    // TODO: AddToSeries(TournamentId),
    AdjudicateResult(GameId, TournamentGameResult),
    DoubleForfeitUnstartedGames(TournamentId),
    ResetAdjudicatedGames(TournamentId),
    Create(Box<TournamentDetails>),
    Delete(TournamentId),
    Finish(TournamentId),
    ProgressToNextRound(TournamentId),
    InvitationAccept(TournamentId),
    InvitationCreate(TournamentId, Uuid),
    InvitationDecline(TournamentId),
    InvitationRetract(TournamentId, Uuid),
    Join(TournamentId),
    Kick(TournamentId, Uuid),
    Leave(TournamentId),
    // TODO: RemoveFromSeries(TournamentId),
    Start(TournamentId),
    /// Leaving a tournament that has already started, as opposed to `Leave`,
    /// which only applies before it does. Either the player themselves or an
    /// organizer may do it; the `Uuid` is the player leaving.
    Withdraw(TournamentId, Uuid),
    /// Undoes a withdrawal. Organizers only.
    Reinstate(TournamentId, Uuid),
    /// An arena admits players while it runs, so joining one is a different
    /// action from joining a tournament before it starts.
    JoinArena(TournamentId),
    /// Stepping out of an arena's pairing pool and back into it, or leaving for
    /// good. One action because the engine records all three the same way.
    ArenaBreak(TournamentId, ArenaEventKind),
    /// Give up half the clock and the whole increment for the arena's scoring
    /// bonus. Declared per game, before it starts.
    Berserk(GameId),
    /// Sit a player out of the round about to be paired, for
    /// `points_zero_point_bye`. Organizers only.
    GrantZeroPointBye(TournamentId, Uuid),
}
