use crate::common::{auth_page_url, safe_return_path};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use url::form_urlencoded;

#[derive(Clone)]
pub struct RefererContext {
    pub pathname: StoredValue<String>,
}

pub fn provide_referer() {
    provide_context(RefererContext {
        pathname: StoredValue::new(String::new()),
    })
}

pub(crate) fn use_auth_return_path() -> Memo<String> {
    let previous = expect_context::<RefererContext>().pathname;
    let search = use_location().search;
    Memo::new(move |_| {
        let query = search.get();
        let path = form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == "return_to")
            .map(|(_, value)| value.into_owned())
            .unwrap_or_else(|| previous.get_value());
        safe_return_path(&path)
    })
}

pub(crate) fn login_redirect_url() -> String {
    let location = use_location();
    let mut path = location.pathname.get_untracked();
    let search = location.search.get_untracked();
    if !search.is_empty() {
        path.push('?');
        path.push_str(&search);
    }
    path.push_str(&location.hash.get_untracked());
    auth_page_url("/login", &path)
}
