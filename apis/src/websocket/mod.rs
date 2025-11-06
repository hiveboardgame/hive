mod chat;
mod lag_tracking;
pub use chat::Chats;
pub use lag_tracking::{Lags, Pings};
pub mod busybee;
pub mod client_handlers;
pub mod new_style;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    mod messages;
    mod tournament_game_start;
    pub mod server_handlers;

    pub use tournament_game_start::TournamentGameStart;
    pub use messages::{InternalServerMessage, MessageDestination, ClientActorMessage};

    #[derive(Default, Debug)]
    pub struct WebsocketData {
        pub chat_storage: Chats,
        pub game_start: TournamentGameStart,
        pub pings: Pings,
        pub lags: Lags,
    }

}}
