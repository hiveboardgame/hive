use crate::organisms::header::Header;
use leptos::*;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
    username: String,
}


#[server(CreateUser, "/api")]
pub async fn create_user(username: String) -> Result<User, ServerFnError> {
    Ok(User { username })
}

#[component]
pub fn UserCreate() -> impl IntoView {
    view! {
        <Header/>
        <button on:click=move |_| {
            // spawn_local(async {
            //     create_user("leex".to_string()).await
            // });
        }>
        "create user"
        </button>
    }
}
