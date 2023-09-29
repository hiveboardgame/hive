use crate::components::organisms::header::Header;
use crate::functions::users::get::GetUser;
use leptos::logging::log;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn UserGet() -> impl IntoView {
    let get_user_action = create_server_action::<GetUser>();
    let value = get_user_action.value();
    let username = move || {
        log!("got called");
        match value() {
            Some(Ok(user)) => {
                log!("Got user: {:?}", user);
                user.username
            }
            _ => String::from("None yet"),
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
