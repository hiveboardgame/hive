use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};
use wasm_bindgen_futures::JsFuture;

const COPY_FEEDBACK_MS: u64 = 2000;

pub(crate) struct ClipboardCopy<F>
where
    F: Fn(String) + Copy + Send + Sync + 'static,
{
    pub copied: RwSignal<bool>,
    pub copy_text: F,
}

/// `copied` flips only once the clipboard promise resolves - it can refuse (permission,
/// insecure context), and confirming early would report a copy that never happened.
pub(crate) fn use_clipboard_copy() -> ClipboardCopy<impl Fn(String) + Copy + Send + Sync + 'static>
{
    let copied = RwSignal::new(false);
    let reset_interval = StoredValue::new(use_interval_fn_with_options(
        move || copied.set(false),
        COPY_FEEDBACK_MS,
        UseIntervalFnOptions::default().immediate(false),
    ));

    let copy_text = move |text: String| {
        // On the server there is no window and nothing to copy; the whole call is a no-op.
        let Some(promise) = use_window()
            .as_ref()
            .map(|window| window.navigator().clipboard().write_text(&text))
        else {
            return;
        };
        leptos::task::spawn_local(async move {
            if JsFuture::from(promise).await.is_ok() {
                // The owner can be disposed while the promise is pending (navigate away mid-copy);
                // a panic here takes down the whole wasm app, so skip the feedback instead.
                if copied.try_set(true).is_some() {
                    return;
                }
                let Some(interval) = reset_interval.try_get_value() else {
                    return;
                };
                (interval.pause)();
                (interval.resume)();
            }
        });
    };

    ClipboardCopy { copied, copy_text }
}
