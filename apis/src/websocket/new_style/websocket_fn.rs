use crate::common::{ClientRequest, ServerMessage};
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

pub const WS_BUFFER_SIZE: usize = 16;
#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use actix_web::web::Data;
    use tokio::{spawn,select};
    use futures::channel::mpsc;
    use crate::functions::{
        db::pool,
        accounts::get::get_account
    };
    use crate::websocket::{
        new_style::server::{server_handler, tasks, ServerData, TabData},
    };
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone()
        .into_inner();
    let user = get_account().await.ok();

    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(WS_BUFFER_SIZE);
    let tab = TabData::new(tx, user, pool().await?);
    
     /* === Long runing tasks with abortable handle === */

    let token = tab.token();
    let tab2 = tab.clone();
    let server2 = server.clone();
    spawn(async move { 
        select!(
            _ = token.cancelled() => {}
            //main handler
            _ = server_handler(input, &tab2, server2) => {},
        );
    });
    let token = tab.token();
    //server notifications stream
    let notifications = server.notifications();
    spawn(async move { 
        select!(
            _ = token.cancelled() => {}
            //subscribe to server notifications
            _ = tasks::server_notifications(&tab, &server, notifications) =>{},
            //one shot tasks then ping client on a loop
            _ = tasks::ping_client(&tab, &server) =>{},

        );
    });
    Ok(rx.into())
}
