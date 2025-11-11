use std::{sync::Arc, vec};

use crate::{
    common::{ClientRequest, ServerMessage},
    websocket::{ServerData, TabData, server_handlers::{
            challenges::handler::ChallengeHandler, chat::handler::ChatHandler, game::handler::GameActionHandler, oauth::handler::OauthHandler, schedules::ScheduleHandler, tournaments::handler::TournamentHandler
        }, server_tasks
    },
};
use futures::StreamExt;
use server_fn::{BoxedStream, ServerFnError};

pub async fn server_handler(
    mut input: BoxedStream<ClientRequest, ServerFnError>,
    tab: &TabData,
    server: Arc<ServerData>,
) {
    server_tasks::send_tournament_invitations(tab, &server).await;
    server_tasks::send_schedules(tab, &server).await;
    server_tasks::send_challenges(tab, &server).await;
    server_tasks::send_urgent_games(tab, &server).await;
    server_tasks::load_online_users(tab, &server);
    while let Some(msg) = input.next().await {
        let messages = async {
            match msg {
                Ok(msg) => match msg {
                    ClientRequest::LinkDiscord => {
                        match tab.account() {
                            Some(account) => OauthHandler::new(account.id).handle().await,
                            None => anyhow::bail!("Anonymous user tried to link discord")
                        }
                    }
                    ClientRequest::Pong(nonce) => {
                        tab.update_pings(nonce);
                        Ok(vec![])
                    }
                    ClientRequest::Game { game_id, action } => {
                        GameActionHandler::new(&game_id, action, tab.clone(), server.clone())
                            .await?
                            .handle()
                            .await
                    }
                    ClientRequest::Challenge(c) => {
                        if tab.account().is_some() {
                            ChallengeHandler::new(c, tab.clone())
                                .await?
                                .handle()
                                .await
                        } else {
                            println!("Anonymous users cant use challenges");
                            Ok(vec![])
                        }
                    }
                    ClientRequest::Schedule(action) => {
                        if !matches!(action, crate::common::ScheduleAction::TournamentPublic(_))
                            && tab.account().is_none()
                        {
                            let err = "Unauthorized user updated schedules";
                            let msg = ServerMessage::Error(err.to_string());
                            println!("{err}");
                            tab.send(msg, &server);
                            Ok(vec![])
                        } else {
                            ScheduleHandler::new(action, tab.clone())
                                .await?
                                .handle()
                                .await
                        }
                    }
                    ClientRequest::Tournament(action) => {
                        TournamentHandler::new(action, tab.clone(), server.clone())
                            .await?
                            .handle()
                            .await
                    }
                    ClientRequest::Chat(container) => {
                        Ok(ChatHandler::new(container, server.clone()).handle())
                    }
                    ClientRequest::Away => {
                        todo!()
                    }
                },
                Err(e) => {
                    let msg = ServerMessage::Error(format!("Error: {e}"));
                    println!("{msg:?}");
                    //tab.send(msg, &server);
                    anyhow::bail!(e)
                }
            }
        };
        match messages.await {
            Ok(messages) => {
                for m in messages {
                    server.send(m).expect("Send internal server message");
                }
            }
            Err(e) => {
                println!("Server Error {e}");
                break;
            }
        }
    }
}
