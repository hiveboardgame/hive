use leptos::prelude::*;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

use actix_web::{get, web::{self, Redirect}, Responder};
use serde::Deserialize;

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

    Redirect::to("/account").temporary()
}
}}

#[server]
pub async fn get_discord_handle() -> Result<String, ServerFnError> {
    use crate::functions::auth::identity::uuid;

    use serde_json::Value;

    if let Ok(uuid) = uuid().await {
        let url = format!("http://localhost:8080/discord/{uuid}");
        let client = match reqwest::Client::builder().build() {
            Ok(client) => client,
            Err(e) => {
                println!("Error creating discord handle client: {e}");
                return Ok("Not logged in".to_string());
            }
        };
        let response = match client.get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                println!("Error loading discord handle: {e}");
                return Ok("Not logged in".to_string());
            }
        };
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => {
                println!("Error reading discord handle response: {e}");
                return Ok("Not logged in".to_string());
            }
        };
        let data: Value = match serde_json::from_str(&body) {
            Ok(data) => data,
            Err(e) => {
                println!("Error parsing discord handle response: {e}");
                return Ok("Not logged in".to_string());
            }
        };
        if let Some(username) = data.get("username") {
            let username = username.to_string().replace("\"", "");
            return Ok(username);
        }
        if let Some(detail) = data.get("detail") {
            let detail = detail.to_string().replace("\"", "");
            return Ok(detail.to_string());
        }
    }
    Ok("Not logged in".to_string())
}
