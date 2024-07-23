pub mod app;
pub mod common;
pub mod components;
pub mod functions;
pub mod lag_tracking;
pub mod pages;
pub mod ping;
pub mod providers;
pub mod responses;

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
