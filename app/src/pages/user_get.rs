use crate::organisms::header::Header;
use crate::pages::user_create::User as LeptosUser;
use leptos_router::ActionForm;
use leptos::*;

#[server(GetUser)]
pub async fn get_user() -> Result<LeptosUser, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool().expect("Failed to get pool");

    use db_lib::models::user::User;
    let user = User::find_by_uid("unique", &pool)
        .await
        .expect("Couldn't get user");
    Ok(LeptosUser {
        username: user.username,
        id: user.uid,
    })
}

#[component]
pub fn UserGet() -> impl IntoView {
    let get_user_action = create_server_action::<GetUser>();
    let value = get_user_action.value();
    let username = move || {
        match value() {
            Some(Ok(user)) => user.username,
            _ => String::new(),
        }
    };

    view! {
        <Header/>
        <ActionForm action=get_user_action>
            <input type="submit" value="Get User"/>
        </ActionForm>
        <p> {username} </p>
    }
}
