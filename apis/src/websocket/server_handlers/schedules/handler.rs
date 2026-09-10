use crate::{
    common::ScheduleAction,
    notifications::{notify, Event},
    responses::ScheduleResponse,
    websocket::{
        messages::{HandlerOutput, InternalServerMessage},
        server_handlers::{
            game::tournament_progression::schedule_response_update_messages,
            tournaments::append_public_slot_patches,
        },
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{ScheduleOffer, Tournament, TournamentSlot},
    DbPool,
};
use diesel_async::AsyncConnection;
use shared_types::{ScheduleOfferStatus, TournamentStatus};
use uuid::Uuid;

pub struct ScheduleHandler {
    pool: DbPool,
    user_id: Uuid,
    action: ScheduleAction,
}

impl ScheduleHandler {
    pub fn new(user_id: Uuid, action: ScheduleAction, pool: &DbPool) -> Self {
        Self {
            pool: pool.clone(),
            user_id,
            action,
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let (responses, notification, public_slots) = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| match self.action.clone() {
                ScheduleAction::Propose {
                    candidate_times,
                    tournament_id,
                    slot_id,
                } => {
                    let tournament = Tournament::find_by_tournament_id(&tournament_id, tc).await?;
                    let offers = ScheduleOffer::propose(
                        self.user_id,
                        tournament.id,
                        slot_id,
                        candidate_times,
                        tc,
                    )
                    .await?;
                    let responses = ScheduleResponse::from_models_batch(offers, tc).await?;
                    let pending = responses
                        .iter()
                        .find(|response| response.status == ScheduleOfferStatus::Pending)
                        .ok_or_else(|| anyhow::anyhow!("new schedule offer is not pending"))?;
                    let notification = Event::SchedulePropose {
                        recipient: pending.opponent_id,
                        proposer: pending.proposer_username.clone(),
                        tournament_nanoid: pending.tournament_id.0.clone(),
                        slot_id: pending.slot_id,
                        candidate_times: pending.candidate_times.clone(),
                    };
                    Ok((responses, Some(notification), None))
                }
                ScheduleAction::Accept {
                    offer_id,
                    selected_time,
                } => {
                    let offer =
                        ScheduleOffer::accept(offer_id, self.user_id, selected_time, tc).await?;
                    let tournament_id = offer.tournament_id;
                    let slot_id = offer.tournament_slot_id;
                    let response = ScheduleResponse::from_model(offer, tc).await?;
                    let notification = Event::ScheduleAccept {
                        recipient: response.proposer_id,
                        opponent: response.opponent_username.clone(),
                        tournament_nanoid: response.tournament_id.0.clone(),
                        slot_id: response.slot_id,
                        when: selected_time,
                    };
                    Ok((
                        vec![response],
                        Some(notification),
                        Some((tournament_id, vec![slot_id])),
                    ))
                }
                ScheduleAction::Decline(offer_id) => {
                    let offer = ScheduleOffer::decline(offer_id, self.user_id, tc).await?;
                    let response = ScheduleResponse::from_model(offer, tc).await?;
                    Ok((vec![response], None, None))
                }
                ScheduleAction::Withdraw(offer_id) => {
                    let offer = ScheduleOffer::withdraw(offer_id, self.user_id, tc).await?;
                    let response = ScheduleResponse::from_model(offer, tc).await?;
                    Ok((vec![response], None, None))
                }
                ScheduleAction::SetDeadline {
                    tournament_id,
                    slot_ids,
                    deadline_at,
                } => {
                    let tournament = Tournament::find_by_tournament_id(&tournament_id, tc).await?;
                    tournament
                        .ensure_user_is_organizer_or_admin(&self.user_id, tc)
                        .await?;
                    if tournament.status() != TournamentStatus::InProgress {
                        return Err(anyhow::anyhow!(
                            "deadlines can only be changed while a tournament is in progress"
                        ));
                    }
                    TournamentSlot::set_deadline_for_slots(
                        tournament.id,
                        &slot_ids,
                        deadline_at,
                        tc,
                    )
                    .await?;
                    Ok((Vec::new(), None, Some((tournament.id, slot_ids))))
                }
            })
            .await?;

        if let Some(notification) = notification {
            notify(notification);
        }
        let mut output = HandlerOutput::from(schedule_response_update_messages(responses));
        if let Some((tournament_id, slot_ids)) = public_slots {
            append_public_slot_patches(tournament_id, &slot_ids, &mut output, &mut conn).await;
        }
        Ok(output.messages)
    }
}
