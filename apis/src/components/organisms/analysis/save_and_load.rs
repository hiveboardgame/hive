use crate::{
    components::molecules::modal::Modal,
    i18n::*,
    providers::{
        analysis::{AnalysisContext, LoadError},
        game_state::GameStateStore,
    },
};
use leptos::{html, html::Dialog, logging, prelude::*, task::spawn_local_scoped_with_cancellation};
use leptos_i18n::I18nContext;
use std::{path::Path, sync::Arc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::Array, Blob, Url};

const BUTTON_CLASS: &str = "ui-button ui-button-primary ui-button-sm h-9 flex-1 px-3 text-xs";

#[derive(Clone, Copy)]
enum AnalysisFileType {
    Json,
    Pgn,
}

#[derive(Clone, Debug)]
enum AnalysisFileError {
    UnsupportedExtension(String),
    MissingExtension,
    UnreadableText,
    Read(String),
    Json(Arc<LoadError>),
    Pgn(Arc<LoadError>),
}

impl AnalysisFileError {
    fn localized(&self, i18n: I18nContext<Locale, I18nKeys>) -> String {
        match self {
            Self::UnsupportedExtension(extension) => t_string!(
                i18n,
                analysis.unsupported_file,
                extension = extension.clone()
            )
            .to_string(),
            Self::MissingExtension => {
                t_string!(i18n, analysis.choose_a_json_or_pgn_file).to_string()
            }
            Self::UnreadableText => t_string!(
                i18n,
                analysis.the_selected_file_did_not_contain_readable_text
            )
            .to_string(),
            Self::Read(error) => {
                t_string!(i18n, analysis.file_read_failed, error = error.clone()).to_string()
            }
            Self::Json(error) => t_string!(
                i18n,
                analysis.json_load_failed,
                error = error.localized(i18n.get_locale())
            )
            .to_string(),
            Self::Pgn(error) => t_string!(
                i18n,
                analysis.pgn_load_failed,
                error = error.localized(i18n.get_locale())
            )
            .to_string(),
        }
    }
}

#[component]
pub fn DownloadTree() -> impl IntoView {
    let i18n = use_i18n();
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
            {t!(i18n, common.save)}
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
    let i18n = use_i18n();
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let input_ref = NodeRef::<html::Input>::new();
    let load_owner = Owner::current().expect("LoadTree must run inside a reactive owner");
    let error = RwSignal::new(None::<AnalysisFileError>);
    let request_id = StoredValue::new(0_u64);
    let oninput = move |_| {
        let Some(input) = input_ref.get_untracked() else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        input.set_value("");
        let next_request = request_id.get_value() + 1;
        request_id.set_value(next_request);
        let extension = Path::new(&file.name())
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        let file_type = match extension.as_deref() {
            Some("json") => AnalysisFileType::Json,
            Some("pgn") => AnalysisFileType::Pgn,
            Some(extension) => {
                error.set(Some(AnalysisFileError::UnsupportedExtension(
                    extension.to_string(),
                )));
                return;
            }
            None => {
                error.set(Some(AnalysisFileError::MissingExtension));
                return;
            }
        };
        error.set(None);
        load_owner.with(|| {
            spawn_local_scoped_with_cancellation(async move {
                let text = match JsFuture::from(file.text()).await {
                    Ok(text) => text.as_string().ok_or(AnalysisFileError::UnreadableText),
                    Err(read_error) => Err(AnalysisFileError::Read(format!("{read_error:?}"))),
                };
                if request_id.get_value() != next_request {
                    return;
                }
                let result = text.and_then(|text| match file_type {
                    AnalysisFileType::Json => analysis
                        .store
                        .load_json(game_state, &text)
                        .map_err(|error| AnalysisFileError::Json(Arc::new(error))),
                    AnalysisFileType::Pgn => analysis
                        .store
                        .load_pgn(game_state, &text)
                        .map_err(|error| AnalysisFileError::Pgn(Arc::new(error))),
                });
                match result {
                    Ok(()) => {
                        analysis.sync_reserve_from_game_state(game_state);
                    }
                    Err(message) => {
                        logging::log!("{message:?}");
                        error.set(Some(message));
                    }
                }
            });
        });
    };
    view! {
        <div class="flex flex-col flex-1 gap-1 min-w-0">
            <label for="load-analysis" class=format!("{BUTTON_CLASS} w-full cursor-pointer")>
                {move || t_string!(i18n, analysis.load_file)}
            </label>
            <input
                node_ref=input_ref
                on:input=oninput
                type="file"
                id="load-analysis"
                accept=".json,.pgn"
                hidden
            />
            <ShowLet some=move || error.get().map(|error| error.localized(i18n)) let:message>
                <span class="text-xs ui-field-error">{message}</span>
            </ShowLet>
        </div>
    }
}

/// Fresh analysis from a pasted HOP: a position, not a game, so it roots a new tree.
#[component]
pub fn LoadHop() -> impl IntoView {
    let i18n = use_i18n();
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let dialog_el = NodeRef::<Dialog>::new();
    let input = RwSignal::new(String::new());
    let error = RwSignal::new(None::<Arc<LoadError>>);

    let open = move |_| {
        input.set(String::new());
        error.set(None);
        if let Some(dialog) = dialog_el.get() {
            let _ = dialog.show_modal();
        }
    };

    let load = move |_| {
        // `load_hop` parses before installing, so a bad HOP leaves the current analysis alone.
        match analysis.store.load_hop(game_state, &input.get_untracked()) {
            Ok(()) => {
                error.set(None);
                analysis.sync_reserve_from_game_state(game_state);
                if let Some(dialog) = dialog_el.get() {
                    dialog.close();
                }
            }
            Err(load_error) => error.set(Some(Arc::new(load_error))),
        }
    };

    view! {
        <button on:click=open class=BUTTON_CLASS>
            {move || t_string!(i18n, analysis.load_hop)}
        </button>
        <Modal dialog_el aria_labelledby="load-hop-label">
            <div class="flex flex-col gap-2 p-4 w-80 max-w-full">
                <label id="load-hop-label" class="ui-field-label" for="load-hop">
                    {move || t_string!(i18n, analysis.load_position_from_hop)}
                </label>
                <textarea
                    id="load-hop"
                    rows="3"
                    class="font-mono text-xs ui-field-textarea"
                    placeholder="base,QA-a,w"
                    prop:value=input
                    aria-invalid=move || error.with(Option::is_some).to_string()
                    aria-describedby=move || {
                        error.with(Option::is_some).then_some("load-hop-error")
                    }
                    on:input=move |ev| {
                        input.set(event_target_value(&ev));
                        error.set(None);
                    }
                />
                <Show when=move || error.with(Option::is_some)>
                    <p id="load-hop-error" class="ui-field-error" aria-live="polite">
                        {move || error.get().map(|error| error.localized(i18n.get_locale()))}
                    </p>
                </Show>
                <p class="ui-field-helper">{move || t_string!(i18n, analysis.hop_help)}</p>
                <button on:click=load class=format!("{BUTTON_CLASS} w-full")>
                    {move || t_string!(i18n, common.load)}
                </button>
            </div>
        </Modal>
    }
}
