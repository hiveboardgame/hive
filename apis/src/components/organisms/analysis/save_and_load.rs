use core::str;
use cookie::time::ext;
use hive_lib::History;
use leptos::{html, logging, prelude::*};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{js_sys::Array, Blob, Url};

use super::AnalysisTree;
use crate::{
    components::organisms::analysis::AnalysisSignal, i18n::namespaces::ns_game::game, providers::game_state::GameStateSignal
};
use std::path::Path;
use wasm_bindgen::closure::Closure;
#[component]
pub fn DownloadTree(tree: String) -> impl IntoView {
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
            class="z-20 justify-center items-center m-1 w-1/3 h-7 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
        >
            "Save"
        </button>
    }
}

fn blob_and_filename(tree: String) -> (Blob, String) {
    let file = Array::from(&JsValue::from(tree));
    let date = chrono::offset::Local::now()
        .format("%d-%b-%Y_%H:%M:%S")
        .to_string();
    (
        Blob::new_with_u8_array_sequence(&file).unwrap(),
        format!("analysis_{date}.json"),
    )
}

#[component]
pub fn LoadTree() -> impl IntoView {
    let analysis = expect_context::<AnalysisSignal>().0;
    let game_state = expect_context::<GameStateSignal>();
    let input_ref = NodeRef::<html::Input>::new();

    let from_pgn = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| { History::from_pgn_str(string).ok() })
            .and_then(|history| hive_lib::State::new_from_history(&history).ok())
            .and_then(|state| {
                game_state.signal.update(|gs| gs.state = state);
                AnalysisTree::from_state(game_state)
            })
            .map(|tree| {
                analysis.set(Some(LocalStorage::wrap(tree)));
            })
    };
    let from_hat = move |string: JsValue| {
        string
            .as_string()
            .and_then(|string| serde_json::from_str::<AnalysisTree>(&string).ok())
            .map(|tree| {
                analysis.set(Some(LocalStorage::wrap(tree.clone())));
                if let Some(node) = tree.current_node {
                    analysis.get().unwrap().update_node(node.get_node_id());
                }
            })
    };
    let oninput = move |_| {
        let file = input_ref
            .get_untracked()
            .unwrap()
            .files()
            .unwrap()
            .get(0)
            .unwrap();
        Path::new(&file.name()).extension()
            .map_or_else(
                || logging::log!("Couldn't open file"),
                |ext| {
                    if ext == "json" {
                        spawn_local(async move {
                            let res = JsFuture::from(file.text())
                            .await
                            .ok()
                            .and_then(from_hat);
                            if res.is_none() {
                                logging::log!("Couldn't open file");
                            }
                        });
                    } else if ext == "pgn" {
                        spawn_local(async move {
                            let res = JsFuture::from(file.text())
                            .await
                            .ok()
                            .and_then(from_pgn);
                            if res.is_none() {
                                logging::log!("Couldn't open file");
                            }
                        });
                    } else {
                        logging::log!("Unsupported file type");
                    }
                    }
            );
    };
    view! {
        <form>
            <label class="flex z-20 justify-center items-center w-full h-full text-white break-words rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95">
                "Load"
            <input
                node_ref=input_ref
                on:input=oninput
                type="file"
                id="load-analysis"
                class="hidden"
                accept=".json,.pgn"
            />
            </label>
        </form>
    }
}
