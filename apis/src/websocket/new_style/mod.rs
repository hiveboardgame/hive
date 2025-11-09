mod websocket_fn;
pub use websocket_fn::{WS_BUFFER_SIZE,websocket_fn};
cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    pub mod server;
}}
