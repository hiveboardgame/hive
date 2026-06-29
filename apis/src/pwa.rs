#[cfg(not(feature = "ssr"))]
mod client;
#[cfg(not(feature = "ssr"))]
pub use client::*;

#[cfg(feature = "ssr")]
mod server;
#[cfg(feature = "ssr")]
pub use server::*;
