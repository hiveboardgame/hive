mod chat;
mod lag_tracking;
pub use chat::Chats;
pub use lag_tracking::{Lags, Pings};
pub mod busybee;
pub mod client_handlers;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    mod messages;
    mod start_conn;
    mod telemetry;
    mod tournament_game_start;
    mod ws_connection;
    mod ws_server;
    pub mod server_handlers;

    pub use start_conn::start_connection;
    pub use telemetry::{diff_and_format, DestKind, DisconnectReason, SendOutcome, TelemetrySnapshot, WsTelemetry};
    pub use tournament_game_start::TournamentGameStart;
    pub use ws_server::WsServer;
    pub use messages::{GameHB, Ping, InternalServerMessage, MessageDestination, ClientActorMessage};

    #[derive(Debug)]
    pub struct WebsocketData {
        pub chat_storage: Chats,
        pub game_start: TournamentGameStart,
        pub pings: Pings,
        pub lags: Lags,
        pub telemetry: std::sync::Arc<WsTelemetry>,
    }

    impl Default for WebsocketData {
        fn default() -> Self {
            Self {
                chat_storage: Chats::default(),
                game_start: TournamentGameStart::default(),
                pings: Pings::default(),
                lags: Lags::default(),
                telemetry: std::sync::Arc::new(WsTelemetry::default()),
            }
        }
    }

}}
