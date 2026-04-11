#[cfg(feature = "ssr")]
pub mod access;
pub mod simple_destination;

pub use shared_types::ChannelKey;
pub use simple_destination::SimpleDestination;
