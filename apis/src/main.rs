#![recursion_limit = "256"]
pub mod api;
pub mod common;
pub mod functions;
pub mod jobs;
pub mod providers;
pub mod responses;
pub mod websocket;
use std::sync::Arc;

use actix_session::config::PersistentSession;
use actix_web::cookie::time::Duration;
use actix_web::middleware::Compress;
use leptos_meta::{HashedStylesheet, MetaTags};
use websocket::WebsocketData;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use crate::websocket::{start_connection, WsServer};
    use api::v1::bot::{games::{api_get_game, api_get_ongoing_games, api_get_pending_games}, play::api_play, challenges::{api_accept_challenge, api_create_challenge, api_get_challenges}};
    use api::v1::auth::get_token_handler::get_token;
    use api::v1::auth::get_identity_handler::get_identity;
    use api::v1::auth::jwt_secret::JwtSecret;
    use api::v1::bot::users::api_get_user;
    use actix::Actor;
    use actix_files::Files;
    use actix_identity::IdentityMiddleware;
    use actix_session::{storage::CookieSessionStore, SessionMiddleware};
    use actix_web::{cookie::Key, web::Data, App, HttpServer};
    use apis::app::App;
    use db_lib::{config::DbConfig, get_pool};
    use diesel::pg::PgConnection;
    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use sha2::*;

    let conf = get_configuration(None).expect("Got configuration");
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);

    simple_logger::init_with_level(log::Level::Warn).expect("couldn't initialize logging");

    let config = DbConfig::from_env().expect("Failed to load config from env");
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../db/migrations");
    let database_url = &config.database_url;
    let mut conn = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}"));
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Ran migrations");

    let hash: [u8; 64] = Sha512::digest(&config.session_secret)
        .as_slice()
        .try_into()
        .expect("Wrong size");
    let cookie_key = Key::from(&hash);
    let pool = get_pool(&config.database_url)
        .await
        .expect("Failed to get pool");
    let data = Data::new(WebsocketData::default());
    let websocket_server = Data::new(WsServer::new(Arc::clone(&data), pool.clone()).start());
    let jwt_secret = JwtSecret::new(config.jwt_secret);
    let jwt_key = Data::new(jwt_secret);

    jobs::tournament_start(pool.clone(), Data::clone(&websocket_server));
    jobs::heartbeat(Data::clone(&websocket_server));
    jobs::ping(Data::clone(&websocket_server));
    jobs::game_cleanup(pool.clone());
    jobs::challenge_cleanup(pool.clone());

    // Run game statistics collection once at startup
    if let Err(e) = jobs::game_stats(pool.clone()).await {
        eprintln!("Failed to run game statistics job: {}", e);
    }

    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::clone(&websocket_server))
            .app_data(Data::clone(&data))
            .app_data(Data::clone(&jwt_key))
            .app_data(Data::new(site_root.to_string()))
            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", site_root.as_ref()))
            // serve the favicon from /favicon.ico
            .service(favicon)
            .service(start_connection)
            .service(functions::pwa::cache)
            .service(functions::oauth::callback)
            .service(get_token)
            .service(get_identity)
            .service(api_play)
            .service(api_get_game)
            .service(api_get_ongoing_games)
            .service(api_get_pending_games)
            .service(api_get_user)
            .service(api_get_challenges)
            .service(api_accept_challenge)
            .service(api_create_challenge)

            // .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .leptos_routes(routes.to_owned(), {
                let leptos_options = leptos_options.clone();
                move || {
                    use leptos::prelude::*;

                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                                <HashedStylesheet options=leptos_options.clone() id="leptos"/>
                            </head>
                            <body>
                                <App/>
                            </body>
                        </html>
                    }
            }})
            .app_data(Data::new(leptos_options.to_owned()))
            // IdentityMiddleware needs to be first
            .wrap(IdentityMiddleware::default())
            // Now SessionMiddleware, this is a bit confusing but actix invokes middlesware in
            // reverse order of registration and the IdentityMiddleware is based on the
            // SessionMiddleware so SessionMiddleware needs to be present
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), cookie_key.clone())
                    .session_lifecycle(PersistentSession::default().session_ttl(Duration::weeks(1)))
                    .build(),
            )
            .wrap(Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[actix_web::get("favicon.ico")]
async fn favicon(
    leptos_options: actix_web::web::Data<leptos::prelude::LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}

}}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // trunk stuff
}
