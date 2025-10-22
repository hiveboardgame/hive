use crate::{
    common::{ClientRequest, ServerMessage},
    functions::accounts::get::get_account,
};
use futures::channel::mpsc;
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use crate::functions::db::pool;
    use crate::websocket::{
        new_style::server::{server_handler, tasks, ServerData, TabData}
    };
    use actix_web::web::Data;
    use tokio::{join, spawn,select};
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server_o = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone()
        .into_inner();
    let user = get_account().await.ok();

    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);

    let tab_o = TabData::new(tx, user, pool().await?);

    //One shot tasks
    let (tab, server) = (tab_o.clone(),server_o.clone());
    spawn(async move {
        let server = server.as_ref();
        join!(
            tasks::send_tournament_invitations(&tab, server),
            tasks::send_schedules(&tab, server),
            tasks::send_challenges(&tab, server),
            tasks::send_urgent_games(&tab, server),
        );
    });

    /* === Long runing tasks with abortable handle === */

    let (tab, server) = (tab_o.clone(),server_o.clone());
    let token = tab_o.token();
    spawn( async move { 
        select!(
            //pings the client on a given interval
            _ = tasks::ping_client(&tab, server.as_ref()) =>{},
            _ = token.cancelled() => {}

        );
    });

    let token = tab_o.token();
    spawn(async move { 
        select!(
            //listens to the server notifications and sends them to the client
            _ = tasks::server_notifications(&tab_o, &server_o) =>{},
            //main handler
            _ = server_handler(input, &tab_o, server_o.clone()) => {},
            _ = token.cancelled() => {}

        );
    });
    Ok(rx.into())
}
