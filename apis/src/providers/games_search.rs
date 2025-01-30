use hive_lib::Color;
use leptos::prelude::{provide_context, RwSignal};
use shared_types::{BatchInfo, GameProgress, GameSpeed, ResultType};
use std::vec;

use crate::responses::{GameResponse, UserResponse};

#[derive(Debug, Clone)]
pub struct ProfileControls {
    pub color: Option<Color>,
    pub result: Option<ResultType>,
    pub speeds: Vec<GameSpeed>,
    pub tab_view: GameProgress,
}

#[derive(Debug, Clone)]
pub struct ProfileGamesContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub more_games: RwSignal<bool>,
    pub batch_info: RwSignal<Option<BatchInfo>>,
    pub user: RwSignal<Option<UserResponse>>,
    pub controls: RwSignal<ProfileControls>,
}

pub fn provide_profile_games() {
    provide_context(ProfileGamesContext {
        games: RwSignal::new(Vec::new()),
        batch_info: RwSignal::new(None),
        more_games: RwSignal::new(true),
        user: RwSignal::new(None),
        controls: RwSignal::new(ProfileControls {
            color: None,
            result: None,
            speeds: vec![],
            tab_view: GameProgress::Playing,
        }),
    });
}
