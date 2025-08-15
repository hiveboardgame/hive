mod websocket_fn;
use websocket_fn::websocket_fn;
mod client;
pub use client::client_handler;
pub use client::ClientApi;
cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    mod server_types;
    mod server_fns;
    pub use server_types::ServerNotifications;
}}
