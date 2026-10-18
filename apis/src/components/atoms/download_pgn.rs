use hive_lib::{Color, GameResult, GameStatus};
use leptos::{logging, prelude::*};
use leptos_icons::*;
use shared_types::DELETED_USER_DISPLAY_NAME;
use wasm_bindgen::JsValue;
use web_sys::{js_sys::Array, Blob};

use crate::{
    common::download_blob,
    providers::game_state::{GameStateStore, GameStateStoreFields},
    responses::{GameResponse, UserResponse},
};

#[component]
pub fn DownloadPgn(
    #[prop(optional, into)] game: Option<StoredValue<GameResponse>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let has_game = move || {
        if game.is_some() {
            true
        } else {
            game_response.with(|game| game.is_some())
        }
    };
    let download = move |_| {
        let file = if let Some(game) = game {
            Some(game.with_value(blob_and_filename))
        } else {
            game_response.with_untracked(|game| game.as_ref().map(blob_and_filename))
        };
        if let Some(file) = file {
            if let Err(error) = file.and_then(|(blob, filename)| download_blob(&blob, &filename)) {
                logging::error!("Couldn't download PGN: {error:?}");
            }
        }
    };

    view! {
        <Show when=has_game>
            <button class="z-20 m-1 ui-button ui-button-primary ui-button-icon" on:click=download>
                <Icon icon=icondata_ai::AiDownloadOutlined attr:class="py-1 size-7" />
            </button>
        </Show>
    }
}

// PGN identities stay stable across UI languages.
fn pgn_username(user: &UserResponse) -> &str {
    if user.deleted {
        DELETED_USER_DISPLAY_NAME
    } else {
        &user.username
    }
}

fn blob_and_filename(game: &GameResponse) -> Result<(Blob, String), JsValue> {
    let white = pgn_username(&game.white_player);
    let black = pgn_username(&game.black_player);
    let date = game.created_at.format("%d-%b-%Y_%H:%M:%S").to_string();
    let game_result = match &game.game_status {
        GameStatus::Finished(result) => match result {
            GameResult::Draw => "Draw".to_owned(),
            GameResult::Unknown => "Unkown".to_owned(),
            GameResult::Winner(Color::White) => "WhiteWins".to_owned(),
            GameResult::Winner(Color::Black) => "BlackWins".to_owned(),
        },
        _ => game.game_status.to_string(),
    };
    let mut file: Vec<String> = Vec::new();
    let header = format!(
        "[GameType \"{}\"]\n\
         [Date \"{}\"]\n\
         [Site \"hivegame.com\"]\n\
         [White \"{}\"]\n\
         [Black \"{}\"]\n\
         [Result \"{}\"]\n\n",
        game.game_type, date, white, black, game_result
    );
    file.push(header);
    let mut history = game
        .history
        .iter()
        .enumerate()
        .map(|(i, (mv, dest))| format!("{}. {} {}\n", i + 1, mv, dest))
        .collect::<Vec<String>>();
    file.append(&mut history);
    if game.finished {
        file.push(format!("\n{game_result}\n"));
    }
    let file = file.into_iter().map(JsValue::from).collect::<Array>();
    let date = game.created_at.format("%+").to_string();
    Ok((
        Blob::new_with_u8_array_sequence(&file)?,
        format!(
            "{}_{}_vs_{}.pgn",
            date,
            white.replace(' ', "_"),
            black.replace(' ', "_")
        ),
    ))
}
