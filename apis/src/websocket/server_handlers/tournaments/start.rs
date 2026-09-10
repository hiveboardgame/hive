use super::{
    append_public_tournament_patches,
    load_player_ids_after_commit,
    player_lifecycle_messages,
    PlayerTournamentLifecycle,
    PublicTournamentSection,
};
use crate::{
    common::{ServerMessage, TournamentUpdate},
    notifications::{notify, Event},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination},
        server_handlers::game::new_tournament_game_messages,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Tournament},
    tournaments::fixed_field,
    DbConn,
    DbPool,
};
use shared_types::{tournament::FormatConfig, TournamentId};
use uuid::Uuid;

pub struct StartHandler {
    tournament_id: TournamentId,
    action: StartAction,
    user_id: Uuid,
    pool: DbPool,
}

enum StartAction {
    Begin(Vec<Uuid>),
    Cancel(Uuid),
    Confirm(Uuid, Vec<Uuid>),
}

impl StartHandler {
    pub fn new(
        tournament_id: TournamentId,
        expected_entrant_ids: Vec<Uuid>,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            action: StartAction::Begin(expected_entrant_ids),
            user_id,
            pool: pool.clone(),
        }
    }

    pub fn cancel(
        tournament_id: TournamentId,
        setup_id: Uuid,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            action: StartAction::Cancel(setup_id),
            user_id,
            pool: pool.clone(),
        }
    }

    pub fn confirm(
        tournament_id: TournamentId,
        setup_id: Uuid,
        order: Vec<Uuid>,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            action: StartAction::Confirm(setup_id, order),
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let outcome = match &self.action {
            StartAction::Begin(expected)
                if matches!(
                    tournament.configuration().format,
                    FormatConfig::Elimination(_)
                ) =>
            {
                let prepared = fixed_field::prepare_elimination_start(
                    tournament.id,
                    self.user_id,
                    expected.clone(),
                    &mut conn,
                )
                .await?;
                return Ok(setup_messages(prepared, &mut conn).await);
            }
            StartAction::Cancel(setup_id) => {
                let cancelled = fixed_field::cancel_elimination_start(
                    tournament.id,
                    self.user_id,
                    *setup_id,
                    &mut conn,
                )
                .await?;
                return Ok(setup_messages(cancelled, &mut conn).await);
            }
            StartAction::Confirm(setup_id, order) => {
                fixed_field::confirm_elimination_start(
                    tournament.id,
                    self.user_id,
                    *setup_id,
                    order.clone(),
                    &mut conn,
                )
                .await?
            }
            StartAction::Begin(expected) => {
                fixed_field::start_by_organizer(
                    tournament.id,
                    self.user_id,
                    expected.clone(),
                    &mut conn,
                )
                .await?
            }
        };
        let (tournament, games, removed_invitees) =
            (outcome.tournament, outcome.games, outcome.removed_invitees);
        Ok(start_outcome_messages(tournament, games, removed_invitees, &mut conn).await)
    }
}

pub(crate) async fn setup_messages(
    tournament: Tournament,
    conn: &mut DbConn<'_>,
) -> Vec<InternalServerMessage> {
    let mut output = HandlerOutput::default();
    append_public_tournament_patches(
        tournament.id,
        &[
            PublicTournamentSection::Lifecycle,
            PublicTournamentSection::Memberships,
        ],
        &mut output,
        conn,
    )
    .await;
    output.messages.push(InternalServerMessage {
        destination: MessageDestination::Global,
        message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(TournamentId(
            tournament.nanoid,
        ))),
    });
    output.messages
}

pub(crate) async fn start_outcome_messages(
    tournament: Tournament,
    games: Vec<Game>,
    removed_invitees: Vec<Uuid>,
    conn: &mut DbConn<'_>,
) -> Vec<InternalServerMessage> {
    let tournament_id = TournamentId(tournament.nanoid.clone());

    let player_ids =
        load_player_ids_after_commit(&tournament, PlayerTournamentLifecycle::Started, conn).await;
    for player_id in &player_ids {
        notify(Event::TournamentStarted {
            recipient: *player_id,
            tournament_name: tournament.name.clone(),
            tournament_nanoid: tournament.nanoid.clone(),
        });
    }

    let messages = removed_invitees
        .into_iter()
        .map(|invitee| InternalServerMessage {
            destination: MessageDestination::User(invitee),
            message: ServerMessage::Tournament(TournamentUpdate::Uninvited(tournament_id.clone())),
        })
        .collect::<Vec<_>>();
    let mut output = HandlerOutput::from(messages);
    append_public_tournament_patches(
        tournament.id,
        &[
            PublicTournamentSection::Lifecycle,
            PublicTournamentSection::Memberships,
            PublicTournamentSection::Format,
            PublicTournamentSection::Standings,
        ],
        &mut output,
        conn,
    )
    .await;
    output.messages.extend(player_lifecycle_messages(
        tournament_id.clone(),
        player_ids,
        PlayerTournamentLifecycle::Started,
    ));
    output.messages.push(InternalServerMessage {
        destination: MessageDestination::Global,
        message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(tournament_id.clone())),
    });
    match new_tournament_game_messages(&games, conn).await {
        Ok(released) => output.messages.extend(released),
        Err(error) => log::error!(
            "Tournament {} started but released-game responses could not be built: {error}",
            tournament.nanoid,
        ),
    }
    output.messages
}
