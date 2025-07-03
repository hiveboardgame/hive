use leptos_i18n_build::{Options, TranslationsInfos};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // path to the out directory for leptos_i18n
    let i18n_mod_directory = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("i18n");

    let options = Options::default()
        .suppress_key_warnings(true)
        .interpolate_display(true);

    let translations_infos = TranslationsInfos::parse(options).unwrap();

    // track changes to translations files
    translations_infos.rerun_if_locales_changed();

    // generate the code, previously done with the `load_locale!` macro
    translations_infos
        .generate_i18n_module(i18n_mod_directory)
        .unwrap();
} 