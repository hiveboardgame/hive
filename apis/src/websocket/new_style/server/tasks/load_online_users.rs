use crate::common::{ServerMessage, UserUpdate};
use crate::websocket::new_style::server::{ServerData, TabData};

pub fn load_online_users(client: &TabData, server_data: &ServerData) {
    println!("Reached load online users");
    for user in server_data.get_online_users() {
        let request = ServerMessage::UserStatus(UserUpdate {
            status: crate::common::UserStatus::Online,
            user,
        });
        client.send(request, server_data);
    }

    if let Some(user) = client.account() {
        server_data.add_user(user.user.clone());
    }
}
