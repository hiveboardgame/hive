use crate::common::{ClientRequest, ServerMessage};
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use actix_web::web::Data;
    use tokio::{join, spawn,select};
    use futures::channel::mpsc;
    use futures::StreamExt;
    use crate::functions::{
        db::pool,
        accounts::get::get_account
    };
    use crate::websocket::{
        new_style::server::{server_handler, tasks, ServerData, TabData},
        server_handlers::{
            game::handler::GameActionHandler, 
            challenges::handler::ChallengeHandler,
            schedules::ScheduleHandler,
            tournaments::handler::TournamentHandler,
        }
    };
    let mut input = input;
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let server = req
        .app_data::<Data<ServerData>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .clone()
        .into_inner();
    let user = get_account().await.ok();

    // create a channel of outgoing websocket messages (from mpsc)
    let (tx, rx) = mpsc::channel(1);
    let tab = TabData::new(tx, user, pool().await?);

    //server notifications stream
    let notifications = server.notifications();

    tasks::send_tournament_invitations(&tab, &server).await;
    tasks::send_schedules(&tab, &server).await;
    tasks::send_challenges(&tab, &server).await;
    tasks::send_urgent_games(&tab, &server).await;
    /* === Long runing tasks with abortable handle === */

    let token = tab.token();
    spawn(async move { 
        select!(
            _ = token.cancelled() => {}
            //main handler
            _ = server_handler(input, &tab, server.clone()) => {},
            //subscribe to server notifications
            _ = tasks::server_notifications(&tab, &server, notifications) =>{},
            //one shot tasks then ping client on a loop
            _ = tasks::ping_client(&tab, &server) =>{},

        );
    });
    Ok(rx.into())
}
