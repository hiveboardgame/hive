pub mod common;
pub mod functions;
pub mod jobs;
pub mod providers;
pub mod pwa_cache;
pub mod responses;
pub mod websocket;
use actix_session::config::PersistentSession;
use actix_web::cookie::time::Duration;
use actix_web::middleware::Compress;
use websocket::{Lags, Pings, TournamentGameStart};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use crate::websocket::{Chats, start_connection, WsServer};
    use crate::pwa_cache::pwa_cache;
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
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use sha2::*;

    let conf = get_configuration(None).await.expect("Got configuration");
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);

    simple_logger::init_with_level(log::Level::Warn).expect("couldn't initialize logging");

    let config = DbConfig::from_env().expect("Failed to load config from env");
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../db/migrations");
    let database_url = &config.database_url;
    let mut conn = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
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
    let chat_history = Data::new(Chats::new());
    let pings = Data::new(Pings::new());
    let lags = Data::new(Lags::new());
    let websocket_server = Data::new(WsServer::new(pings.clone(), pool.clone()).start());
    let tournament_game_start = Data::new(TournamentGameStart::new());

    jobs::tournament_start(pool.clone(), Data::clone(&websocket_server));
    jobs::heartbeat(Data::clone(&websocket_server));
    jobs::ping(Data::clone(&websocket_server));

    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::clone(&chat_history))
            .app_data(Data::clone(&websocket_server))
            .app_data(Data::clone(&tournament_game_start))
            .app_data(Data::clone(&pings))
            .app_data(Data::clone(&lags))
            .app_data(Data::new(site_root.to_string()))

            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", site_root))
            // serve the favicon from /favicon.ico
            .service(favicon)
            .service(start_connection)
            .service(pwa_cache)
            // .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                || view! { <App/> },
            )
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
    leptos_options: actix_web::web::Data<leptos::LeptosOptions>,
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
