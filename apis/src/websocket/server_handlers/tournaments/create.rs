use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination, SocketTx},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use db_lib::{
    get_conn,
    helpers::run_serializable,
    models::{NewTournament, Tournament},
    DbPool,
};
use shared_types::{tournament::Format, TournamentDetails, TournamentId};
use std::sync::Arc;
use uuid::Uuid;

pub struct CreateHandler {
    details: TournamentDetails,
    user_id: Uuid,
    received_from: SocketTx,
    pool: DbPool,
}

impl CreateHandler {
    pub fn new(
        details: TournamentDetails,
        user_id: Uuid,
        received_from: SocketTx,
        pool: &DbPool,
    ) -> Self {
        Self {
            details,
            user_id,
            received_from,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let new_tournament = NewTournament::new(self.details.clone())?;
        let new_tournament = Arc::new(new_tournament);
        let user_id = self.user_id;
        let mut conn = get_conn(&self.pool).await?;
        let tournament = run_serializable(&mut conn, move |tc| {
            let new_tournament = Arc::clone(&new_tournament);
            Box::pin(async move { Tournament::create(user_id, &new_tournament, tc).await })
        })
        .await?;

        let format = tournament.configuration().format();
        let tournament_id = TournamentId(tournament.nanoid);
        Ok(creation_messages(
            tournament_id,
            format,
            tournament.starts_at,
            self.received_from.clone(),
        ))
    }
}

fn creation_messages(
    tournament_id: TournamentId,
    format: Format,
    starts_at: Option<DateTime<Utc>>,
    received_from: SocketTx,
) -> Vec<InternalServerMessage> {
    vec![
        InternalServerMessage {
            destination: MessageDestination::Direct(received_from),
            message: ServerMessage::Tournament(TournamentUpdate::CreationSucceeded(
                tournament_id.clone(),
            )),
        },
        InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Created {
                tournament_id,
                format,
                starts_at,
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::creation_messages;
    use crate::{
        common::{ServerMessage, ServerResult, TournamentUpdate},
        websocket::messages::{MessageDestination, SocketFormat, SocketTx},
    };
    use chrono::DateTime;
    use codee::{binary::MsgpackSerdeCodec, Decoder, Encoder};
    use shared_types::{tournament::Format, TournamentId};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[test]
    fn creation_broadcasts_discovery_metadata_and_acknowledges_only_the_submitting_socket() {
        let tournament_id = TournamentId(String::from("public-nanoid"));
        let (tx, _rx) = mpsc::channel(2);
        let socket = SocketTx {
            socket_id: Uuid::from_u128(1),
            format: SocketFormat::Msgpack,
            tx,
        };
        for format in [
            Format::RoundRobin,
            Format::Swiss,
            Format::DoubleSwiss,
            Format::SingleElimination,
            Format::DoubleElimination,
            Format::Arena,
        ] {
            for starts_at in [None, DateTime::from_timestamp(1_800_000_000, 0)] {
                let messages =
                    creation_messages(tournament_id.clone(), format, starts_at, socket.clone());
                assert_eq!(messages.len(), 2);
                assert!(matches!(
                    &messages[0].destination,
                    MessageDestination::Direct(destination)
                        if destination.socket_id == socket.socket_id
                            && destination.tx.same_channel(&socket.tx)
                ));
                assert!(matches!(
                    &messages[0].message,
                    ServerMessage::Tournament(TournamentUpdate::CreationSucceeded(id))
                        if id == &tournament_id
                ));
                assert!(matches!(
                    messages[1].destination,
                    MessageDestination::Global
                ));
                let wire = MsgpackSerdeCodec::encode(&ServerResult::Ok(Box::new(
                    messages[1].message.clone(),
                )))
                .expect("encode public creation");
                let decoded: ServerResult =
                    MsgpackSerdeCodec::decode(&wire).expect("decode public creation");
                assert!(matches!(
                    decoded,
                    ServerResult::Ok(message) if matches!(
                        message.as_ref(),
                        ServerMessage::Tournament(TournamentUpdate::Created {
                            tournament_id: id,
                            format: received_format,
                            starts_at: received_starts_at,
                        }) if id == &tournament_id
                            && *received_format == format
                            && *received_starts_at == starts_at
                    )
                ));
            }
        }
    }
}
