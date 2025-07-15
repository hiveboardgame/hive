pub mod app;
pub mod common;
pub mod components;
pub mod functions;
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

      leptos::mount::hydrate_body(App);
    }
}
}
