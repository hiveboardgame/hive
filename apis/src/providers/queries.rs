use crate::functions::challenges::{get_public::get_public_challenges,challenge_response::ChallengeResponse};
use leptos::*;
use leptos_query::*;
use std::time::Duration;

pub fn use_challenge_query() -> QueryResult<Result<Vec<ChallengeResponse>, ServerFnError>, impl RefetchFn> {
    leptos_query::use_query(
        || (),
        |_| async move { get_public_challenges().await },
        QueryOptions {
            default_value: None,
            refetch_interval: Some(Duration::from_secs(15)),
            resource_option: ResourceOption::NonBlocking,
            stale_time: Some(Duration::from_secs(30)),
            cache_time: Some(Duration::from_secs(60)),
        },
    )
}


