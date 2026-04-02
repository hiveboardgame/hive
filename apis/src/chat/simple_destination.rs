use shared_types::TournamentId;

#[derive(Debug, Clone)]
pub enum SimpleDestination {
    User,
    Game,
    Tournament(TournamentId),
    Global,
}
