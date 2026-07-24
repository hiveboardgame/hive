use crate::providers::{analysis::AnalysisContext, game_state::GameStateStore};
use leptos::{html, logging, prelude::*, task::spawn_local_scoped_with_cancellation};
use std::path::Path;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::Array, Blob, Url};

const BUTTON_CLASS: &str = "ui-button ui-button-primary ui-button-sm h-9 flex-1 px-3 text-xs";

#[component]
pub fn DownloadTree() -> impl IntoView {
    let analysis = expect_context::<AnalysisContext>().store;

    let download = move |_| {
        let Ok(tree_json) = analysis.to_json() else {
            logging::log!("Couldn't serialize analysis");
            return;
        };

        let (blob, filename) = blob_and_filename(tree_json);
        let url = Url::create_object_url_with_blob(&blob).unwrap();
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
        <button on:click=download class=BUTTON_CLASS>
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
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let input_ref = NodeRef::<html::Input>::new();
    let load_owner = Owner::current().expect("LoadTree must run inside a reactive owner");
    let error = RwSignal::new(None::<String>);
    let request_id = RwSignal::new(0_u64);
    let oninput = move |_| {
        let Some(input) = input_ref.get_untracked() else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        input.set_value("");
        let next_request = request_id.get_untracked() + 1;
        request_id.set(next_request);
        let extension = Path::new(&file.name())
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        let Some(extension) = extension else {
            error.set(Some("Choose a .json or .pgn file.".to_string()));
            return;
        };
        if extension != "json" && extension != "pgn" {
            error.set(Some(format!(
                "Unsupported .{extension} file. Choose JSON or PGN."
            )));
            return;
        }
        error.set(None);
        load_owner.with(|| {
            spawn_local_scoped_with_cancellation(async move {
                let text = match JsFuture::from(file.text()).await {
                    Ok(text) => text.as_string().ok_or_else(|| {
                        "The selected file did not contain readable text.".to_string()
                    }),
                    Err(read_error) => {
                        Err(format!("Could not read the selected file: {read_error:?}"))
                    }
                };
                if request_id.get_untracked() != next_request {
                    return;
                }
                let result = text.and_then(|text| {
                    if extension == "json" {
                        analysis
                            .store
                            .load_json(game_state, &text)
                            .map_err(|error| format!("Could not open analysis JSON: {error}"))
                    } else {
                        analysis
                            .store
                            .load_pgn(game_state, &text)
                            .map_err(|error| format!("Could not open PGN: {error}"))
                    }
                });
                match result {
                    Ok(()) => {
                        analysis.sync_reserve_from_game_state(game_state);
                    }
                    Err(message) => {
                        logging::log!("{message}");
                        error.set(Some(message));
                    }
                }
            });
        });
    };
    view! {
        <div class="flex flex-col flex-1 gap-1 min-w-0">
            <label for="load-analysis" class=format!("{BUTTON_CLASS} w-full cursor-pointer")>
                "Load"
            </label>
            <input
                node_ref=input_ref
                on:input=oninput
                type="file"
                id="load-analysis"
                accept=".json,.pgn"
                hidden
            />
            <ShowLet some=move || error.get() let:message>
                <span class="text-xs ui-field-error">{message}</span>
            </ShowLet>
        </div>
    }
}
