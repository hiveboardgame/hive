use crate::responses::ScheduleResponse;
use leptos::prelude::{provide_context, RwSignal, StoredValue, UpdateValue, WithValue};
use shared_types::GameId;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

type ScheduleMap = HashMap<GameId, HashMap<Uuid, ScheduleResponse>>;

#[derive(Clone, Debug, Copy)]
pub struct SchedulesContext {
    pub own: RwSignal<ScheduleMap>,
    pub tournament: RwSignal<ScheduleMap>,
    own_schedule_resync_dirty: StoredValue<HashSet<Uuid>>,
    own_game_resync_dirty: StoredValue<HashSet<GameId>>,
}

impl SchedulesContext {
    pub fn new() -> Self {
        Self {
            own: RwSignal::new(HashMap::new()),
            tournament: RwSignal::new(HashMap::new()),
            own_schedule_resync_dirty: StoredValue::new(HashSet::new()),
            own_game_resync_dirty: StoredValue::new(HashSet::new()),
        }
    }

    pub fn begin_resync(&self) {
        self.clear_own_resync_dirty();
    }

    pub fn mark_own_schedule_dirty(&self, schedule_id: Uuid) {
        self.own_schedule_resync_dirty.update_value(|dirty| {
            dirty.insert(schedule_id);
        });
    }

    pub fn mark_own_game_dirty(&self, game_id: &GameId) {
        self.own_game_resync_dirty.update_value(|dirty| {
            dirty.insert(game_id.clone());
        });
    }

    pub fn is_own_schedule_resync_dirty(&self, schedule_id: Uuid) -> bool {
        self.own_schedule_resync_dirty
            .with_value(|dirty| dirty.contains(&schedule_id))
    }

    pub fn is_own_acceptance_snapshot_dirty(&self, schedule: &ScheduleResponse) -> bool {
        self.is_own_schedule_resync_dirty(schedule.id)
            || self
                .own_game_resync_dirty
                .with_value(|dirty| dirty.contains(&schedule.game_id))
    }

    pub fn clear_own_resync_dirty(&self) {
        self.own_schedule_resync_dirty
            .update_value(|dirty| dirty.clear());
        self.own_game_resync_dirty
            .update_value(|dirty| dirty.clear());
    }
}

impl Default for SchedulesContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_schedules() {
    provide_context(SchedulesContext::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use leptos::prelude::Owner;
    use shared_types::TournamentId;

    fn schedule(schedule_id: Uuid, game_id: &str) -> ScheduleResponse {
        ScheduleResponse {
            id: schedule_id,
            tournament_name: "Tournament".to_string(),
            tournament_id: TournamentId("tournament".to_string()),
            proposer_id: Uuid::new_v4(),
            proposer_username: "proposer".to_string(),
            opponent_id: Uuid::new_v4(),
            opponent_username: "opponent".to_string(),
            game_id: GameId(game_id.to_string()),
            start_t: Utc::now(),
            agreed: false,
            notified: false,
        }
    }

    #[test]
    fn dirty_schedule_id_blocks_snapshot_write_until_cleared() {
        let owner = Owner::new();
        owner.with(|| {
            let ctx = SchedulesContext::new();
            let schedule_id = Uuid::new_v4();
            let response = schedule(schedule_id, "game-a");

            ctx.begin_resync();
            ctx.mark_own_schedule_dirty(schedule_id);

            assert!(ctx.is_own_schedule_resync_dirty(response.id));

            ctx.clear_own_resync_dirty();

            assert!(!ctx.is_own_schedule_resync_dirty(response.id));
        });
    }

    #[test]
    fn dirty_game_id_blocks_acceptance_snapshot_writes_for_that_game() {
        let owner = Owner::new();
        owner.with(|| {
            let ctx = SchedulesContext::new();
            let dirty_game = GameId("game-a".to_string());
            let same_game_other_schedule = schedule(Uuid::new_v4(), "game-a");
            let other_game = schedule(Uuid::new_v4(), "game-b");

            ctx.begin_resync();
            ctx.mark_own_game_dirty(&dirty_game);

            assert!(ctx.is_own_acceptance_snapshot_dirty(&same_game_other_schedule));
            assert!(!ctx.is_own_acceptance_snapshot_dirty(&other_game));
        });
    }
}
