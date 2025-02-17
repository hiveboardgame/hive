use serde::Deserialize;
use leptos::*;

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {

use actix_web::{get, web::{self, Redirect}, HttpResponse, Responder};
use serde_json::Value;

#[derive(Deserialize)]
struct OAuthParams {
    code: String,
    state: String,
}
#[get("/oauth/callback")]
///http://localhost:3000/oauth/callback?code=cdOaUkwh3bW9QosMRJxjHbEND8QXsq&state=t01q4LKAPcOEAMFWs_Y9AzOqYRMIyElXMkSIqYLqMvctX6vVZ_n6ad2HrG25Xy0Fj7UG0vrtVRWFm8JaTDWFzWcxQ
pub async fn callback(params: web::Query<OAuthParams>) -> impl Responder {
    println!("code: {:?} state: {:?}", params.code, params.state);
    let url = format!("http://localhost:8080/oauth/callback?code={}&state={}", params.code, params.state);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .send()
        .await.unwrap();

    let json_str = response.text().await.unwrap();
    println!("Body is: {}", json_str);

    Redirect::to("/discord").temporary()
}
}}

#[derive(Deserialize)]
struct DiscordParams {
    discord_id: String,
    avatar_url: String,
    username: String,
}

#[server]
pub async fn get_discord_handle() -> Result<String, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use serde::Deserialize;
    use serde_json::Value;

    if let Ok(uuid) = uuid() {
        println!("Trying to get discord handle for: {}", uuid);
        let url = format!("http://localhost:8080/discord/{}", uuid);
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let data: DiscordParams = response.text().await?;
        return Ok(data.username);
    }
    return Ok("Not logged in".to_string());
}
