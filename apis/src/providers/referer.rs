use leptos::prelude::{provide_context, StoredValue};

#[derive(Clone)]
pub struct RefererContext {
    pub pathname: StoredValue<String>,
}

pub fn provide_referer() {
    provide_context(RefererContext {
        pathname: StoredValue::new(String::new()),
    })
}
