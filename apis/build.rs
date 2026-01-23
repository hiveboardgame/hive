use leptos_i18n_build::{Config, ParseOptions, TranslationsInfos};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // path to the out directory for leptos_i18n
    let i18n_mod_directory = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("i18n");

    let options = ParseOptions::default()
        .suppress_key_warnings(true)
        .interpolate_display(true);

    let cfg = Config::new("en")?
        .add_locales(["ca", "de", "es", "fr", "hu", "it", "pt", "ro", "ru"])?
        // Commented out very incomplete locales (< 30% translated)
        // .add_locales(["cs", "ja", "nl", "sv"])?
        .add_namespaces([
            "header",
            "home",
            "faq",
            "resources",
            "profile",
            "user_config",
            "game",
            "tournaments",
            "donate",
        ])?
        .parse_options(options);

    let translations_infos = TranslationsInfos::parse(cfg)?;

    // track changes to translations files
    translations_infos.rerun_if_locales_changed();

    // emit errors and warnings
    translations_infos.emit_diagnostics();

    // generate the code, previously done with the `load_locale!` macro
    translations_infos
        .generate_i18n_module(i18n_mod_directory)
        .unwrap();

    Ok(())
}
