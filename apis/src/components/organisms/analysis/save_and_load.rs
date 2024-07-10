use base64::{engine::general_purpose, Engine as _};
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
            class="z-20 justify-center items-center m-1 w-1/3 h-7 text-white rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
        >
            "Save"
        </button>
    }
}

fn blob_and_filename(tree: AnalysisTree) -> (Blob, String) {
    let tree = bincode::serialize(&tree).unwrap();
    let tree = general_purpose::STANDARD.encode(tree);
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
    let loaded = RwSignal::new(false);
    let div_ref = NodeRef::<html::Div>::new();
    div_ref.on_load(move |_| {
        let _ = div_ref
            .get_untracked()
            .expect("div to be loaded")
            .on_mount(move |_| loaded.set(true));
    });
    let maybe_update_tree = move |maybe_tree: Option<AnalysisTree>| {
        if let Some(tree) = maybe_tree {
            analysis.update(|a| {
                if let Some(a) = a {
                    a.reset();
                    a.tree = tree.tree;
                    if let Some(current_node) = tree.current_node {
                        a.update_node(current_node.get_node_id());
                    }
                }
            });
            Some(())
        } else {
            None
        }
    };
    view! {
        <div ref=div_ref class="m-1 w-1/3 h-7">
            <Show when=loaded>

                {
                    let from_hat = Closure::new(move |string: JsValue| {
                        let res = string
                            .as_string()
                            .and_then(|string| general_purpose::STANDARD.decode(string).ok())
                            .and_then(|bytes| {
                                maybe_update_tree(bincode::deserialize::<AnalysisTree>(&bytes).ok())
                            });
                        if res.is_none() {
                            logging::log!("Couldn't open file");
                        }
                    });
                    let from_pgn = Closure::new(move |string: JsValue| {
                        let res = string
                            .as_string()
                            .and_then(|string| { History::from_pgn_str(string).ok() })
                            .and_then(|history| hive_lib::State::new_from_history(&history).ok())
                            .and_then(|state| {
                                let mut new_gs = GameState::new();
                                new_gs.state = state;
                                let new_gs_signal = GameStateSignal::new();
                                new_gs_signal.signal.update_untracked(|gs| *gs = new_gs);
                                maybe_update_tree(AnalysisTree::from_state(new_gs_signal))
                            });
                        if res.is_none() {
                            logging::log!("Couldn't open file");
                        }
                    });
                    view! {
                        <label
                            for="load-analysis"
                            class="flex z-20 justify-center items-center w-full h-full text-white break-words rounded-sm transition-transform duration-300 transform aspect-square bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                        >
                            "Load"
                            <input
                                ref=input_ref
                                on:input=move |_| {
                                    let file = input_ref
                                        .get_untracked()
                                        .unwrap()
                                        .files()
                                        .unwrap()
                                        .get(0)
                                        .unwrap();
                                    Path::new(&file.name())
                                        .extension()
                                        .map_or_else(
                                            || logging::log!("Couldn't open file"),
                                            |ext| {
                                                if ext == "hat" {
                                                    let _ = file.text().then(&from_hat);
                                                } else if ext == "pgn" {
                                                    let _ = file.text().then(&from_pgn);
                                                } else {
                                                    logging::log!("Couldn't open file");
                                                }
                                            },
                                        );
                                }

                                type="file"
                                id="load-analysis"
                                class="hidden"
                                accept=".hat,.pgn"
                            />
                        </label>
                    }
                }

            </Show>
        </div>
    }
}
