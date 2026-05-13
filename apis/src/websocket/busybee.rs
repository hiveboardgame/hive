use std::{collections::HashMap, sync::OnceLock};
use uuid::Uuid;

pub struct Busybee {}

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

impl Busybee {
    fn client() -> Result<&'static reqwest::Client, reqwest::Error> {
        if let Some(client) = CLIENT.get() {
            return Ok(client);
        }

        let client = reqwest::Client::builder().build()?;
        Ok(CLIENT.get_or_init(|| client))
    }

    pub async fn msg(to: Uuid, msg: String) -> Result<reqwest::Response, reqwest::Error> {
        let url = format!("http://localhost:8080/msg/{to}");
        let mut json = HashMap::new();
        json.insert("content", msg);
        let client = Self::client()?;
        client.post(url).json(&json).send().await
    }
}
