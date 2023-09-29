use crate::lobby::Lobby;
use actix::Actor;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, web, App, HttpServer};
use actix_web::middleware::Compress;
use app::*;
use db_lib::get_pool;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

mod lobby;
mod messages;
mod start_connection;
mod ws;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(|| view! { <App/> });
    let chat_server = Lobby::default().start();

    // TODO FIXME
    // let config = DbConfig::from_env().expect("Failed to load config from env");
    // let session_secret = Key::from(config.session_secret.as_bytes());
    // let secret_key = Key::from(session_secret.as_bytes());
    let database_url = "postgres://hive-dev@localhost:/hive-local";
    let pool = get_pool(database_url).await.expect("Failed to get pool");
    let key = Key::generate();

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

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
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                || view! { <App/> },
            )
            .app_data(web::Data::new(leptos_options.to_owned()))
            .wrap(Compress::default())
            // IdentityMiddleware needs to be first
            .wrap(IdentityMiddleware::default())
            // Now SessionMiddleware, this is a bit confusing but actix invokes middlesware in
            // reverse order of registration and the IdentityMiddleware is based on the
            // SessionMiddleware so SessionMiddleware needs to be present
            .wrap(SessionMiddleware::new(CookieSessionStore::default(), key.clone()))
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
