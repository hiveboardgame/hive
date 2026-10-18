use crate::common::analysis_url;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

pub(crate) fn use_analysis_href() -> Memo<String> {
    let params = use_params_map();
    let queries = use_query_map();
    Memo::new(move |_| {
        analysis_url(
            params.get().get("nanoid").as_deref(),
            queries.get().get("move").as_deref(),
        )
    })
}
