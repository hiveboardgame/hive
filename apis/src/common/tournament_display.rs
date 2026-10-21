use crate::{
    i18n::*,
    providers::{RoundRobinStateStoreFields, SwissStateStoreFields, TournamentFormatState},
    responses::TournamentMemberships,
};
use chrono::{DateTime, Utc};
use hive_lib::GameStatus;
use leptos::prelude::*;
use shared_types::{
    tournament::{elimination::EntrantSide, swiss::System as SwissSystem, ReleasePolicy, SlotKey},
    tournament_view::{ArenaPlayerStatsResponse, SlotResponse},
};
use uuid::Uuid;

#[cfg(test)]
mod tests;

pub(crate) fn arena_player_rating(stats: &ArenaPlayerStatsResponse) -> Option<u64> {
    stats
        .arena_rating
        .and_then(|rating| u64::try_from(rating).ok())
}

pub(crate) fn elimination_color_sequence(order: &[EntrantSide], repeats: bool) -> String {
    let i18n = use_i18n();
    let sequence = order
        .iter()
        .map(|side| match side {
            EntrantSide::First => {
                t_string!(i18n, tournaments.creation.elimination.colors.white_short).to_string()
            }
            EntrantSide::Second => {
                t_string!(i18n, tournaments.creation.elimination.colors.black_short).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let colors = t_string!(
        i18n,
        tournaments.creation.elimination.colors.top_sequence,
        sequence = sequence
    )
    .to_string();
    if repeats {
        let repeat = t_string!(i18n, tournaments.creation.elimination.colors.reverse_repeat);
        format!("{colors} · {repeat}")
    } else {
        colors
    }
}

pub fn tournament_slot_fragment(slot_id: Uuid) -> String {
    format!("tournament-slot-{slot_id}")
}

pub fn parse_tournament_slot_fragment(fragment: &str) -> Option<Uuid> {
    fragment
        .strip_prefix('#')
        .unwrap_or(fragment)
        .strip_prefix("tournament-slot-")?
        .parse()
        .ok()
}

pub(crate) fn tournament_player_name(memberships: &TournamentMemberships, player: Uuid) -> String {
    memberships
        .players
        .get(&player)
        .map(|user| user.username.clone())
        .unwrap_or_else(|| String::from("—"))
}

pub(crate) fn tournament_slot_opponent(slot: &SlotResponse, player: Uuid) -> Option<Uuid> {
    let side = slot
        .participants
        .iter()
        .position(|participant| *participant == player)?;
    Some(slot.participants[1 - side])
}

pub(crate) fn tournament_game_number(
    format: TournamentFormatState,
    slot: &SlotResponse,
    scheduled_number: usize,
) -> Option<usize> {
    match format {
        TournamentFormatState::RoundRobin(state)
            if state.configuration().get().release_policy == ReleasePolicy::FullyUnlocked =>
        {
            state
                .slots()
                .with(|slots| completion_game_number(slot, slots.values()))
        }
        TournamentFormatState::Swiss(state)
            if state.configuration().with(|config| {
                matches!(config.system, SwissSystem::DoubleSwiss(_))
                    && config.double_swiss_release_policy == ReleasePolicy::FullyUnlocked
            }) =>
        {
            state
                .slots()
                .with(|slots| completion_game_number(slot, slots.values()))
        }
        _ => Some(scheduled_number),
    }
}

fn completion_game_number<'a>(
    slot: &SlotResponse,
    slots: impl Iterator<Item = &'a SlotResponse>,
) -> Option<usize> {
    let completion = game_completion(slot)?;
    Some(
        1 + slots
            .filter(|other| {
                let same_encounter = match (slot.key, other.key) {
                    (SlotKey::RoundRobin { .. }, SlotKey::RoundRobin { .. }) => {
                        slot.participants == other.participants
                            || slot.participants == [other.participants[1], other.participants[0]]
                    }
                    (SlotKey::Swiss { slot: left }, SlotKey::Swiss { slot: right }) => {
                        left.round_index == right.round_index
                            && left.pairing_index == right.pairing_index
                    }
                    _ => false,
                };
                same_encounter && game_completion(other).is_some_and(|other| other < completion)
            })
            .count(),
    )
}

fn game_completion(slot: &SlotResponse) -> Option<(DateTime<Utc>, Uuid)> {
    // A game can finish before its tournament result arrives. Adjudications can
    // resolve a slot without a played game, so use their resolution time instead.
    slot.game
        .as_ref()
        .and_then(|game| game.finished_at)
        .or_else(|| slot.outcome().and(slot.resolved_at))
        .map(|finished_at| (finished_at, slot.id))
}

pub(crate) fn tournament_completion_order(
    slot: &SlotResponse,
) -> (bool, Option<(DateTime<Utc>, Uuid)>) {
    let completion = game_completion(slot);
    (completion.is_none(), completion)
}

pub(crate) fn unfinished_tournament_game_label(locale: Locale, slot: &SlotResponse) -> String {
    if slot.game.as_ref().is_some_and(|game| game.finished) {
        td_string!(locale, common.finished).to_string()
    } else if slot
        .game
        .as_ref()
        .is_some_and(|game| game.status == GameStatus::InProgress)
    {
        td_string!(locale, common.playing).to_string()
    } else if slot.scheduled_at.is_some() {
        td_string!(locale, common.scheduled).to_string()
    } else if slot.resolution.is_some() {
        td_string!(locale, common.not_played).to_string()
    } else {
        td_string!(locale, common.not_started).to_string()
    }
}

pub(crate) fn rounded_percentage(numerator: u32, denominator: u32) -> Option<u64> {
    (denominator != 0)
        .then(|| (u64::from(numerator) * 100 + u64::from(denominator) / 2) / u64::from(denominator))
}

pub(crate) fn result_marker_color(marker: &str) -> &'static str {
    match marker {
        "W" => "text-grasshopper-green",
        "D" => "text-pillbug-teal",
        "L" => "text-ladybug-red",
        _ => "text-gray-500 dark:text-gray-400",
    }
}
