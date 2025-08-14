use crate::common::{ClientRequest, ServerMessage};
use futures::channel::mpsc;
use leptos::prelude::*;
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};

type ClientResult = Result<ClientRequest, ServerFnError>;

//client api holds the client mpsc sender 
//and the latest message received from the server
#[derive(Clone)]
pub struct ClientApi {
    sender: StoredValue<mpsc::Sender<ClientResult>>,
    pub latest: RwSignal<Result<ServerMessage, ServerFnError>>,
}
impl ClientApi {
    pub fn new(sender: mpsc::Sender<ClientResult>) -> Self {
        Self {
            sender: StoredValue::new(sender),
            latest: RwSignal::new(Ok(ServerMessage::Error("".into()))),
        }
    }
    pub fn send(&self, client_request: ClientRequest) {
        let mut sender = self.sender.get_value();

        let _ = sender.try_send(Ok(client_request));
    }
}

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
pub async fn websocket_fn(
    input: BoxedStream<ClientRequest, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use futures::{channel::mpsc, SinkExt, StreamExt};

    use crate::functions::auth::identity::uuid;
    use crate::websocket::new_style::ServerNotifications;
    use actix_web::web::Data;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;

    let mut input = input; // FIXME :-) server fn fields should pass mut through to destructure

    let server_notifications = req
        .app_data::<Data<ServerNotifications>>()
        .ok_or("Failed to get server notifications")
        .map_err(ServerFnError::new)?
        .get_ref();
    let mut server_reciever = server_notifications.receiver();
    let server_sender = server_notifications.sender();

    // create a channel of outgoing websocket messages (from mpsc)
    let (mut client_sender, rx) = mpsc::channel(1);
    let mut client_sender2 = client_sender.clone();
    
    //this thread listens to the global notifications and sends them to the client
    leptos::task::spawn(async move {
        loop {
            if let Ok(()) = server_reciever.changed().await {
                let InternalServerMessage {
                    destination,
                    message,
                } = server_reciever.borrow().clone();
                match destination {
                    MessageDestination::Global => {
                        let msg =
                            ServerMessage::Error(format!("Got global notification: {message:?}"));
                        let _ = client_sender2.send(Ok(msg)).await;
                    }
                    MessageDestination::User(id) => {
                        if Ok(id) == uuid().await {
                            let msg =
                                ServerMessage::Error(format!("Got user notification: {message:?}"));
                            let _ = client_sender2.send(Ok(msg)).await;
                        }
                    }
                    _ => {
                        todo!()
                    }
                }
            }
        }
    });
    //this thread recieves client requests and does processing
    leptos::task::spawn(async move {
        while let Some(msg) = input.next().await {
            let msg = match msg {
                Ok(msg) => match msg {
                    ClientRequest::DbgMsg(msg) => {
                        if msg.contains("server") {
                            //since we have a cloned sender, we can send global notifications
                            //in respnse to a client request
                            server_sender
                                .send(InternalServerMessage {
                                    destination: MessageDestination::Global,
                                    message: ServerMessage::Error(msg.clone()),
                                })
                                .unwrap();
                            "Sent global notification".to_string()
                        } else {
                            msg
                        }
                    }
                    _ => "Not implemented".to_string(),
                },
                Err(e) => {
                    format!("Error: {e}")
                }
            };
            println!("In server: {msg:?}");
            let _ = client_sender.send(Ok(ServerMessage::Error(msg))).await;
        }
    });

    Ok(rx.into())
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    //using a watch channel to send messages to all clients (clonable receiver)
    //only retans the last message
    use tokio::sync::watch;
    use crate::websocket::messages::{MessageDestination, InternalServerMessage};

    pub struct ServerNotifications {
        sender: watch::Sender<InternalServerMessage>,
    }
    impl Default for ServerNotifications {
        fn default() -> Self {
            let (sender, _) = watch::channel(InternalServerMessage{
                destination: MessageDestination::Global,
                message: ServerMessage::Error("Server notifications initialized".to_string())
            });
            Self { sender }
        }
    }
    impl ServerNotifications {
        pub fn receiver(&self) -> watch::Receiver<InternalServerMessage> {
            self.sender.subscribe()
        }
        pub fn sender(&self) -> watch::Sender<InternalServerMessage> {
            self.sender.clone()
        }
        pub fn send(&self, message: InternalServerMessage) -> Result<(), ServerFnError> {
            self.sender.send(message)?;
            Ok(())
        }
    }
}}
