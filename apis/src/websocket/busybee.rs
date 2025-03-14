use std::collections::HashMap;
use uuid::Uuid;

pub struct Busybee {}

impl Busybee {
    pub async fn msg(to: Uuid, msg: String) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("http://localhost:8080/msg/{}", to);
        let mut json = HashMap::new();
        json.insert("content", msg);
        let client = reqwest::Client::new();
        client.post(url).json(&json).send().await
    }
}
