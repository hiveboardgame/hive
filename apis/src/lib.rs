// SSR-only modules. Mirror what main.rs declares for the binary so that
// ssr-gated lib-side code (server function bodies, websocket handlers,
// identity helpers) can reach `crate::api::v1::*`.
#[cfg(feature = "ssr")]
pub mod api;
pub mod app;
pub mod client;
pub mod common;
pub mod components;
pub mod functions;
#[cfg(feature = "ssr")]
pub mod notifications;
pub mod pages;
pub mod providers;
pub mod responses;
pub mod websocket;
// leptos_i18n::load_locales!();
include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));

use cfg_if::cfg_if;

cfg_if! {
if #[cfg(feature = "hydrate")] {

  use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    pub fn hydrate() {
      use app::*;

      console_error_panic_hook::set_once();
      // Rehydrate any stored bearer token before server functions fire. Empty
      // on first run, populated after the user logs in.
      client::load_token_from_storage();

      leptos::mount::hydrate_body(App);
    }
}
}

cfg_if! {
if #[cfg(feature = "csr")] {

  use wasm_bindgen::prelude::wasm_bindgen;

    /// CSR-only entry point used by the HiveGame mobile shell (culex) and any
    /// other bundled-WASM consumer. Points server functions at the backend URL
    /// chosen at build time via `LEPTOS_SERVER_URL` (default: local dev
    /// server). For prod HiveGame mobile bundles, build with
    /// `LEPTOS_SERVER_URL=https://hivegame.com trunk build`.
    /// SSR/hydrate paths use the `hydrate()` fn above instead.
    #[wasm_bindgen(start)]
    pub fn main() {
      use app::*;

      const SERVER_URL: &str = match option_env!("LEPTOS_SERVER_URL") {
        Some(url) => url,
        None => "http://localhost:3000",
      };

      console_error_panic_hook::set_once();
      server_fn::client::set_server_url(SERVER_URL);
      // Rehydrate any stored bearer token before server functions fire.
      client::load_token_from_storage();
      leptos::mount::mount_to_body(App);
    }
}
}
