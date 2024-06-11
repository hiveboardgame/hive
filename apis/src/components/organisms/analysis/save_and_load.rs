use core::str;
use hive_lib::History;
use leptos::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{js_sys::Array, Blob, Url};

use super::AnalysisTree;
use crate::{
    components::organisms::analysis::AnalysisSignal,
    providers::game_state::{GameState, GameStateSignal},
};
use std::path::Path;
use wasm_bindgen::closure::Closure;
#[component]
pub fn DownloadTree(tree: AnalysisTree) -> impl IntoView {
    let download = move |_| {
        let (blob, filename) = blob_and_filename(tree.clone());
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
    };

    view! {
        <button
            on:click=download
            class="z-20 justify-center items-center m-1 w-full h-7 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
        >
            "Save Analysis"
        </button>
    }
}

fn blob_and_filename(tree: AnalysisTree) -> (Blob, String) {
    let tree = bincode::serialize(&tree).unwrap();
    let tree = String::from_utf8(tree).unwrap();
    let file = Array::from(&JsValue::from(tree));
    let date = chrono::offset::Local::now()
        .format("%d-%b-%Y_%H:%M:%S")
        .to_string();
    (
        Blob::new_with_u8_array_sequence(&file).unwrap(),
        format!("analysis_{date}.hat"),
    )
}

#[component]
pub fn LoadTree() -> impl IntoView {
    let input_ref = create_node_ref::<html::Input>();
    let analysis = expect_context::<AnalysisSignal>().0;
    let from_hat = Closure::new(move |string: JsValue| {
        let bytes = string.as_string().unwrap().as_bytes().to_vec();
        if let Ok(tree) = bincode::deserialize::<AnalysisTree>(&bytes) {
            analysis.update(|a| {
                if let Some(a) = a {
                    a.reset();
                    a.tree = tree.tree;
                    if let Some(current_node) = tree.current_node {
                        a.update_node(current_node.get_node_id());
                    }
                }
            });
        } else {
            logging::log!("Couldn't open analysis file");
        }
    });
    let from_pgn = Closure::new(move |string: JsValue| {
        let string = string.as_string().unwrap();
        let history = History::from_pgn_str(string);
        if let Ok(history) = history {
            let state =
                hive_lib::State::new_from_history(&history).expect("Couldn't create game state");
            let mut new_gs = GameState::new();
            new_gs.state = state;
            let new_gs_signal = GameStateSignal::new();
            new_gs_signal.signal.update_untracked(|gs| *gs = new_gs);
            if let Some(tree) = AnalysisTree::from_state(new_gs_signal) {
                analysis.update(|a| {
                    if let Some(a) = a {
                        a.reset();
                        a.tree = tree.tree;
                        if let Some(current_node) = tree.current_node {
                            a.update_node(current_node.get_node_id());
                        }
                    }
                });
            } else {
                logging::log!("Couldn't open pgn file");
            }
        }
    });
    view! {
        <label
            for="load-analysis"
            class="flex z-20 justify-center items-center m-1 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
        >
            "Load Analysis or Game"
        </label>
        <input
            ref=input_ref
            on:input=move |_| {
                let file = input_ref.get_untracked().unwrap().files().unwrap().get(0).unwrap();
                let filename = file.name();
                let ext = Path::new(&filename).extension().unwrap();
                ext.to_str()
                    .map_or_else(
                        || logging::log!("Couldn't open file"),
                        |ext| {
                            match ext {
                                "hat" => {
                                    let _ = file.text().then(&from_hat);
                                }
                                "pgn" => {
                                    let _ = file.text().then(&from_pgn);
                                }
                                _ => logging::log!("Couldn't open file"),
                            }
                        },
                    );
            }

            type="file"
            id="load-analysis"
            class="hidden"
            accept=".hat,.pgn"
        />
    }
}
