#[cfg(feature = "ssr")]
pub mod websockets;

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix::Actor;
    use actix_files::Files;
    use actix_identity::IdentityMiddleware;
    use actix_session::{storage::CookieSessionStore, SessionMiddleware};
    use actix_web::{cookie::Key, web, App, HttpServer};
    use apis::app::App;
    use crate::websockets::lobby::Lobby;
    use crate::websockets::start_connection;
    use db_lib::{config::DbConfig, get_pool};
    use diesel::Connection;
    use diesel::pg::PgConnection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use sha2::*;

    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let chat_server = Lobby::default().start();

    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    let config = DbConfig::from_env().expect("Failed to load config from env");
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../db/migrations");
    let database_url = &config.database_url;
    let mut conn = PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let hash: [u8; 64] = Sha512::digest(&config.session_secret)
        .as_slice()
        .try_into()
        .expect("Wrong size");
    let cookie_key = Key::from(&hash);
    let pool = get_pool(&config.database_url)
        .await
        .expect("Failed to get pool");

    println!("listening on http://{}", &addr);

    leptos_query::suppress_query_load(true);
    let routes = generate_route_list(App);
    leptos_query::suppress_query_load(false);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        let routes = &routes;

        App::new()
            .app_data(actix_web::web::Data::new(pool.clone()))
            .app_data(actix_web::web::Data::new(chat_server.clone()))
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            // serve JS/WASM/CSS from `pkg`
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            // serve other assets from the `assets` directory
            .service(Files::new("/assets", site_root))
            // serve the favicon from /favicon.ico
            .service(favicon)
            .service(start_connection::start_connection)
            // .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                || view! { <App/> }
            )
            .app_data(web::Data::new(leptos_options.to_owned()))
            //.wrap(Compress::default())
            // IdentityMiddleware needs to be first
            .wrap(IdentityMiddleware::default())
            // Now SessionMiddleware, this is a bit confusing but actix invokes middlesware in
            // reverse order of registration and the IdentityMiddleware is based on the
            // SessionMiddleware so SessionMiddleware needs to be present
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                cookie_key.clone(),
            ))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(feature = "ssr")]
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

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
    // see optional feature `csr` instead
}




