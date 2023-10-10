use crate::functions::accounts::get::GetAccount;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn UserGet() -> impl IntoView {
    let get_user_action = create_server_action::<GetAccount>();
    let value = get_user_action.value();
    let username = move || match value() {
        Some(Ok(user)) => user.username,
        _ => String::from("None yet"),
    };

    view! {
        <ActionForm action=get_user_action>
            <input type="submit" value="Get User"/>
        </ActionForm>
        <p> {username} </p>
    }
}
