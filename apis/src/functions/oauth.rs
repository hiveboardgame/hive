use leptos::*;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

use actix_web::{get, web::{self, Redirect}, HttpResponse, Responder};
use serde_json::Value;
use serde::Deserialize;

#[derive(Deserialize)]
struct OAuthParams {
    code: String,
    state: String,
}
#[get("/oauth/callback")]
pub async fn callback(params: web::Query<OAuthParams>) -> impl Responder {
    let url = format!("http://localhost:8080/oauth/callback?code={}&state={}", params.code, params.state);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .send()
        .await.unwrap();

    Redirect::to("/account").temporary()
}
}}

#[server]
pub async fn get_discord_handle() -> Result<String, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use serde::Deserialize;
    use serde_json::Value;

    if let Ok(uuid) = uuid() {
        let url = format!("http://localhost:8080/discord/{}", uuid);
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let body = response.text().await?;
        let data: Value = serde_json::from_str(&body)?;
        if let Some(username) = data.get("username") {
            let username = username.to_string().replace("\"", "");
            return Ok(username);
        }
        if let Some(detail) = data.get("detail") {
            let detail = detail.to_string().replace("\"", "");
            return Ok(detail.to_string());
        }
    }
    return Ok("Not logged in".to_string());
}
