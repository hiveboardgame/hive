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
use actix_web::{
    cookie::{time::Duration, SameSite},
    middleware::Compress,
};
use leptos_meta::{HashedStylesheet, MetaTags};
use websocket::WebsocketData;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use crate::websocket::{start_connection, WsServer};
    use api::v1::bot::{games::{api_get_game, api_get_ongoing_games, api_get_pending_games}, play::{api_control, api_play}, challenges::{api_accept_challenge, api_create_challenge, api_get_challenges}};
    use api::v1::auth::get_token_handler::get_token;
    use api::v1::auth::get_identity_handler::get_identity;
    use api::v1::chat::send::send_chat;
    use api::v1::chat::history::get_channel_history;
    use api::v1::users::blocks::{add_block, list_blocks, remove_block};
    use api::v1::users::tournament_chat_mutes::{add_mute, remove_mute};
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
            .service(send_chat)
            .service(get_channel_history)
            .service(add_block)
            .service(remove_block)
            .service(list_blocks)
            .service(add_mute)
            .service(remove_mute)
            .service(api_play)
            .service(api_control)
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
            .wrap({
                let mut session_builder = SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    cookie_key.clone()
                )
                .session_lifecycle(PersistentSession::default().session_ttl(Duration::weeks(12)));
                if cfg!(debug_assertions) {
                    // Development mode: allow HTTP and cross-origin for testing from different IPs
                    session_builder = session_builder
                        .cookie_secure(false)
                        .cookie_same_site(SameSite::Lax);
                }

                session_builder.build()
            })
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

#[cfg(test)]
mod tests {
    use actix_http::Request;
    use actix_web::{dev::ServiceResponse, test, web::Data, App};
    use crate::api::v1::chat::history::get_channel_history;
    use crate::api::v1::chat::send::send_chat;
    use crate::api::v1::users::blocks::{add_block, list_blocks, remove_block};
    use crate::api::v1::users::tournament_chat_mutes::{add_mute, remove_mute};

    /// Build minimal app with chat routes. Requires DATABASE_URL; test is skipped if unset.
    async fn init_chat_app() -> Option<(impl actix_web::dev::Service<
        Request,
        Response = ServiceResponse,
        Error = actix_web::Error,
    >,)> {
        let database_url = std::env::var("DATABASE_URL").ok()?;
        let pool = db_lib::get_pool(&database_url).await.ok()?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(send_chat)
                .service(get_channel_history)
                .service(add_block)
                .service(remove_block)
                .service(list_blocks)
                .service(add_mute)
                .service(remove_mute),
        )
        .await;
        Some((app,))
    }

    #[actix_web::test]
    async fn test_send_chat_returns_401_without_auth() {
        let Some((app,)) = init_chat_app().await else {
            return; // skip when DATABASE_URL not set
        };
        let req = test::TestRequest::post()
            .uri("/api/v1/chat/send")
            .set_payload(r#"{"channel_type":"global","channel_id":"global","body":"hi"}"#)
            .insert_header((actix_web::http::header::CONTENT_TYPE, "application/json"))
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error(), "expected 4xx");
        assert_eq!(resp.status().as_u16(), 401, "expected 401 Unauthorized");
    }

    #[actix_web::test]
    async fn test_get_channel_returns_401_without_auth() {
        let Some((app,)) = init_chat_app().await else {
            return; // skip when DATABASE_URL not set
        };
        let req = test::TestRequest::get()
            .uri("/api/v1/chat/channel?channel_type=global&channel_id=global")
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error(), "expected 4xx");
        assert_eq!(resp.status().as_u16(), 401, "expected 401 Unauthorized");
    }

    #[actix_web::test]
    async fn test_blocks_api_returns_401_without_auth() {
        let Some((app,)) = init_chat_app().await else {
            return;
        };
        let req = test::TestRequest::post()
            .uri("/api/v1/users/me/blocks")
            .set_json(&serde_json::json!({ "user_id": "00000000-0000-0000-0000-000000000001" }))
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 401, "POST /users/me/blocks must require auth");

        let req = test::TestRequest::delete()
            .uri("/api/v1/users/me/blocks/00000000-0000-0000-0000-000000000001")
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 401, "DELETE /users/me/blocks must require auth");

        let req = test::TestRequest::get().uri("/api/v1/users/me/blocks").to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 401, "GET /users/me/blocks must require auth");
    }

    #[actix_web::test]
    async fn test_tournament_chat_mutes_api_returns_401_without_auth() {
        let Some((app,)) = init_chat_app().await else {
            return;
        };
        let req = test::TestRequest::post()
            .uri("/api/v1/users/me/tournament-chat-mutes")
            .set_json(&serde_json::json!({ "tournament_id": "some-nanoid" }))
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 401, "POST tournament-chat-mutes must require auth");

        let req = test::TestRequest::delete()
            .uri("/api/v1/users/me/tournament-chat-mutes/some-nanoid")
            .to_request();
        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 401, "DELETE tournament-chat-mutes must require auth");
    }
}

}}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // trunk stuff
}
