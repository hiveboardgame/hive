use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("User not found with id: {id}")]
    UserNotFound { id: String },

    #[error("Game not found with id: {id}")]
    GameNotFound { id: String },

    #[error("Player not found with id: {id}")]
    PlayerNotFound { id: String },

    #[error("Tournament does not have enough players")]
    NotEnoughPlayers,

    #[error("Tournament is full")]
    TournamentFull,

    #[error("Cannot join an invite only tournament")]
    TournamentInviteOnly,

    #[error("Tournament invitation not found")]
    TournamentInvitationNotFound,

    #[error("Tournament not found with id: {id}")]
    TournamentNotFound { id: String },

    #[error("Organizer not found with id: {id}")]
    OrganizerNotFound { id: String },

    #[error("Game already finished, not accepting new moves")]
    GameIsOver,

    #[error("Invalid tournament details: {info}")]
    InvalidTournamentDetails { info: String },

    #[error("Invalid Input: {error}")]
    InvalidInput { info: String, error: String },

    #[error("Invalid action: {info}")]
    InvalidAction { info: String },

    #[error("Not found: {reason}")]
    NotFound { reason: String },

    #[error("Time not found: {reason}")]
    TimeNotFound { reason: String },

    #[error("You are not authorized to perform that action")]
    Unauthorized,

    #[error("Internal error in database operation")]
    InternalError,

    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Format error: {0}")]
    FormatError(String),
}

impl From<shared_types::ChallengeError> for DbError {
    fn from(err: shared_types::ChallengeError) -> DbError {
        match err {
            shared_types::ChallengeError::NotValidTimeMode { found } => {
                DbError::TimeNotFound { reason: found }
            }
            _ => DbError::InternalError,
        }
    }
}

impl From<std::fmt::Error> for DbError {
    fn from(error: std::fmt::Error) -> Self {
        DbError::FormatError(error.to_string())
    }
}

impl DbError {
    pub fn log_detailed(&self) {
        match self {
            DbError::UserNotFound { id } => println!("ERROR: User not found with id: {}", id),
            DbError::GameNotFound { id } => println!("ERROR: Game not found with id: {}", id),
            DbError::PlayerNotFound { id } => println!("ERROR: Player not found with id: {}", id),
            DbError::NotEnoughPlayers => println!("ERROR: Not enough players in tournament"),
            DbError::TournamentFull => println!("ERROR: Tournament is full"),
            DbError::TournamentInviteOnly => println!("ERROR: Tournament requires an invitation"),
            DbError::TournamentInvitationNotFound => {
                println!("ERROR: Tournament invitation not found")
            }
            DbError::TournamentNotFound { id } => {
                println!("ERROR: Tournament not found with id: {}", id)
            }
            DbError::OrganizerNotFound { id } => {
                println!("ERROR: Organizer not found with id: {}", id)
            }
            DbError::GameIsOver => println!("ERROR: Game is already over"),
            DbError::InvalidTournamentDetails { info } => {
                println!("ERROR: Invalid tournament details: {}", info)
            }
            DbError::InvalidInput { info, error } => {
                println!("ERROR: Invalid input: {}", info);
                println!("ERROR DETAILS: {}", error);
            }
            DbError::InvalidAction { info } => println!("ERROR: Invalid action: {}", info),
            DbError::NotFound { reason } => println!("ERROR: Resource not found: {}", reason),
            DbError::TimeNotFound { reason } => println!("ERROR: Time not found: {}", reason),
            DbError::Unauthorized => println!("ERROR: Unauthorized operation"),
            DbError::InternalError => println!("ERROR: Internal error (unexpected condition)"),
            DbError::DatabaseError(e) => println!("ERROR: Database error: {:?}", e),
            DbError::IoError(e) => println!("ERROR: IO error: {:?}", e),
            DbError::FormatError(e) => println!("ERROR: Format error: {}", e),
        }
    }
}
