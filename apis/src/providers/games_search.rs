use hive_lib::Color;
use leptos::{provide_context, RwSignal};
use shared_types::{BatchInfo, GameSpeed, ResultType};
use std::{fmt::Display, str::FromStr, vec};

use crate::responses::{GameResponse, UserResponse};

#[derive(Debug, Clone)]
pub struct ProfileControls {
    pub color: Option<Color>,
    pub result: Option<ResultType>,
    pub speeds: Vec<GameSpeed>,
    pub tab_view: ProfileGamesView,
}

#[derive(Debug, Clone)]
pub struct ProfileGamesContext {
    pub games: RwSignal<Vec<GameResponse>>,
    pub more_games: RwSignal<bool>,
    pub batch_info: RwSignal<Option<BatchInfo>>,
    pub user: RwSignal<Option<UserResponse>>,
    pub controls: RwSignal<ProfileControls>,
}

#[derive(Clone, PartialEq, Copy, Debug, Eq, Hash, Default)]
pub enum ProfileGamesView {
    Unstarted,
    #[default]
    Playing,
    Finished,
}
impl FromStr for ProfileGamesView {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unstarted" => Ok(ProfileGamesView::Unstarted),
            "Playing" => Ok(ProfileGamesView::Playing),
            "Finished" => Ok(ProfileGamesView::Finished),
            _ => Err(anyhow::anyhow!("Invalid ProfileGamesView string")),
        }
    }
}
impl Display for ProfileGamesView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view = match self {
            ProfileGamesView::Unstarted => "Unstarted",
            ProfileGamesView::Playing => "Playing",
            ProfileGamesView::Finished => "Finished",
        };
        write!(f, "{view}")
    }
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
            tab_view: ProfileGamesView::Playing,
        }),
    });
}
