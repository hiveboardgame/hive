use crate::{
    common::{
        ScheduleAction::{self, Accept, Cancel, Propose, TournamentOwn, TournamentPublic},
        ScheduleUpdate, ServerMessage,
    },
    responses::ScheduleResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, NewSchedule, Schedule, Tournament},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::GameId;
use std::{collections::HashMap, vec};
use uuid::Uuid;

pub struct ScheduleHandler {
    pool: DbPool,
    user_id: Uuid,
    action: ScheduleAction,
}

impl ScheduleHandler {
    pub async fn new(user_id: Uuid, action: ScheduleAction, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            user_id,
            action,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let (update, destinations) = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    Ok(match self.action.clone() {
                        Accept(id) => {
                            let mut schedule = Schedule::from_id(id, tc).await?;
                            schedule.accept(self.user_id, tc).await?;
                            let schedule = ScheduleResponse::from_model(schedule, tc).await?;
                            (
                                ScheduleUpdate::Accepted(schedule),
                                vec![MessageDestination::Global],
                            )
                        }
                        Cancel(id) => {
                            let mut schedule = Schedule::from_id(id, tc).await?;
                            schedule.cancel(self.user_id, tc).await?;
                            let schedule = ScheduleResponse::from_model(schedule, tc).await?;
                            (
                                ScheduleUpdate::Deleted(schedule),
                                vec![MessageDestination::Global],
                            )
                        }
                        Propose(date, game_id) => {
                            let schedule =
                                NewSchedule::new(self.user_id, &game_id, date, tc).await?;
                            let schedule = Schedule::create(schedule, self.user_id, tc).await?;
                            let opponent_id = schedule.opponent_id;
                            let schedule = ScheduleResponse::from_model(schedule, tc).await?;
                            let destinations = vec![
                                MessageDestination::User(self.user_id),
                                MessageDestination::User(opponent_id),
                            ];
                            (ScheduleUpdate::Proposed(schedule), destinations)
                        }
                        TournamentPublic(id) => {
                            let tournament = Tournament::from_nanoid(&id.to_string(), tc).await?;
                            let game_ids =
                                Game::get_ongoing_ids_for_tournament(tournament.id, tc).await?;

                            let mut all_schedules = HashMap::new();
                            for id in game_ids {
                                let game_schedules =
                                    Schedule::all_from_nanoid(id.clone(), tc).await?;
                                let mut game_schedules_map = HashMap::new();
                                for schedule in game_schedules {
                                    let response =
                                        ScheduleResponse::from_model(schedule, tc).await?;
                                    game_schedules_map.insert(response.id, response);
                                }
                                all_schedules.insert(GameId(id), game_schedules_map);
                            }
                            (
                                ScheduleUpdate::OwnTournamentSchedules(all_schedules),
                                vec![MessageDestination::User(self.user_id)],
                            )
                        }
                        TournamentOwn(id) => {
                            let tournament = Tournament::from_nanoid(&id.to_string(), tc).await?;
                            let game_ids = Game::get_ongoing_ids_for_tournament_by_user(
                                tournament.id,
                                self.user_id,
                                tc,
                            )
                            .await?;
                            let mut all_schedules = HashMap::new();
                            for id in game_ids {
                                let game_schedules =
                                    Schedule::all_from_nanoid(id.clone(), tc).await?;
                                let mut game_schedules_map = HashMap::new();
                                for schedule in game_schedules {
                                    let response =
                                        ScheduleResponse::from_model(schedule, tc).await?;
                                    game_schedules_map.insert(response.id, response);
                                }
                                all_schedules.insert(GameId(id), game_schedules_map);
                            }
                            (
                                ScheduleUpdate::OwnTournamentSchedules(all_schedules),
                                vec![MessageDestination::User(self.user_id)],
                            )
                        }
                    })
                }
                .scope_boxed()
            })
            .await?;
        Ok(destinations
            .into_iter()
            .map(|d| InternalServerMessage {
                destination: d.clone(),
                message: ServerMessage::Schedule(update.clone()),
            })
            .collect())
    }
}
