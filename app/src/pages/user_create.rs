use crate::organisms::header::Header;
use leptos::*;
use leptos_router::ActionForm;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub id: String,
}

#[server(CreateUser)]
pub async fn create_user(username: String) -> Result<User, ServerFnError> {
    use actix_web::http::header::{HeaderMap, HeaderValue, SET_COOKIE};
    use leptos_actix::{ResponseOptions, ResponseParts};

    let response =
        use_context::<ResponseOptions>().expect("to have leptos_actix::ResponseOptions provided");
    let mut response_parts = ResponseParts::default();
    let mut headers = HeaderMap::new();
    let user_id = Uuid::new_v4().to_string();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!("user_id={user_id}; Path=/"))
            .expect("to create header value"),
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&format!("username={username}; Path=/"))
            .expect("to create header value"),
    );
    response_parts.headers = headers;

    response.overwrite(response_parts);
    Ok(User {
        username,
        id: user_id,
    })
}

#[cfg(not(feature = "ssr"))]
fn username_from_cookie() -> Option<String> {
    use cookie::*;
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    for cookie in Cookie::split_parse(doc.cookie().unwrap_or_default()) {
        if let Ok(cookie) = cookie {
            match cookie.name() {
                "username" => return Some(cookie.value().to_string()),
                _ => {}
            }
        }
    }
    None
}

#[cfg(feature = "ssr")]
fn username_from_cookie() -> Option<String> {
    use_context::<actix_web::HttpRequest>()
        .and_then(|req| {
            req.cookies()
                .map(|cookies| {
                    cookies.iter().find_map(|cookie| {
                        if cookie.name() == "username" {
                            Some(cookie.value().to_string())
                        } else {
                            None
                        }
                    })
                })
                .ok()
        })
        .unwrap_or(None)
}

#[component]
pub fn UserCreate() -> impl IntoView {
    let maybe_username = username_from_cookie();
    let get_user = create_server_action::<CreateUser>();
    // holds the latest *returned* value from the server
    let value = get_user.value();
    // check if the server has returned an error
    // let has_error = move || value.with(|val| matches!(val, Some(Err(_))));

    let username = move || {
        match value() {
            Some(Ok(user)) => user.username,
            // otherwise, use the initial value
            _ => maybe_username.to_owned().unwrap_or_default(),
        }
    };

    view! {
        <Header/>
        <ActionForm action=get_user>
            <label>
                "Create a user"
                // `title` matches the `title` argument to `add_todo`
                <input type="text" name="username"/>
            </label>
            <input type="submit" value="Create"/>
        </ActionForm>
        <p> { username } </p>
    }
}
