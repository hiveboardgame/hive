pub mod api;
pub mod app;
#[cfg(feature = "ssr")]
pub mod chat;
pub mod common;
pub mod components;
#[cfg(feature = "ssr")]
pub mod email;
pub mod functions;
pub mod hiveground;
pub mod hooks;
#[cfg(feature = "ssr")]
pub mod jobs;
#[cfg(feature = "ssr")]
pub mod notifications;
pub mod pages;
pub mod providers;
pub mod pwa;
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

#[cfg(test)]
mod locale_tests {
    use std::collections::BTreeSet;

    /// The locales compiled into the binary, per `apis/build.rs`. `cs`, `ja`, `nl` and `sv` have
    /// directories but are commented out there as too incomplete, so they are not checked.
    const ACTIVE_LOCALES: [&str; 9] = ["ca", "de", "es", "fr", "hu", "it", "pt", "ro", "ru"];

    const NAMESPACES: [&str; 12] = [
        "header",
        "home",
        "faq",
        "resources",
        "profile",
        "user_config",
        "game",
        "tournaments",
        "donate",
        "archive",
        "notifications",
        "messages",
    ];

    fn keys(locale: &str, namespace: &str) -> BTreeSet<String> {
        let path = format!(
            "{}/locales/{locale}/{namespace}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        value
            .as_object()
            .expect("flat object")
            .keys()
            .cloned()
            .collect()
    }

    /// `suppress_key_warnings(true)` makes a missing key silently fall back to English - this
    /// test is the only thing that notices an untranslated `archive` key.
    #[test]
    fn archive_namespace_is_translated_everywhere() {
        let english = keys("en", "archive");
        let mut gaps = Vec::new();
        for locale in ACTIVE_LOCALES {
            let missing: Vec<String> = english
                .difference(&keys(locale, "archive"))
                .cloned()
                .collect();
            if !missing.is_empty() {
                gaps.push(format!("{locale}: {}", missing.join(", ")));
            }
        }
        assert!(
            gaps.is_empty(),
            "untranslated archive keys:\n{}",
            gaps.join("\n")
        );
    }

    /// The other namespaces are not complete and this does not pretend otherwise - it reports the
    /// backlog so it stays visible rather than being rediscovered by a user seeing English.
    #[test]
    fn report_translation_backlog() {
        let mut backlog: Vec<(usize, &str)> = NAMESPACES
            .iter()
            .map(|namespace| {
                let english = keys("en", namespace);
                let missing = ACTIVE_LOCALES
                    .iter()
                    .map(|locale| english.difference(&keys(locale, namespace)).count())
                    .sum();
                (missing, *namespace)
            })
            .filter(|(missing, _)| *missing > 0)
            .collect();
        backlog.sort_by_key(|&(missing, _)| std::cmp::Reverse(missing));
        println!("missing keys across the 9 active locales: {backlog:?}");
    }
}
