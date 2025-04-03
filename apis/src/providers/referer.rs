use leptos::prelude::{provide_context, RwSignal};

#[derive(Clone)]
pub struct RefererContext {
    pub pathname: RwSignal<String>,
}

pub fn provide_referer() {
    provide_context(RefererContext {
        pathname: RwSignal::new(String::new()),
    })
}
