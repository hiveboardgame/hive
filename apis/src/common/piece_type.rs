use std::fmt;

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub enum PieceType {
    // movable pieve on the board
    Board,
    // covered piece on the board
    Covered,
    // piece in history view
    History,
    // not your turn
    Inactive,
    // a not yet moved piece on the board
    Move,
    // uninteractive
    #[default]
    Nope,
    // piece in reserve
    Reserve,
    // a not yet spawned piece on a spawn point
    Spawn,
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            PieceType::Board => "board",
            PieceType::Covered => "covered",
            PieceType::Inactive => "inactive",
            PieceType::History => "history",
            PieceType::Nope => "nope",
            PieceType::Reserve => "reserve",
            PieceType::Spawn => "spawn",
            PieceType::Move => "move",
        };
        write!(f, "{}", name)
    }
}
