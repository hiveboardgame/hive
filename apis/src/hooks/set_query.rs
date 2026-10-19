use leptos::prelude::*;
use leptos_router::{
    hooks::{use_location, use_navigate, use_query_map},
    NavigateOptions,
};

/// Sets (or, given None, removes) query parameters of the page in place, without a history
/// entry or a scroll: the URL follows what is on screen, so a link or a reload brings it back.
pub fn use_set_query() -> Callback<Vec<(&'static str, Option<String>)>> {
    let navigate = use_navigate();
    let location = use_location();
    let queries = use_query_map();
    Callback::new(move |changes: Vec<(&'static str, Option<String>)>| {
        let mut query = queries.get_untracked();
        let before = query.to_query_string();
        for (key, value) in changes {
            match value {
                Some(value) => query.replace(key, value),
                None => {
                    query.remove(key);
                }
            }
        }
        if query.to_query_string() == before {
            return;
        }
        navigate(
            &format!(
                "{}{}",
                location.pathname.get_untracked(),
                query.to_query_string()
            ),
            NavigateOptions {
                replace: true,
                scroll: false,
                ..Default::default()
            },
        );
    })
}
