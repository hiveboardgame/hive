use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscordHandleStatus {
    Linked(String),
    NotLinked,
    NotLoggedIn,
    Unavailable,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

use actix_web::{get, web::{self, Redirect}, Responder};

#[derive(Deserialize)]
struct OAuthParams {
    code: String,
    state: String,
}
#[get("/oauth/callback")]
pub async fn callback(params: web::Query<OAuthParams>) -> impl Responder {
    let url = format!("http://localhost:8080/oauth/callback?code={}&state={}", params.code, params.state);
    match reqwest::Client::builder().build() {
        Ok(client) => {
            if let Err(e) = client.post(url).send().await {
                println!("Error in oauth callback: {e}");
            };
        }
        Err(e) => println!("Error creating oauth callback client: {e}"),
    }

    Redirect::to("/notifications").temporary()
}
}}

#[server]
pub async fn get_discord_handle() -> Result<DiscordHandleStatus, ServerFnError> {
    use crate::functions::auth::identity::uuid;

    use serde_json::Value;

    if let Ok(uuid) = uuid().await {
        let url = format!("http://localhost:8080/discord/{uuid}");
        let client = match reqwest::Client::builder().build() {
            Ok(client) => client,
            Err(e) => {
                println!("Error creating discord handle client: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                println!("Error loading discord handle: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => {
                println!("Error reading discord handle response: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        let data: Value = match serde_json::from_str(&body) {
            Ok(data) => data,
            Err(e) => {
                println!("Error parsing discord handle response: {e}");
                return Ok(DiscordHandleStatus::Unavailable);
            }
        };
        if let Some(username) = data.get("username").and_then(Value::as_str) {
            return Ok(DiscordHandleStatus::Linked(username.to_string()));
        }
        return Ok(DiscordHandleStatus::NotLinked);
    }
    Ok(DiscordHandleStatus::NotLoggedIn)
}
