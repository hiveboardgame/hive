use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};

const COPY_FEEDBACK_MS: u64 = 2000;

pub(crate) struct ClipboardCopy<F>
where
    F: Fn(String) + Copy + Send + Sync + 'static,
{
    pub copied: RwSignal<bool>,
    pub copy_text: F,
}

pub(crate) fn use_clipboard_copy() -> ClipboardCopy<impl Fn(String) + Copy + Send + Sync + 'static>
{
    let copied = RwSignal::new(false);
    let reset_interval = StoredValue::new(use_interval_fn_with_options(
        move || copied.set(false),
        COPY_FEEDBACK_MS,
        UseIntervalFnOptions::default().immediate(false),
    ));

    let copy_text = move |text: String| {
        if let Some(window) = use_window().as_ref() {
            let _ = window.navigator().clipboard().write_text(&text);
            copied.set(true);
            let interval = reset_interval.get_value();
            (interval.pause)();
            (interval.resume)();
        }
    };

    ClipboardCopy { copied, copy_text }
}
