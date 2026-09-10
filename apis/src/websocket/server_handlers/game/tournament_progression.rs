use crate::{
    common::{
        GameActionResponse,
        GameReaction,
        GameUpdate,
        ScheduleUpdate,
        ServerMessage,
        TournamentUpdate,
    },
    notifications::{
        game_end_reason_from,
        notify_game_control,
        notify_game_ended_excluding,
        notify_your_turn,
        GameControlKind,
        GameEndReason,
    },
    responses::{GameResponse, ScheduleResponse},
    websocket::{
        messages::{
            GameFinalize,
            HandlerOutput,
            InternalServerMessage,
            MessageDestination,
            Reaction,
            TerminalGameRetry,
            TournamentAudience,
            TvUpdate,
        },
        server_handlers::tournaments::{
            append_public_slot_patches_from_response,
            append_public_tournament_patches_from_response,
            arena::append_terminal_arena_game_projection,
            load_player_ids_after_commit,
            load_public_tournament_response,
            player_lifecycle_messages,
            PlayerTournamentLifecycle,
            PublicTournamentSection,
        },
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    db_error::DbError,
    game_command::{self, Command, Outcome},
    models::{Game, ScheduleOffer, Tournament, User},
    tournaments::{fixed_field, FixedFieldCommit},
    DbConn,
};
use hive_lib::GameControl;
use shared_types::{tournament::Format, Conclusion, GameId, GameSpeed, TournamentId};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

pub(crate) struct TerminalGameFanout<'a> {
    pub(crate) game: &'a Game,
    pub(crate) reaction: GameReaction,
    pub(crate) actor_id: Uuid,
    pub(crate) actor_username: Option<&'a str>,
    pub(crate) end_reason: Option<GameEndReason>,
    pub(crate) notification_excluded: Option<Uuid>,
    pub(crate) publish_tv: bool,
    pub(crate) deleted_row: bool,
}

struct GameReactionProjection<'a> {
    game: &'a Game,
    reaction: GameReaction,
    actor_id: Uuid,
    actor_username: Option<&'a str>,
    final_state: bool,
    deleted_row: bool,
    publish_tv: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NonterminalTournamentProjection {
    Slot { tournament_id: Uuid, slot_id: Uuid },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixedFieldPatchPlan {
    slots: bool,
    availability: bool,
    format: bool,
}

const fn fixed_field_patch_plan(
    format_changed: bool,
    availability_changed: bool,
) -> FixedFieldPatchPlan {
    FixedFieldPatchPlan {
        slots: !format_changed,
        availability: !format_changed && availability_changed,
        format: format_changed,
    }
}

const fn ready_changes_availability(format: Format) -> bool {
    matches!(
        format,
        Format::RoundRobin | Format::Swiss | Format::DoubleSwiss
    )
}

fn nonterminal_projection_plan(
    newly_terminal: bool,
    removed: bool,
    reaction: Option<&GameReaction>,
    tournament_id: Option<Uuid>,
    tournament_slot_id: Option<Uuid>,
) -> Option<NonterminalTournamentProjection> {
    if newly_terminal || removed {
        return None;
    }
    match (reaction, tournament_id) {
        (Some(GameReaction::Started), Some(tournament_id)) => {
            tournament_slot_id.map(|slot_id| NonterminalTournamentProjection::Slot {
                tournament_id,
                slot_id,
            })
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandAudience {
    Websocket,
    Bot,
}

#[derive(Clone, Debug)]
pub(crate) struct GameCommandContext {
    reaction: Option<GameReaction>,
    actor_id: Option<Uuid>,
    actor_username: Option<String>,
    audience: CommandAudience,
}

impl GameCommandContext {
    pub(crate) const fn silent() -> Self {
        Self {
            reaction: None,
            actor_id: None,
            actor_username: None,
            audience: CommandAudience::Websocket,
        }
    }

    pub(crate) fn websocket(
        reaction: GameReaction,
        actor_id: Uuid,
        actor_username: impl Into<String>,
    ) -> Self {
        Self::action(
            reaction,
            actor_id,
            actor_username,
            CommandAudience::Websocket,
        )
    }

    pub(crate) fn bot(
        reaction: GameReaction,
        actor_id: Uuid,
        actor_username: impl Into<String>,
    ) -> Self {
        Self::action(reaction, actor_id, actor_username, CommandAudience::Bot)
    }

    pub(crate) const fn websocket_actor(reaction: GameReaction, actor_id: Uuid) -> Self {
        Self {
            reaction: Some(reaction),
            actor_id: Some(actor_id),
            actor_username: None,
            audience: CommandAudience::Websocket,
        }
    }

    fn action(
        reaction: GameReaction,
        actor_id: Uuid,
        actor_username: impl Into<String>,
        audience: CommandAudience,
    ) -> Self {
        Self {
            reaction: Some(reaction),
            actor_id: Some(actor_id),
            actor_username: Some(actor_username.into()),
            audience,
        }
    }
}

pub(crate) enum GameProjectionInput {
    Command(Outcome),
    Committed { game: Game, newly_terminal: bool },
}

impl From<Outcome> for GameProjectionInput {
    fn from(outcome: Outcome) -> Self {
        Self::Command(outcome)
    }
}

pub(crate) struct ProjectedGame {
    pub(crate) game: Game,
    pub(crate) output: HandlerOutput,
    pub(crate) rejected: Option<DbError>,
    pub(crate) removed: bool,
}

/// Projects a committed fixed-field transition. The database result owns the
/// released Game identities and closed schedule offers; public Tournament
/// sections are reloaded only after the transaction commits.
pub(crate) async fn append_fixed_field_commit(
    effects: Option<FixedFieldCommit>,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let Some(effects) = effects else {
        return;
    };
    let FixedFieldCommit {
        tournament_id,
        affected_slot_ids,
        released_game_ids,
        standings_changed,
        format_changed,
        availability_changed,
        finished_now,
        catalog_changed,
        schedule_updates,
    } = effects;

    let tournament = match Tournament::find(tournament_id, conn).await {
        Ok(tournament) => Some(tournament),
        Err(error) => {
            log::error!("project committed tournament {tournament_id}: {error}");
            None
        }
    };
    let response_id = tournament
        .as_ref()
        .map(|tournament| TournamentId(tournament.nanoid.clone()));

    let patch_plan = fixed_field_patch_plan(format_changed, availability_changed);
    let mut sections = Vec::with_capacity(4);
    if patch_plan.availability {
        sections.push(PublicTournamentSection::Availability);
    }
    if standings_changed {
        sections.push(PublicTournamentSection::Standings);
    }
    if patch_plan.format {
        sections.push(PublicTournamentSection::Format);
    }
    if finished_now {
        sections.push(PublicTournamentSection::Lifecycle);
    }
    let needs_slot_patches = patch_plan.slots && !affected_slot_ids.is_empty();
    if needs_slot_patches || !sections.is_empty() {
        if let Some(response) = load_public_tournament_response(tournament_id, conn).await {
            if needs_slot_patches {
                append_public_slot_patches_from_response(&response, &affected_slot_ids, output);
            }
            if !sections.is_empty() {
                append_public_tournament_patches_from_response(&response, &sections, output);
            }
        }
    }

    if let Some(response_id) = response_id {
        if finished_now {
            let player_ids = match tournament.as_ref() {
                Some(tournament) => {
                    load_player_ids_after_commit(
                        tournament,
                        PlayerTournamentLifecycle::Finished,
                        conn,
                    )
                    .await
                }
                None => Vec::new(),
            };
            output.messages.extend(player_lifecycle_messages(
                response_id.clone(),
                player_ids,
                PlayerTournamentLifecycle::Finished,
            ));
            output.messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Schedule(ScheduleUpdate::TournamentPurged(
                    response_id.clone(),
                )),
            });
        }
        if catalog_changed {
            output.messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Tournament(TournamentUpdate::CatalogChanged(
                    response_id.clone(),
                )),
            });
        }
    }

    match new_tournament_game_messages_by_ids(&released_game_ids, conn).await {
        Ok(messages) => output.messages.extend(messages),
        Err(error) => {
            log::error!("build committed tournament released-game responses: {error}")
        }
    }
    output
        .messages
        .extend(schedule_offer_update_messages(schedule_updates, conn).await);
}

pub(crate) async fn schedule_offer_update_messages(
    offers: Vec<ScheduleOffer>,
    conn: &mut DbConn<'_>,
) -> Vec<InternalServerMessage> {
    match ScheduleResponse::from_models_batch(offers, conn).await {
        Ok(responses) => schedule_response_update_messages(responses),
        Err(error) => {
            log::error!("build automatic schedule offer updates: {error}");
            Vec::new()
        }
    }
}

pub(crate) fn schedule_response_update_messages(
    responses: Vec<ScheduleResponse>,
) -> Vec<InternalServerMessage> {
    let mut by_tournament = HashMap::<TournamentId, Vec<ScheduleResponse>>::new();
    for response in &responses {
        by_tournament
            .entry(response.tournament_id.clone())
            .or_default()
            .push(response.clone());
    }
    let mut messages: Vec<_> = schedule_responses_by_recipient(responses)
        .into_iter()
        .map(|(recipient, responses)| InternalServerMessage {
            destination: MessageDestination::User(recipient),
            message: ServerMessage::Schedule(ScheduleUpdate::Changed(responses)),
        })
        .collect();
    messages.extend(by_tournament.into_iter().map(|(tournament_id, responses)| {
        InternalServerMessage {
            destination: MessageDestination::Tournament {
                tournament_id,
                audience: TournamentAudience::ScheduleViewers,
            },
            message: ServerMessage::Schedule(ScheduleUpdate::Changed(responses)),
        }
    }));
    messages
}

fn schedule_responses_by_recipient(
    responses: Vec<ScheduleResponse>,
) -> Vec<(Uuid, Vec<ScheduleResponse>)> {
    let responses = responses
        .into_iter()
        .map(|response| (response.id, response))
        .collect::<BTreeMap<_, _>>();
    let mut by_recipient = BTreeMap::<Uuid, Vec<ScheduleResponse>>::new();
    for response in responses.into_values() {
        by_recipient
            .entry(response.proposer_id)
            .or_default()
            .push(response.clone());
        if response.opponent_id != response.proposer_id {
            by_recipient
                .entry(response.opponent_id)
                .or_default()
                .push(response);
        }
    }
    by_recipient.into_iter().collect()
}

/// Builds the common post-commit reaction, TV candidate, notification,
/// own-game removal, and finalization fanout. Dispatch remains the outer
/// boundary's responsibility.
pub(crate) async fn append_terminal_game_fanout(
    data: &WebsocketData,
    effects: &[TerminalGameFanout<'_>],
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    for effect in effects {
        let game = effect.game;
        let game_id = GameId(game.nanoid.clone());
        data.invalidate_game_response(&game_id);
        let published = append_game_reaction(
            data,
            GameReactionProjection {
                game,
                reaction: effect.reaction.clone(),
                actor_id: effect.actor_id,
                actor_username: effect.actor_username,
                final_state: true,
                deleted_row: effect.deleted_row,
                publish_tv: effect.publish_tv,
            },
            output,
            conn,
        )
        .await;
        if let Some(end_reason) = effect.end_reason {
            if let Err(error) =
                notify_game_ended_excluding(game, end_reason, effect.notification_excluded, conn)
                    .await
            {
                log::error!("notify terminal game {}: {error}", game.nanoid);
            }
        }
        if !published {
            if effect.deleted_row {
                output.terminal_game_retries.push(TerminalGameRetry {
                    game: game.clone(),
                    reaction: effect.reaction.clone(),
                    actor_id: effect.actor_id,
                    actor_username: effect.actor_username.map(str::to_owned),
                    publish_tv: effect.publish_tv,
                });
            }
            continue;
        }
        let finalize = GameFinalize {
            game_id,
            white_id: game.white_id,
            black_id: game.black_id,
        };
        output.messages.extend(finalize.own_game_removed_messages());
        output.finalize_games.push(finalize);
    }
}

pub(crate) async fn retry_deleted_terminal_game_projection(
    data: &WebsocketData,
    retry: &TerminalGameRetry,
    conn: &mut DbConn<'_>,
) -> HandlerOutput {
    let mut output = HandlerOutput::empty();
    append_terminal_game_fanout(
        data,
        &[TerminalGameFanout {
            game: &retry.game,
            reaction: retry.reaction.clone(),
            actor_id: retry.actor_id,
            actor_username: retry.actor_username.as_deref(),
            end_reason: None,
            notification_excluded: None,
            publish_tv: retry.publish_tv,
            deleted_row: true,
        }],
        &mut output,
        conn,
    )
    .await;
    output
}

async fn append_game_reaction(
    data: &WebsocketData,
    projection: GameReactionProjection<'_>,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) -> bool {
    let GameReactionProjection {
        game,
        reaction,
        actor_id,
        actor_username,
        final_state,
        deleted_row,
        publish_tv,
    } = projection;
    let response = if deleted_row {
        data.telemetry.inc_from_model();
        GameResponse::from_model(game, conn).await
    } else {
        data.get_or_build_response(game, conn)
            .await
            .map(|response| (*response).clone())
    };
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            log::error!(
                "Game {} committed but its response could not be built: {error}",
                game.nanoid,
            );
            return false;
        }
    };
    let username = actor_username.map(str::to_owned).or_else(|| {
        if response.white_player.uid == actor_id {
            Some(response.white_player.username.clone())
        } else if response.black_player.uid == actor_id {
            Some(response.black_player.username.clone())
        } else {
            None
        }
    });
    let Some(username) = username else {
        log::error!(
            "Game {} committed but reaction actor {actor_id} has no response identity",
            game.nanoid,
        );
        return false;
    };
    let game_id = GameId(game.nanoid.clone());
    output.reactions.push(Reaction {
        game_id: game_id.clone(),
        white_id: game.white_id,
        black_id: game.black_id,
        gar: GameActionResponse {
            game_action: reaction,
            game: response.clone(),
            game_id: game_id.clone(),
            user_id: actor_id,
            username,
        },
    });
    if publish_tv {
        output.tv_updates.push(TvUpdate {
            game_id,
            game: response,
            final_state,
        });
    }
    true
}

async fn append_urgent_games(
    data: &WebsocketData,
    recipient_id: Uuid,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let result = async {
        let recipient = User::find_by_uuid(&recipient_id, conn).await?;
        let games = recipient.get_games_with_notifications(conn).await?;
        let mut responses = Vec::with_capacity(games.len());
        for game in &games {
            responses.push((*data.get_or_build_response(game, conn).await?).clone());
        }
        Ok::<_, anyhow::Error>(responses)
    }
    .await;
    match result {
        Ok(responses) => output.messages.push(InternalServerMessage {
            destination: MessageDestination::User(recipient_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Urgent(responses))),
        }),
        Err(error) => log::error!("build committed urgent game update: {error}"),
    }
}

const fn control_notification_kind(control: GameControl) -> Option<GameControlKind> {
    match control {
        GameControl::DrawOffer(_) => Some(GameControlKind::DrawOffered),
        GameControl::TakebackRequest(_) => Some(GameControlKind::TakebackRequested),
        GameControl::DrawReject(_) => Some(GameControlKind::DrawRejected),
        GameControl::TakebackAccept(_) => Some(GameControlKind::TakebackAccepted),
        GameControl::TakebackReject(_) => Some(GameControlKind::TakebackRejected),
        GameControl::Resign(_) | GameControl::Abort(_) | GameControl::DrawAccept(_) => None,
    }
}

async fn append_action_side_effects(
    context: &GameCommandContext,
    game: &Game,
    terminal: bool,
    data: &WebsocketData,
    output: &mut HandlerOutput,
    conn: &mut DbConn<'_>,
) {
    let (Some(reaction), Some(actor_id), Some(actor_username)) = (
        context.reaction.as_ref(),
        context.actor_id,
        context.actor_username.as_deref(),
    ) else {
        return;
    };
    match reaction {
        GameReaction::Turn(_) => {
            append_urgent_games(data, game.current_player_id, output, conn).await;
            if !terminal {
                notify_your_turn(game, actor_username.to_owned());
            }
        }
        GameReaction::Control(control) => {
            let urgent_recipient = match context.audience {
                CommandAudience::Bot => Some(if game.white_id == actor_id {
                    game.black_id
                } else {
                    game.white_id
                }),
                CommandAudience::Websocket
                    if matches!(
                        control,
                        GameControl::DrawOffer(_) | GameControl::TakebackRequest(_)
                    ) =>
                {
                    Some(game.current_player_id)
                }
                CommandAudience::Websocket => None,
            };
            if let Some(recipient_id) = urgent_recipient {
                append_urgent_games(data, recipient_id, output, conn).await;
            }
            if context.audience == CommandAudience::Websocket {
                if let Some(kind) = control_notification_kind(*control) {
                    let recipient = if actor_id == game.white_id {
                        game.black_id
                    } else {
                        game.white_id
                    };
                    match game.speed.parse::<GameSpeed>() {
                        Ok(speed) => notify_game_control(
                            recipient,
                            actor_username.to_owned(),
                            game.nanoid.clone(),
                            kind,
                            speed,
                        ),
                        Err(error) => log::error!(
                            "Game {} committed but its control notification speed is invalid: {error}",
                            game.nanoid,
                        ),
                    }
                }
            }
        }
        GameReaction::Started
        | GameReaction::Ready
        | GameReaction::Berserk
        | GameReaction::TimedOut
        | GameReaction::Join
        | GameReaction::New
        | GameReaction::Reopened
        | GameReaction::Adjudicated
        | GameReaction::Finished
        | GameReaction::Tv => {}
    }
}

fn terminal_end_reason(game: &Game, reaction: &GameReaction) -> Option<GameEndReason> {
    match reaction {
        GameReaction::TimedOut => Some(GameEndReason::Timeout),
        GameReaction::Turn(_) => Some(game_end_reason_from(game, GameEndReason::Move)),
        GameReaction::Control(GameControl::Resign(_)) => {
            Some(game_end_reason_from(game, GameEndReason::Resignation))
        }
        GameReaction::Control(GameControl::DrawAccept(_)) => {
            Some(game_end_reason_from(game, GameEndReason::Agreement))
        }
        GameReaction::Control(_)
        | GameReaction::Started
        | GameReaction::Ready
        | GameReaction::Berserk
        | GameReaction::Join
        | GameReaction::New
        | GameReaction::Reopened
        | GameReaction::Adjudicated
        | GameReaction::Finished
        | GameReaction::Tv => None,
    }
}

/// The common API projection for committed Game changes. It invalidates the
/// game cache and handles the authoritative reaction, urgent update, TV
/// candidate, notification, and terminal fanout. Dispatch remains the outer
/// boundary's responsibility.
pub(crate) async fn project_committed_game(
    input: impl Into<GameProjectionInput>,
    context: GameCommandContext,
    data: &WebsocketData,
    conn: &mut DbConn<'_>,
) -> ProjectedGame {
    let (mut game, newly_terminal, schedule_offer_updates, rejected, removed) = match input.into() {
        GameProjectionInput::Command(Outcome::Applied {
            game,
            newly_terminal,
            schedule_offer_updates,
        }) => (game, newly_terminal, schedule_offer_updates, None, false),
        GameProjectionInput::Command(Outcome::TimedOut {
            game,
            rejected,
            schedule_offer_updates,
        }) => (game, true, schedule_offer_updates, Some(rejected), false),
        GameProjectionInput::Command(Outcome::Removed { previous }) => {
            (previous, false, Vec::new(), None, true)
        }
        GameProjectionInput::Committed {
            game,
            newly_terminal,
        } => (game, newly_terminal, Vec::new(), None, false),
    };
    if removed {
        game.finished = true;
    }
    data.invalidate_game_response(&GameId(game.nanoid.clone()));
    let mut output = HandlerOutput::empty();
    output
        .messages
        .extend(schedule_offer_update_messages(schedule_offer_updates, conn).await);

    let timed_out = newly_terminal && game.conclusion == Conclusion::Timeout.to_string();
    let reaction = if timed_out {
        Some(GameReaction::TimedOut)
    } else {
        context.reaction.clone()
    };
    let nonterminal_projection = nonterminal_projection_plan(
        newly_terminal,
        removed,
        reaction.as_ref(),
        game.tournament_id,
        game.tournament_slot_id,
    );
    if !timed_out {
        append_action_side_effects(
            &context,
            &game,
            newly_terminal || removed,
            data,
            &mut output,
            conn,
        )
        .await;
    }
    if let Some(reaction) = reaction {
        let actor_id = if timed_out {
            game.current_player_id
        } else {
            context
                .actor_id
                .expect("a projected game reaction has an actor")
        };
        let actor_username = (!timed_out)
            .then_some(context.actor_username.as_deref())
            .flatten();
        if newly_terminal || removed {
            let effect = TerminalGameFanout {
                game: &game,
                end_reason: terminal_end_reason(&game, &reaction),
                reaction,
                actor_id,
                actor_username,
                deleted_row: removed,
                notification_excluded: None,
                publish_tv: true,
            };
            append_terminal_game_fanout(data, &[effect], &mut output, conn).await;
        } else {
            append_game_reaction(
                data,
                GameReactionProjection {
                    game: &game,
                    reaction,
                    actor_id,
                    actor_username,
                    final_state: false,
                    deleted_row: false,
                    publish_tv: true,
                },
                &mut output,
                conn,
            )
            .await;
        }
    }

    match nonterminal_projection {
        Some(NonterminalTournamentProjection::Slot {
            tournament_id,
            slot_id,
        }) => {
            // Starting a Ready Game also clears its Slot's schedule. A single
            // Slot patch carries both committed compact changes.
            if let Some(response) = load_public_tournament_response(tournament_id, conn).await {
                append_public_slot_patches_from_response(&response, &[slot_id], &mut output);
                if ready_changes_availability(response.format.kind()) {
                    append_public_tournament_patches_from_response(
                        &response,
                        &[PublicTournamentSection::Availability],
                        &mut output,
                    );
                }
            }
        }
        None => {}
    }

    if newly_terminal && game.arena_ordinal.is_some() {
        append_terminal_arena_game_projection(&game, &mut output, conn).await;
    }

    let reconciliation = if newly_terminal && game.tournament_slot_id.is_some() {
        match game.tournament_id {
            Some(tournament_id) => match fixed_field::reconcile(tournament_id, conn).await {
                Ok(effects) => effects,
                Err(error) => {
                    log::error!(
                        "Game {} committed but fixed-field tournament {} reconciliation failed: {error}",
                        game.nanoid,
                        tournament_id,
                    );
                    None
                }
            },
            None => {
                log::error!(
                    "Game {} committed with a Slot but no Tournament identity",
                    game.nanoid,
                );
                None
            }
        }
    } else {
        None
    };
    append_fixed_field_commit(reconciliation, &mut output, conn).await;

    ProjectedGame {
        game,
        output,
        rejected,
        removed,
    }
}

pub(crate) fn finish_post_commit_projection(
    game_id: &str,
    mut committed: HandlerOutput,
    projection: Result<HandlerOutput>,
) -> HandlerOutput {
    match projection {
        Ok(projected) => committed.append(projected),
        Err(error) => {
            log::error!(
                "Game {game_id} committed but subsequent websocket effects could not be built: {error}",
            );
            committed.request_error = Some(error);
        }
    }
    committed
}

pub(crate) async fn new_tournament_game_messages(
    released_games: &[Game],
    conn: &mut DbConn<'_>,
) -> Result<Vec<InternalServerMessage>> {
    let released_ids = released_games
        .iter()
        .map(|game| GameId(game.nanoid.clone()))
        .collect::<Vec<_>>();
    new_tournament_game_messages_by_ids(&released_ids, conn).await
}

pub(crate) async fn new_tournament_game_messages_by_ids(
    released_ids: &[GameId],
    conn: &mut DbConn<'_>,
) -> Result<Vec<InternalServerMessage>> {
    let current_games = Game::find_by_nanoids(released_ids, conn).await?;
    let games = GameResponse::from_games_batch(current_games, conn).await?;
    let mut messages = Vec::with_capacity(games.len().saturating_mul(2));
    for game in games {
        if game.finished {
            continue;
        }
        for (user_id, username) in [
            (game.white_player.uid, game.white_player.username.clone()),
            (game.black_player.uid, game.black_player.username.clone()),
        ] {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(user_id),
                message: ServerMessage::Game(Box::new(crate::common::GameUpdate::Reaction(
                    GameActionResponse {
                        game_action: GameReaction::New,
                        game: game.clone(),
                        game_id: game.game_id.clone(),
                        user_id,
                        username,
                    },
                ))),
            });
        }
    }
    Ok(messages)
}

pub(crate) async fn settle_deadline(
    game_id: Uuid,
    data: &WebsocketData,
    conn: &mut DbConn<'_>,
) -> std::result::Result<ProjectedGame, DbError> {
    let outcome = game_command::execute(game_id, Command::SettleDeadline, conn).await?;
    Ok(project_committed_game(outcome, GameCommandContext::silent(), data, conn).await)
}

#[cfg(test)]
mod tests {
    use super::{
        nonterminal_projection_plan,
        schedule_response_update_messages,
        schedule_responses_by_recipient,
        NonterminalTournamentProjection,
    };
    use crate::{
        common::{GameReaction, ScheduleUpdate, ServerMessage},
        responses::ScheduleResponse,
        websocket::{MessageDestination, TournamentAudience},
    };
    use chrono::{DateTime, Utc};
    use hive_lib::{Bug, Color, Piece, Position, Turn};
    use shared_types::{ScheduleOfferStatus, TournamentId};
    use uuid::Uuid;

    fn schedule_response(id: u128, participants: [Uuid; 2]) -> ScheduleResponse {
        let created_at = DateTime::<Utc>::from_timestamp(1, 0).unwrap();
        ScheduleResponse {
            id: Uuid::from_u128(id),
            slot_id: Uuid::from_u128(id + 100),
            slot_context: String::from("Game 1"),
            tournament_name: String::from("Cup"),
            tournament_id: TournamentId(String::from("cup")),
            proposer_id: participants[0],
            proposer_username: String::from("proposer"),
            opponent_id: participants[1],
            opponent_username: String::from("opponent"),
            candidate_times: vec![created_at],
            status: ScheduleOfferStatus::Cancelled,
            selected_time: None,
            created_at,
            resolved_at: Some(created_at),
            resolved_by: None,
            notified: false,
        }
    }

    #[test]
    fn routes_only_committed_nonterminal_compact_tournament_changes() {
        let tournament_id = Uuid::from_u128(1);
        let slot_id = Uuid::from_u128(2);

        assert_eq!(
            nonterminal_projection_plan(
                false,
                false,
                Some(&GameReaction::Started),
                Some(tournament_id),
                Some(slot_id),
            ),
            Some(NonterminalTournamentProjection::Slot {
                tournament_id,
                slot_id,
            })
        );
        assert_eq!(
            nonterminal_projection_plan(
                false,
                false,
                Some(&GameReaction::Berserk),
                Some(tournament_id),
                None,
            ),
            None
        );
        assert_eq!(
            nonterminal_projection_plan(
                false,
                false,
                Some(&GameReaction::Ready),
                Some(tournament_id),
                Some(slot_id),
            ),
            None
        );
        assert_eq!(
            nonterminal_projection_plan(
                true,
                false,
                Some(&GameReaction::Started),
                Some(tournament_id),
                Some(slot_id),
            ),
            None
        );
    }

    #[test]
    fn fixed_field_move_does_not_publish_tournament_changes() {
        let tournament_id = Uuid::from_u128(1);
        let slot_id = Uuid::from_u128(2);
        let plan = nonterminal_projection_plan(
            false,
            false,
            Some(&GameReaction::Turn(Turn::Move(
                Piece::new_from(Bug::Queen, Color::White, 0),
                Position { q: 0, r: 0 },
            ))),
            Some(tournament_id),
            Some(slot_id),
        );

        assert_eq!(plan, None);
    }

    #[test]
    fn schedule_updates_include_private_viewers_of_only_the_affected_tournament() {
        let first = schedule_response(11, [Uuid::from_u128(1), Uuid::from_u128(2)]);
        let mut second = schedule_response(12, [Uuid::from_u128(3), Uuid::from_u128(4)]);
        second.tournament_id = TournamentId(String::from("other-cup"));
        let messages = schedule_response_update_messages(vec![first.clone(), second.clone()]);
        let mut viewer_updates = 0;
        for message in messages {
            let ServerMessage::Schedule(ScheduleUpdate::Changed(responses)) = message.message
            else {
                panic!("expected schedule updates");
            };
            match message.destination {
                MessageDestination::Tournament {
                    tournament_id,
                    audience: TournamentAudience::ScheduleViewers,
                } => {
                    viewer_updates += 1;
                    assert_eq!(responses.len(), 1);
                    assert_eq!(responses[0].tournament_id, tournament_id);
                }
                MessageDestination::User(user_id) => {
                    assert!(responses.iter().all(|response| {
                        response.proposer_id == user_id || response.opponent_id == user_id
                    }));
                }
                _ => panic!("private offers must not use a public audience"),
            }
        }
        assert_eq!(viewer_updates, 2);
    }

    #[test]
    fn schedule_batches_contain_only_each_recipient_offers() {
        let players = [
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(3),
            Uuid::from_u128(4),
        ];
        let first = schedule_response(11, [players[0], players[1]]);
        let second = schedule_response(12, [players[2], players[3]]);
        let third = schedule_response(13, [players[1], players[2]]);

        let deliveries = schedule_responses_by_recipient(vec![
            third.clone(),
            first.clone(),
            second.clone(),
            first,
        ]);

        assert_eq!(
            deliveries
                .iter()
                .map(|(recipient, _)| *recipient)
                .collect::<Vec<_>>(),
            players,
        );
        for (recipient, responses) in &deliveries {
            assert!(responses.iter().all(|response| {
                response.proposer_id == *recipient || response.opponent_id == *recipient
            }));
        }
        assert_eq!(
            deliveries
                .iter()
                .map(|(_, responses)| responses
                    .iter()
                    .map(|response| response.id)
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![
                vec![Uuid::from_u128(11)],
                vec![Uuid::from_u128(11), Uuid::from_u128(13)],
                vec![Uuid::from_u128(12), Uuid::from_u128(13)],
                vec![Uuid::from_u128(12)],
            ],
        );
    }
}
