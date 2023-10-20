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
            ..Default::default()
        },
    )
}

