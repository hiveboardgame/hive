use hive_lib::{Color, GameResult, GameStatus};
use leptos::prelude::*;
use leptos_icons::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{js_sys::Array, Blob, Url};

use crate::{providers::game_state::GameStateSignal, responses::GameResponse};

#[component]
pub fn DownloadPgn(
    #[prop(optional, into)] game: Option<StoredValue<GameResponse>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let maybe_game = move || {
        if let Some(game) = game {
            Some(game)
        } else {
            game_state.signal.with(|gs| {
                gs.game_response
                    .as_ref()
                    .map(|v| StoredValue::new(v.clone()))
            })
        }
    };
    let download = move |_| {
        if let Some(game) = maybe_game() {
            let (blob, filename) = blob_and_filename(game);
            // Create an object URL for the blob
            let url = Url::create_object_url_with_blob(&blob).unwrap();
            // Create a download link
            let a = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_element("a")
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .expect("This element is not an HtmlElement");
            a.set_attribute("href", &url).unwrap();
            a.set_attribute("download", &filename).unwrap();
            a.click();
            let _ = Url::revoke_object_url(&url);
        }
    };

    view! {
        <Show when=move || maybe_game().is_some()>
            <button
                class="flex z-20 justify-center items-center m-1 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                on:click=download
            >
                <Icon icon=icondata_ai::AiDownloadOutlined attr:class="py-1 w-7 h-7" />
            </button>
        </Show>
    }
}

fn blob_and_filename(game: StoredValue<GameResponse>) -> (Blob, String) {
    game.with_value(|game| {
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
            game.game_type,
            date,
            game.white_player.username,
            game.black_player.username,
            game_result
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
        (
            Blob::new_with_u8_array_sequence(&file).unwrap(),
            format!(
                "{}_{}_vs_{}.pgn",
                date, game.white_player.username, game.black_player.username
            ),
        )
    })
}
