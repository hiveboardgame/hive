use crate::lobby::Lobby;

use actix::Actor;

use actix_files::Files;
use actix_web::*;
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

    // Todo fixme
    // let config = DbConfig::from_env().expect("Failed to load config from env");

    let database_url = "postgres://hive-dev@localhost:/hive-local";
    let pool = get_pool(database_url).await.expect("Failed to get pool");

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
        //.wrap(middleware::Compress::default())
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
