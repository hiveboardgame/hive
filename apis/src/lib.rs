pub mod app;
pub mod common;
pub mod components;
pub mod functions;
pub mod pages;
pub mod providers;
pub mod pwa_cache;
pub mod responses;
pub mod websocket;
leptos_i18n::load_locales!();

use cfg_if::cfg_if;

cfg_if! {
if #[cfg(feature = "hydrate")] {

  use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    pub fn hydrate() {
      use app::*;

      console_error_panic_hook::set_once();

      leptos::mount_to_body(App);
    }
}
}
