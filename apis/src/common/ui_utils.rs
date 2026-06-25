use leptos::prelude::{IntoView, TextProp};

pub fn with_class(base: &str, extra: impl AsRef<str>) -> String {
    let extra = extra.as_ref().trim();
    if extra.is_empty() {
        base.to_string()
    } else {
        format!("{base} {extra}")
    }
}

pub fn render_text_prop(text: TextProp) -> impl IntoView {
    move || text.get()
}
