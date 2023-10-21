use crate::functions::challenges::{
    challenge_response::ChallengeResponse, get_public::get_public_challenges,
};
use leptos::*;
use leptos_query::*;
use std::time::Duration;

pub fn use_challenge_query(
) -> QueryResult<Result<Vec<ChallengeResponse>, ServerFnError>, impl RefetchFn> {
    use_query(
        || (),
        move |_| get_public_challenges(),
        QueryOptions {
            default_value: None,
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
            stale_time: Some(Duration::from_secs(30)),
            cache_time: Some(Duration::from_secs(300)),
        },
    )
}

