use leptos::leptos_dom::helpers::set_timeout_with_handle;
use std::time::Duration;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, HtmlAnchorElement, Url};

pub(crate) fn download_blob(blob: &Blob, filename: &str) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("Downloads require a browser document"))?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("Downloads require a document body"))?;
    let anchor = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    let url = Url::create_object_url_with_blob(blob)?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    if let Err(error) = body.append_child(&anchor) {
        let _ = Url::revoke_object_url(&url);
        return Err(error);
    }
    // Allow time for a download prompt. Schedule cleanup before clicking so a
    // timer failure cannot cancel a download that has already started. The
    // callback owns only the URL and remains safe after component disposal.
    let revoke_url = url.clone();
    let cleanup = set_timeout_with_handle(
        move || {
            let _ = Url::revoke_object_url(&revoke_url);
        },
        Duration::from_secs(40),
    );
    if let Err(error) = cleanup {
        anchor.remove();
        let _ = Url::revoke_object_url(&url);
        return Err(error);
    }
    anchor.click();
    anchor.remove();
    Ok(())
}
