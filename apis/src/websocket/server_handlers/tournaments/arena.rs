use super::{
    append_public_tournament_patches_from_response,
    load_player_ids_after_commit,
    load_public_tournament_response,
    player_lifecycle_messages,
    public_tournament_patch_message,
    PlayerTournamentLifecycle,
    PublicTournamentSection,
};
use crate::{
    common::{ScheduleUpdate, ServerMessage, TournamentUpdate},
    responses::{TournamentMemberships, TournamentPatch, TournamentResponse, TournamentStandings},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination},
        server_handlers::game::new_tournament_game_messages,
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Tournament},
    tournaments::{arena, public::load_arena_player_stats},
    DbConn,
    DbPool,
};
use shared_types::{
    tournament::arena::PairingIntent,
    tournament_view::{ArenaGameResponse, ArenaPlayerStatsResponse, TournamentFormatResponse},
    GameId,
    TournamentId,
};
use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum ArenaAction {
    Join,
    Pause,
    Resume,
}

pub struct ArenaHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    action: ArenaAction,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl ArenaHandler {
    pub fn join(
        tournament_id: TournamentId,
        user_id: Uuid,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self::new(tournament_id, user_id, ArenaAction::Join, hub, pool)
    }

    pub fn pause(
        tournament_id: TournamentId,
        user_id: Uuid,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self::new(tournament_id, user_id, ArenaAction::Pause, hub, pool)
    }

    pub fn resume(
        tournament_id: TournamentId,
        user_id: Uuid,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self::new(tournament_id, user_id, ArenaAction::Resume, hub, pool)
    }

    fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        action: ArenaAction,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            user_id,
            action,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        match self.action {
            ArenaAction::Join => self.join_live(tournament, &mut conn).await,
            ArenaAction::Pause => {
                self.set_intent(tournament.id, PairingIntent::Paused, &mut conn)
                    .await
            }
            ArenaAction::Resume => {
                self.set_intent(tournament.id, PairingIntent::Enabled, &mut conn)
                    .await
            }
        }
    }

    async fn join_live(
        &self,
        tournament: Tournament,
        conn: &mut DbConn<'_>,
    ) -> Result<HandlerOutput> {
        let presence_hub = Arc::clone(&self.hub);
        let outcome = arena::join_with_presence(
            tournament.id,
            self.user_id,
            move |candidates| presence_hub.authenticated_presence_for(candidates),
            conn,
        )
        .await?;
        let response_id = TournamentId(tournament.nanoid.clone());
        let mut output = HandlerOutput::empty();
        append_arena_incremental_effects(
            tournament.id,
            outcome.joined_now,
            &[self.user_id],
            &[],
            &mut output,
            conn,
        )
        .await;
        if outcome.joined_now {
            output.messages.push(InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Tournament(TournamentUpdate::Joined(response_id.clone())),
            });
            output.messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(response_id)),
            });
        }
        Ok(output)
    }

    async fn set_intent(
        &self,
        tournament_id: Uuid,
        intent: PairingIntent,
        conn: &mut DbConn<'_>,
    ) -> Result<HandlerOutput> {
        arena::set_pairing_intent(tournament_id, self.user_id, intent, conn).await?;
        let mut output = HandlerOutput::empty();
        append_arena_player_stats_patches(tournament_id, &[self.user_id], &mut output, conn).await;
        Ok(output)
    }
}

fn arena_entity_patches(
    response: &TournamentResponse,
    game_ids: &[GameId],
    player_ids: &[Uuid],
    feature_if_new_game: bool,
) -> Result<Vec<TournamentPatch>> {
    let TournamentFormatResponse::Arena {
        games,
        player_stats,
        featured_game_id,
        ..
    } = &response.format
    else {
        anyhow::bail!("public projection is not an Arena")
    };
    arena_format_entity_patches(
        games,
        player_stats,
        featured_game_id,
        game_ids,
        player_ids,
        feature_if_new_game,
    )
}

fn arena_format_entity_patches(
    games: &[ArenaGameResponse],
    player_stats: &[ArenaPlayerStatsResponse],
    featured_game_id: &Option<GameId>,
    game_ids: &[GameId],
    player_ids: &[Uuid],
    feature_if_new_game: bool,
) -> Result<Vec<TournamentPatch>> {
    let mut patches = Vec::with_capacity(
        game_ids
            .len()
            .saturating_add(player_ids.len())
            .saturating_add(usize::from(feature_if_new_game)),
    );
    for game_id in game_ids {
        let game = games
            .iter()
            .find(|game| &game.game.game_id == game_id)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("Arena Game {game_id} is absent from the public projection")
            })?;
        patches.push(TournamentPatch::ArenaGameUpsert(game));
    }
    let mut selected_players = HashSet::with_capacity(player_ids.len());
    for player_id in player_ids {
        if !selected_players.insert(*player_id) {
            continue;
        }
        let stats = player_stats
            .iter()
            .find(|stats| stats.player == *player_id)
            .copied()
            .ok_or_else(|| {
                anyhow::anyhow!("Arena player {player_id} is absent from the public projection")
            })?;
        patches.push(TournamentPatch::ArenaPlayerStatsUpsert(stats));
    }
    if feature_if_new_game
        && featured_game_id
            .as_ref()
            .is_some_and(|featured| game_ids.contains(featured))
    {
        patches.push(TournamentPatch::ArenaFeaturedGameChanged(
            featured_game_id.clone(),
        ));
    }
    Ok(patches)
}

fn append_patch_messages(
    response: &TournamentResponse,
    patches: impl IntoIterator<Item = TournamentPatch>,
    output: &mut HandlerOutput,
) {
    output.messages.extend(
        patches
            .into_iter()
            .map(|patch| public_tournament_patch_message(response.tournament_id.clone(), patch)),
    );
}

fn terminal_arena_patch_set(
    mut entity_patches: Vec<TournamentPatch>,
    standings: TournamentStandings,
) -> Result<Vec<TournamentPatch>> {
    let projected_game_is_terminal = matches!(
        entity_patches.first(),
        Some(TournamentPatch::ArenaGameUpsert(game)) if game.game.finished
    );
    if !projected_game_is_terminal || entity_patches.len() != 3 {
        anyhow::bail!(
            "terminal Arena Game did not project as one terminal Game and two player stats"
        );
    }
    entity_patches.push(TournamentPatch::StandingsReplace(standings));
    Ok(entity_patches)
}

pub(crate) async fn append_arena_player_stats_patches(
    tournament_id: Uuid,
    player_ids: &[Uuid],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let patches = async {
        let (response_id, stats) = load_arena_player_stats(tournament_id, conn).await?;
        let patches = arena_format_entity_patches(&[], &stats, &None, &[], player_ids, false)?;
        Ok::<_, anyhow::Error>((response_id, patches))
    }
    .await;
    match patches {
        Ok((response_id, patches)) => output.messages.extend(
            patches
                .into_iter()
                .map(|patch| public_tournament_patch_message(response_id.clone(), patch)),
        ),
        Err(error) => log::error!(
            "Arena {tournament_id} committed but its player patch could not be built: {error}"
        ),
    }
}

async fn append_arena_incremental_effects(
    tournament_id: Uuid,
    memberships_changed: bool,
    additionally_affected_players: &[Uuid],
    new_games: &[Game],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    if memberships_changed || !additionally_affected_players.is_empty() || !new_games.is_empty() {
        if let Some(response) = load_public_tournament_response(tournament_id, conn).await {
            let game_ids = new_games
                .iter()
                .map(|game| GameId(game.nanoid.clone()))
                .collect::<Vec<_>>();
            let player_ids = additionally_affected_players
                .iter()
                .copied()
                .chain(
                    new_games
                        .iter()
                        .flat_map(|game| [game.white_id, game.black_id]),
                )
                .collect::<Vec<_>>();
            let patches =
                arena_entity_patches(&response, &game_ids, &player_ids, true).map(|mut patches| {
                    if memberships_changed {
                        patches.insert(
                            0,
                            TournamentPatch::MembershipsReplace(
                                TournamentMemberships::from_response(&response),
                            ),
                        );
                    }
                    patches
                });
            match patches {
                Ok(patches) => append_patch_messages(&response, patches, output),
                Err(error) => log::error!(
                    "Arena {tournament_id} committed but its incremental patches could not be built: {error}"
                ),
            }
        }
    }
    match new_tournament_game_messages(new_games, conn).await {
        Ok(messages) => output.messages.extend(messages),
        Err(error) => {
            log::error!("Arena mutation committed but new-game effects could not be built: {error}")
        }
    }
}

pub(crate) async fn append_arena_pairing_effects(
    tournament_id: Uuid,
    new_games: &[Game],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    append_arena_incremental_effects(tournament_id, false, &[], new_games, output, conn).await;
}

pub(crate) async fn append_terminal_arena_game_projection(
    game: &Game,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let Some(tournament_id) = game.tournament_id else {
        log::error!(
            "Arena Game {} committed without a Tournament identity",
            game.nanoid
        );
        return;
    };
    let Some(response) = load_public_tournament_response(tournament_id, conn).await else {
        return;
    };
    let game_id = GameId(game.nanoid.clone());
    let patches = arena_entity_patches(
        &response,
        std::slice::from_ref(&game_id),
        &[game.white_id, game.black_id],
        false,
    )
    .and_then(|patches| {
        terminal_arena_patch_set(patches, TournamentStandings::from_response(&response))
    });
    match patches {
        Ok(patches) => append_patch_messages(&response, patches, output),
        Err(error) => {
            if response
                .arena_games()
                .iter()
                .all(|projected| projected.game.game_id != game_id)
                && response.status == shared_types::TournamentStatus::Finished
            {
                // A Game excluded from the frozen cutoff projection remains
                // durable and rated, but must not re-enter Arena history.
                return;
            }
            log::error!(
                "Arena Game {} committed but its terminal patches could not be built: {error}",
                game.nanoid
            );
        }
    }
}

pub(crate) async fn append_arena_finalization_effects(
    tournament_id: Uuid,
    fallback_id: TournamentId,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let response = load_public_tournament_response(tournament_id, conn).await;
    let response_id = response
        .as_ref()
        .map(|response| response.tournament_id.clone())
        .unwrap_or(fallback_id);
    let player_ids = if let Some(response) = response.as_ref() {
        response.players.keys().copied().collect()
    } else {
        match Tournament::find(tournament_id, conn).await {
            Ok(tournament) => {
                load_player_ids_after_commit(&tournament, PlayerTournamentLifecycle::Finished, conn)
                    .await
            }
            Err(error) => {
                log::error!(
                    "Arena {tournament_id} finished but its participants could not be loaded for lifecycle messages: {error}"
                );
                Vec::new()
            }
        }
    };
    if let Some(response) = response {
        append_public_tournament_patches_from_response(
            &response,
            &[
                PublicTournamentSection::Lifecycle,
                PublicTournamentSection::Format,
                PublicTournamentSection::Standings,
            ],
            output,
        );
    }
    output
        .messages
        .extend(arena_finished_semantic_messages(response_id, player_ids));
}

fn arena_finished_semantic_messages(
    tournament_id: TournamentId,
    player_ids: Vec<Uuid>,
) -> Vec<InternalServerMessage> {
    let mut messages = player_lifecycle_messages(
        tournament_id.clone(),
        player_ids,
        PlayerTournamentLifecycle::Finished,
    );
    messages.extend([
        InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(
                tournament_id.clone(),
            )),
        },
        InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Schedule(ScheduleUpdate::TournamentPurged(
                tournament_id.clone(),
            )),
        },
    ]);
    messages
}

#[cfg(test)]
mod tests {
    use super::{
        arena_finished_semantic_messages,
        arena_format_entity_patches,
        terminal_arena_patch_set,
    };
    use crate::{
        common::{ScheduleUpdate, ServerMessage, TournamentUpdate},
        responses::{TournamentPatch, TournamentStandings},
        websocket::messages::MessageDestination,
    };
    use chrono::{DateTime, Utc};
    use hive_lib::GameStatus;
    use shared_types::{
        tournament::Score,
        tournament_view::{
            ArenaGameResponse,
            ArenaPlayerStatsResponse,
            CompactTournamentGameResponse,
        },
        GameId,
        GameSpeed,
        GameStart,
        TournamentId,
    };
    use uuid::Uuid;

    fn compact_game(game_id: &str, participants: [Uuid; 2]) -> CompactTournamentGameResponse {
        CompactTournamentGameResponse {
            game_id: GameId(game_id.to_owned()),
            participants,
            status: GameStatus::Adjudicated,
            start: GameStart::Arena,
            finished: true,
            ratings: [Some(1_500), Some(1_500)],
            berserked: [false, false],
            speed: GameSpeed::Blitz,
            finished_at: Some(DateTime::<Utc>::from_timestamp(2, 0).unwrap()),
        }
    }

    fn arena_game(game_id: &str, participants: [Uuid; 2]) -> ArenaGameResponse {
        ArenaGameResponse {
            ordinal: 0,
            game: compact_game(game_id, participants),
            outcome: None,
            awarded_points: None,
            doubled: None,
            no_start_absent: None,
        }
    }

    fn arena_stats(player: Uuid) -> ArenaPlayerStatsResponse {
        ArenaPlayerStatsResponse {
            player,
            points: Score::new(0),
            performance_rating: None,
            average_opponent_rating: None,
            performance_games: 0,
            arena_rating: Some(1_500),
            games_scored: 0,
            games_played: 0,
            no_starts: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            current_streak: 0,
            on_fire: false,
            best_streak: 0,
            berserks: 0,
            paused: false,
        }
    }

    #[test]
    fn terminal_arena_delta_is_exactly_one_game_two_players_and_standings() {
        let players = [Uuid::from_u128(1), Uuid::from_u128(2)];
        let unrelated = Uuid::from_u128(3);
        let requested_id = GameId(String::from("requested"));
        let games = [
            arena_game("unrelated", [players[0], unrelated]),
            arena_game(&requested_id.0, players),
        ];
        let stats = [
            arena_stats(unrelated),
            arena_stats(players[1]),
            arena_stats(players[0]),
        ];

        let entity_patches = arena_format_entity_patches(
            &games,
            &stats,
            &Some(requested_id.clone()),
            std::slice::from_ref(&requested_id),
            &players,
            false,
        )
        .expect("select exact Arena entities");
        let patches = terminal_arena_patch_set(entity_patches, TournamentStandings::default())
            .expect("complete the terminal Arena patch set");

        assert_eq!(patches.len(), 4);
        assert!(matches!(
            &patches[0],
            TournamentPatch::ArenaGameUpsert(game) if game.game.game_id == requested_id
        ));
        assert!(matches!(
            &patches[1],
            TournamentPatch::ArenaPlayerStatsUpsert(stats) if stats.player == players[0]
        ));
        assert!(matches!(
            &patches[2],
            TournamentPatch::ArenaPlayerStatsUpsert(stats) if stats.player == players[1]
        ));
        assert!(matches!(&patches[3], TournamentPatch::StandingsReplace(_)));
    }

    #[test]
    fn pairing_delta_marks_feature_only_when_the_new_wave_changed_it() {
        let players = [Uuid::from_u128(1), Uuid::from_u128(2)];
        let requested_id = GameId(String::from("new"));
        let games = [arena_game(&requested_id.0, players)];
        let stats = [arena_stats(players[0]), arena_stats(players[1])];

        let patches = arena_format_entity_patches(
            &games,
            &stats,
            &Some(requested_id.clone()),
            std::slice::from_ref(&requested_id),
            &players,
            true,
        )
        .expect("select pairing wave");
        assert!(matches!(
            patches.last(),
            Some(TournamentPatch::ArenaFeaturedGameChanged(Some(game_id)))
                if game_id == &requested_id
        ));

        let unchanged = arena_format_entity_patches(
            &games,
            &stats,
            &Some(GameId(String::from("existing"))),
            &[requested_id],
            &players,
            true,
        )
        .expect("select pairing wave with unchanged feature");
        assert!(unchanged
            .iter()
            .all(|patch| !matches!(patch, TournamentPatch::ArenaFeaturedGameChanged(_))));
    }

    #[test]
    fn arena_finish_notifies_players_and_keeps_catalog_and_capability_routing_scoped() {
        let tournament_id = TournamentId(String::from("arena"));
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let messages =
            arena_finished_semantic_messages(tournament_id.clone(), vec![second, first, second]);

        assert!(matches!(
            &messages[0],
            crate::websocket::messages::InternalServerMessage {
                destination: MessageDestination::User(destination_id),
                message: ServerMessage::Tournament(TournamentUpdate::Finished(message_id)),
            } if *destination_id == first && message_id == &tournament_id
        ));
        assert!(matches!(
            &messages[1],
            crate::websocket::messages::InternalServerMessage {
                destination: MessageDestination::User(destination_id),
                message: ServerMessage::Tournament(TournamentUpdate::Finished(message_id)),
            } if *destination_id == second && message_id == &tournament_id
        ));
        assert!(matches!(
            &messages[3],
            crate::websocket::messages::InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(message_id)),
            } if message_id == &tournament_id
        ));
        assert!(matches!(
            &messages[4],
            crate::websocket::messages::InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Schedule(ScheduleUpdate::TournamentPurged(message_id)),
            } if message_id == &tournament_id
        ));
    }
}
