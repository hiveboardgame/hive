//! HOP (Hive Open Position): a coordinate-free encoding of a Hive position. <https://pepeke.app/help/hop>

mod error;
mod frame;
mod parse;
mod serialize;
#[cfg(test)]
mod tests;

pub use error::HopError;
pub use parse::{parse, to_hash, HopPosition};
pub use serialize::{canonicalize, from_position};
